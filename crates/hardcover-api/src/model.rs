//! Domain structs owned by this crate. See CONTEXT.md and ADR 0001.
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Book {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub pages: Option<i64>,
    pub release_year: Option<i64>,
    pub rating: Option<f64>,
    pub ratings_count: i64,
    pub reviews_count: i64,
    pub users_count: i64,
    pub cover_url: Option<String>,
    pub contributors: Vec<Contributor>,
    pub series: Vec<SeriesMembership>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Contributor {
    pub id: i64,
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SeriesMembership {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub position: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub name: Option<String>,
}

/// Entity type a Search targets. One type per search.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    Book,
    Author,
    Series,
    Character,
    List,
    Prompt,
    Publisher,
    User,
}

impl SearchType {
    pub fn as_str(self) -> &'static str {
        match self {
            SearchType::Book => "book",
            SearchType::Author => "author",
            SearchType::Series => "series",
            SearchType::Character => "character",
            SearchType::List => "list",
            SearchType::Prompt => "prompt",
            SearchType::Publisher => "publisher",
            SearchType::User => "user",
        }
    }
}

impl std::str::FromStr for SearchType {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "book" | "books" => SearchType::Book,
            "author" | "authors" => SearchType::Author,
            "series" => SearchType::Series,
            "character" | "characters" => SearchType::Character,
            "list" | "lists" => SearchType::List,
            "prompt" | "prompts" => SearchType::Prompt,
            "publisher" | "publishers" => SearchType::Publisher,
            "user" | "users" => SearchType::User,
            other => return Err(format!("unknown search type: {other}")),
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SearchResults {
    pub query: String,
    pub query_type: SearchType,
    pub page: i64,
    pub per_page: i64,
    /// Total matches across all pages.
    pub found: i64,
    pub hits: Vec<SearchHit>,
}

/// One search match. `id`, `slug` and `label` are stable; `document` is the
/// upstream search document for the matched entity, passed through as-is.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SearchHit {
    pub id: i64,
    pub slug: Option<String>,
    pub label: String,
    pub document: serde_json::Value,
}
