//! Collection queries: paged listings beneath a parent entity.
use crate::client::Client;
use crate::error::Result;
use crate::model::{reading_format, BookSummary, Edition, ListEntry, Publisher, SeriesEntry};
use crate::queries::*;

/// One page of a collection. `offset` is 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub limit: i64,
    pub offset: i64,
}

pub(crate) fn cover_url(v: &serde_json::Value) -> Option<String> {
    v.get("url").and_then(|u| u.as_str()).map(str::to_owned)
}

macro_rules! summary {
    ($b:expr) => {
        BookSummary {
            id: $b.id,
            slug: $b.slug.unwrap_or_default(),
            title: $b.title.unwrap_or_default(),
            release_year: $b.release_year,
            rating: $b.rating,
            users_count: $b.users_count,
            cover_url: cover_url(&$b.cached_image),
        }
    };
}

impl Client {
    pub async fn author_books(&self, author_id: i64, page: Page) -> Result<Vec<BookSummary>> {
        let data = self
            .execute::<AuthorBooks>(author_books::Variables { author_id, limit: page.limit, offset: page.offset })
            .await?;
        Ok(data.books.into_iter().map(|b| summary!(b)).collect())
    }

    pub async fn series_books(&self, series_id: i64, page: Page) -> Result<Vec<SeriesEntry>> {
        let data = self
            .execute::<SeriesBooks>(series_books::Variables { series_id, limit: page.limit, offset: page.offset })
            .await?;
        Ok(data
            .book_series
            .into_iter()
            .filter_map(|e| e.book.map(|b| SeriesEntry { position: e.position, book: summary!(b) }))
            .collect())
    }

    pub async fn list_books(&self, list_id: i64, page: Page) -> Result<Vec<ListEntry>> {
        let data = self
            .execute::<ListBooks>(list_books::Variables { list_id, limit: page.limit, offset: page.offset })
            .await?;
        Ok(data
            .list_books
            .into_iter()
            .map(|e| ListEntry { position: e.position, reason: e.reason, book: summary!(e.book) })
            .collect())
    }

    pub async fn book_editions(&self, book_id: i64, page: Page) -> Result<Vec<Edition>> {
        let data = self
            .execute::<BookEditions>(book_editions::Variables { book_id, limit: page.limit, offset: page.offset })
            .await?;
        Ok(data
            .editions
            .into_iter()
            .map(|e| Edition {
                id: e.id,
                book_id: e.book_id,
                title: e.title.unwrap_or_default(),
                subtitle: e.subtitle,
                isbn_10: e.isbn_10,
                isbn_13: e.isbn_13,
                asin: e.asin,
                format: reading_format(Some(e.reading_format_id)),
                edition_format: e.edition_format,
                pages: e.pages,
                audio_seconds: e.audio_seconds,
                release_date: e.release_date,
                language: e.language.map(|l| l.language),
                publisher: e.publisher.map(|p| Publisher { id: p.id, name: p.name }),
                cover_url: cover_url(&e.cached_image),
            })
            .collect())
    }
}
