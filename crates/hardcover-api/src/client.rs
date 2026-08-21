use crate::error::{Error, Result};
use crate::model::{Book, Contributor, SeriesMembership};
use crate::queries::{book_by_id, BookById};
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
        let parsed: Response<Q::ResponseData> = resp.json().await?;
        if let Some(errors) = parsed.errors.filter(|e| !e.is_empty()) {
            return Err(Error::Upstream(
                errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; "),
            ));
        }
        parsed.data.ok_or_else(|| Error::Upstream("response had no data".into()))
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
