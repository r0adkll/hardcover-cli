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

/// How a Book may be identified on input. See CONTEXT.md "Identifier".
///
/// Parsing rules: `id:`, `slug:`, `isbn:` prefixes force a form. Otherwise
/// all-digit strings are an Id, 10/13-character ISBN-shaped strings (hyphens
/// ignored, trailing X allowed) are an Isbn, anything else is a Slug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookIdentifier {
    Id(i64),
    Slug(String),
    Isbn(String),
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedBy {
    Id,
    Slug,
    Isbn,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedBook {
    pub id: i64,
    pub resolved_by: ResolvedBy,
}

fn normalize_isbn(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| *c != '-' && *c != ' ').collect();
    let valid_shape = match digits.len() {
        10 => digits[..9].chars().all(|c| c.is_ascii_digit()) && digits.ends_with(|c: char| c.is_ascii_digit() || c == 'X' || c == 'x'),
        13 => digits.chars().all(|c| c.is_ascii_digit()),
        _ => false,
    };
    valid_shape.then(|| digits.to_ascii_uppercase())
}

impl std::str::FromStr for BookIdentifier {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        let s = s.trim();
        if let Some(rest) = s.strip_prefix("id:") {
            return rest.parse().map(BookIdentifier::Id).map_err(|_| format!("not a numeric id: {rest}"));
        }
        if let Some(rest) = s.strip_prefix("slug:") {
            return Ok(BookIdentifier::Slug(rest.to_string()));
        }
        if let Some(rest) = s.strip_prefix("isbn:") {
            return normalize_isbn(rest).map(BookIdentifier::Isbn).ok_or_else(|| format!("not an ISBN: {rest}"));
        }
        if s.is_empty() {
            return Err("empty identifier".into());
        }
        // 10-digit all-numeric strings are ambiguous between id and ISBN-10; ISBN-13
        // is always 13 digits. Treat exact ISBN lengths as ISBN, other digit runs as ids.
        if let Some(isbn) = normalize_isbn(s) {
            return Ok(BookIdentifier::Isbn(isbn));
        }
        if s.chars().all(|c| c.is_ascii_digit()) {
            return s.parse().map(BookIdentifier::Id).map_err(|e| e.to_string());
        }
        Ok(BookIdentifier::Slug(s.to_string()))
    }
}

/// A Book as it appears inside a collection: enough to identify and rank it.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BookSummary {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub release_year: Option<i64>,
    pub rating: Option<f64>,
    pub users_count: i64,
    pub cover_url: Option<String>,
}

/// A Book's place in a Series.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SeriesEntry {
    pub position: Option<f64>,
    pub book: BookSummary,
}

/// A Book's place in a List.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ListEntry {
    pub position: Option<i64>,
    pub reason: Option<String>,
    pub book: BookSummary,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Edition {
    pub id: i64,
    pub book_id: i64,
    pub title: String,
    pub subtitle: Option<String>,
    pub isbn_10: Option<String>,
    pub isbn_13: Option<String>,
    pub asin: Option<String>,
    /// Reading format: physical, audiobook, both, ebook.
    pub format: Option<String>,
    /// Publisher-described form, e.g. "Paperback", "Kindle Edition".
    pub edition_format: Option<String>,
    pub pages: Option<i64>,
    pub audio_seconds: Option<i64>,
    pub release_date: Option<String>,
    pub language: Option<String>,
    pub publisher: Option<Publisher>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Publisher {
    pub id: i64,
    pub name: Option<String>,
}

pub(crate) fn reading_format(id: Option<i64>) -> Option<String> {
    Some(
        match id? {
            1 => "physical",
            2 => "audiobook",
            3 => "both",
            4 => "ebook",
            _ => return None,
        }
        .to_string(),
    )
}
