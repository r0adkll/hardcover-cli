//! The authenticated user's Library (User Data, read-only).
use crate::client::Client;
use crate::collections::{cover_url, Page};
use crate::error::{Error, Result};
use crate::model::*;
use crate::queries::{library, library_entry_query, Library, LibraryEntryQuery};

macro_rules! entry {
    ($e:expr) => {{
        let e = $e;
        let status_id = e.status_id;
        LibraryEntry {
            id: e.id,
            book: BookSummary {
                id: e.book.id,
                slug: e.book.slug.unwrap_or_default(),
                title: e.book.title.unwrap_or_default(),
                release_year: e.book.release_year,
                rating: e.book.rating,
                users_count: e.book.users_count,
                cover_url: cover_url(&e.book.cached_image),
            },
            status: ReadingStatus::from_id(status_id).unwrap_or(ReadingStatus::Ignored),
            status_id,
            rating: e.rating,
            owned: e.owned,
            read_count: e.read_count,
            privacy: privacy_name(e.privacy_setting_id),
            privacy_id: e.privacy_setting_id,
            has_review: e.has_review,
            reviewed_at: e.reviewed_at,
            first_read_date: e.first_read_date,
            last_read_date: e.last_read_date,
            date_added: e.date_added,
            updated_at: e.updated_at,
            edition_id: e.edition_id,
        }
    }};
}

impl Client {
    /// The caller's user id, fetched once via `me` and cached for the client's lifetime.
    pub async fn me_id(&self) -> Result<i64> {
        if let Some(id) = *self.me_id.lock().unwrap() {
            return Ok(id);
        }
        let id = self.me().await?.id;
        *self.me_id.lock().unwrap() = Some(id);
        Ok(id)
    }

    /// The user's Library, most recently updated first.
    pub async fn library(&self, filter: LibraryFilter, page: Page) -> Result<Vec<LibraryEntry>> {
        let user_id = self.me_id().await?;
        let status_ids = match filter.status {
            Some(s) => vec![s.id()],
            None => ReadingStatus::ALL.iter().map(|s| s.id()).collect(),
        };
        let owned = match filter.owned {
            Some(o) => vec![o],
            None => vec![true, false],
        };
        let data = self
            .execute::<Library>(library::Variables {
                user_id,
                status_ids,
                owned,
                limit: page.limit,
                offset: page.offset,
            })
            .await?;
        Ok(data.user_books.into_iter().map(|e| entry!(e)).collect())
    }

    /// The user's entry for one Book, with reads and review.
    pub async fn library_entry(&self, book_id: i64) -> Result<LibraryEntryDetail> {
        let user_id = self.me_id().await?;
        let e = self
            .execute::<LibraryEntryQuery>(library_entry_query::Variables { user_id, book_id })
            .await?
            .user_books
            .into_iter()
            .next()
            .ok_or_else(|| Error::NotFound(format!("book {book_id} is not in your library")))?;
        let review = e.review.clone();
        let reads = e
            .user_book_reads
            .iter()
            .map(|r| Read {
                id: r.id,
                started_at: r.started_at.clone(),
                finished_at: r.finished_at.clone(),
                paused_at: r.paused_at.clone(),
                progress: r.progress,
                progress_pages: r.progress_pages,
                progress_seconds: r.progress_seconds,
                edition_id: r.edition_id,
            })
            .collect();
        Ok(LibraryEntryDetail {
            entry: entry!(e),
            review,
            reads,
        })
    }
}
