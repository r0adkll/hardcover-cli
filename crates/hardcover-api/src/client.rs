use crate::error::{Error, Result};
use crate::model::{Book, BookIdentifier, Contributor, ResolvedBook, ResolvedBy, SearchHit, SearchResults, SearchType, SeriesMembership, User};
use crate::queries::{book_by_id, book_id_by_isbn, book_id_by_slug, me, search, BookById, BookIdByIsbn, BookIdBySlug, Me, Search};
use reqwest::StatusCode;
use graphql_client::{GraphQLQuery, Response};

pub const DEFAULT_BASE_URL: &str = "https://api.hardcover.app";
const USER_AGENT: &str = concat!(
    "hardcover-cli/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/r0adkll/hardcover-cli)"
);

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

pub struct ClientBuilder {
    base_url: String,
    token: String,
}

impl Client {
    pub fn builder(token: impl Into<String>) -> ClientBuilder {
        ClientBuilder { base_url: DEFAULT_BASE_URL.to_string(), token: token.into() }
    }

    async fn execute<Q: GraphQLQuery>(&self, variables: Q::Variables) -> Result<Q::ResponseData> {
        let body = Q::build_query(variables);
        let resp = self
            .http
            .post(format!("{}/v1/graphql", self.base_url))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?;
        match resp.status() {
            StatusCode::UNAUTHORIZED => return Err(Error::InvalidToken),
            StatusCode::FORBIDDEN => {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                return Err(match body.get("error").and_then(|e| e.as_str()) {
                    Some("insufficient_scope") => Error::InsufficientScope(
                        body.get("scope").and_then(|s| s.as_str()).unwrap_or("unknown").to_string(),
                    ),
                    _ => Error::Upstream(body.to_string()),
                });
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after_secs = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok());
                return Err(Error::RateLimited { retry_after_secs });
            }
            _ => {}
        }
        let parsed: Response<Q::ResponseData> = resp.json().await?;
        if let Some(errors) = parsed.errors.filter(|e| !e.is_empty()) {
            return Err(Error::Upstream(
                errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; "),
            ));
        }
        parsed.data.ok_or_else(|| Error::Upstream("response had no data".into()))
    }

    /// The authenticated user. Also serves as token verification.
    pub async fn me(&self) -> Result<User> {
        let data = self.execute::<Me>(me::Variables {}).await?;
        let u = data.me.into_iter().next().ok_or(Error::InvalidToken)?;
        Ok(User { id: u.id, username: u.username.unwrap_or_default(), name: u.name })
    }

    /// Full-text search over one entity type. Pages are 1-based.
    pub async fn search(&self, query: &str, query_type: SearchType, page: i64, per_page: i64) -> Result<SearchResults> {
        let data = self
            .execute::<Search>(search::Variables {
                query: query.to_string(),
                query_type: query_type.as_str().to_string(),
                page,
                per_page,
            })
            .await?;
        let out = data.search.ok_or_else(|| Error::Upstream("search returned no payload".into()))?;
        if let Some(err) = out.error {
            return Err(Error::Upstream(err));
        }
        let results = out.results.unwrap_or_default();
        let hits: Vec<SearchHit> = results
            .get("hits")
            .and_then(|h| h.as_array())
            .map(|hits| hits.iter().filter_map(|h| hit_from_document(h.get("document")?)).collect())
            .unwrap_or_default();
        Ok(SearchResults {
            query: out.query.unwrap_or_else(|| query.to_string()),
            query_type,
            page: out.page.unwrap_or(page),
            per_page: out.per_page.unwrap_or(per_page),
            found: results.get("found").and_then(|f| f.as_i64()).unwrap_or(hits.len() as i64),
            hits,
        })
    }

    /// Resolve any Identifier form to a numeric book id.
    pub async fn resolve_book(&self, ident: &BookIdentifier) -> Result<ResolvedBook> {
        match ident {
            BookIdentifier::Id(id) => Ok(ResolvedBook { id: *id, resolved_by: ResolvedBy::Id }),
            BookIdentifier::Slug(slug) => {
                let data = self.execute::<BookIdBySlug>(book_id_by_slug::Variables { slug: slug.clone() }).await?;
                let id = data.books.first().map(|b| b.id).ok_or_else(|| Error::NotFound(format!("book with slug {slug}")))?;
                Ok(ResolvedBook { id, resolved_by: ResolvedBy::Slug })
            }
            BookIdentifier::Isbn(isbn) => {
                let data = self.execute::<BookIdByIsbn>(book_id_by_isbn::Variables { isbn: isbn.clone() }).await?;
                let id = data.editions.first().map(|e| e.book_id).ok_or_else(|| Error::NotFound(format!("edition with ISBN {isbn}")))?;
                Ok(ResolvedBook { id, resolved_by: ResolvedBy::Isbn })
            }
        }
    }

    pub async fn book(&self, id: i64) -> Result<Book> {
        let data = self.execute::<BookById>(book_by_id::Variables { id }).await?;
        let b = data.books_by_pk.ok_or_else(|| Error::NotFound(format!("book {id}")))?;
        Ok(Book {
            id: b.id,
            slug: b.slug.unwrap_or_default(),
            title: b.title.unwrap_or_default(),
            subtitle: b.subtitle,
            description: b.description,
            pages: b.pages,
            release_year: b.release_year,
            rating: b.rating,
            ratings_count: b.ratings_count,
            reviews_count: b.reviews_count,
            users_count: b.users_count,
            cover_url: b
                .cached_image
                .get("url")
                .and_then(|u| u.as_str())
                .map(str::to_owned),
            contributors: b
                .contributions
                .into_iter()
                .filter_map(|c| c.author)
                .map(|a| Contributor { id: a.id, slug: a.slug.unwrap_or_default(), name: a.name })
                .collect(),
            series: b
                .book_series
                .into_iter()
                .filter_map(|bs| bs.series.map(|s| (bs.position, s)))
                .map(|(position, s)| SeriesMembership { id: s.id, slug: s.slug, name: s.name, position })
                .collect(),
        })
    }
}

impl ClientBuilder {
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }

    pub fn build(self) -> Client {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("reqwest client");
        Client { http, base_url: self.base_url, token: self.token }
    }
}

fn hit_from_document(doc: &serde_json::Value) -> Option<SearchHit> {
    // Typesense documents carry ids as strings.
    let id = match doc.get("id")? {
        serde_json::Value::String(s) => s.parse().ok()?,
        v => v.as_i64()?,
    };
    let label = ["title", "name", "question", "username"]
        .iter()
        .find_map(|k| doc.get(k).and_then(|v| v.as_str()))
        .unwrap_or_default()
        .to_string();
    Some(SearchHit {
        id,
        slug: doc.get("slug").and_then(|v| v.as_str()).map(str::to_owned),
        label,
        document: doc.clone(),
    })
}
