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
