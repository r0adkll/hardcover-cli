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

/// How to retry requests rejected with HTTP 429. `Retry-After` is always honoured
/// when present; otherwise delay grows exponentially from `base_delay`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Total attempts including the first. `1` disables retrying.
    pub max_attempts: u32,
    pub base_delay: std::time::Duration,
    pub max_delay: std::time::Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 3, base_delay: std::time::Duration::from_millis(500), max_delay: std::time::Duration::from_secs(60) }
    }
}

impl RetryPolicy {
    pub fn none() -> Self {
        Self { max_attempts: 1, ..Self::default() }
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    token: String,
    retry: RetryPolicy,
    raw: Option<std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>>,
}

pub struct ClientBuilder {
    base_url: String,
    token: String,
    retry: RetryPolicy,
    capture_raw: bool,
}

impl Client {
    pub fn builder(token: impl Into<String>) -> ClientBuilder {
        ClientBuilder { base_url: DEFAULT_BASE_URL.to_string(), token: token.into(), retry: RetryPolicy::default(), capture_raw: false }
    }

    /// Upstream payloads captured since the last call, oldest first. Empty unless
    /// the client was built with `capture_raw(true)`.
    pub fn take_raw(&self) -> Vec<serde_json::Value> {
        self.raw.as_ref().map(|r| std::mem::take(&mut *r.lock().unwrap())).unwrap_or_default()
    }

    pub(crate) async fn execute<Q: GraphQLQuery>(&self, variables: Q::Variables) -> Result<Q::ResponseData> {
        let body = Q::build_query(variables);
        let mut attempt = 0u32;
        let resp = loop {
            attempt += 1;
            let resp = self
                .http
                .post(format!("{}/v1/graphql", self.base_url))
                .bearer_auth(&self.token)
                .json(&body)
                .send()
                .await?;
            if resp.status() != StatusCode::TOO_MANY_REQUESTS || attempt >= self.retry.max_attempts {
                break resp;
            }
            let retry_after = retry_after_secs(&resp);
            let backoff = self.retry.base_delay.saturating_mul(2u32.saturating_pow(attempt - 1));
            let delay = retry_after
                .map(std::time::Duration::from_secs)
                .unwrap_or(backoff)
                .min(self.retry.max_delay);
            tokio::time::sleep(delay).await;
        };
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
            StatusCode::TOO_MANY_REQUESTS => return Err(Error::RateLimited { retry_after_secs: retry_after_secs(&resp) }),
            _ => {}
        }
        let raw: serde_json::Value = resp.json().await?;
        if let Some(sink) = &self.raw {
            sink.lock().unwrap().push(raw.clone());
        }
        let parsed: Response<Q::ResponseData> =
            serde_json::from_value(raw).map_err(|e| Error::Upstream(format!("unexpected response shape: {e}")))?;
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

    pub fn retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    /// Keep a copy of every upstream payload, retrievable via [`Client::take_raw`].
    pub fn capture_raw(mut self, on: bool) -> Self {
        self.capture_raw = on;
        self
    }

    pub fn build(self) -> Client {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("reqwest client");
        Client {
            http,
            base_url: self.base_url,
            token: self.token,
            retry: self.retry,
            raw: self.capture_raw.then(Default::default),
        }
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

fn retry_after_secs(resp: &reqwest::Response) -> Option<u64> {
    resp.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok())
}
