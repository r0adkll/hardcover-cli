//! The authenticated user's Library (User Data, read-only).
use crate::client::Client;
use crate::collections::{cover_url, Page};
use crate::error::{Error, Result};
use crate::model::*;
use crate::queries::*;

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
            review: e.review_markdown.filter(|r| !r.is_empty()),
            review_html: e.review,
            review_has_spoilers: Some(e.review_has_spoilers),
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
            reads,
        })
    }
}

// ---- writes -----------------------------------------------------------------

macro_rules! read_from {
    ($r:expr) => {{
        let r = $r;
        Read {
            id: r.id,
            started_at: r.started_at,
            finished_at: r.finished_at,
            paused_at: r.paused_at,
            progress: r.progress,
            progress_pages: r.progress_pages,
            progress_seconds: r.progress_seconds,
            edition_id: r.edition_id,
        }
    }};
}

/// Hardcover's write actions report failures as an `error` string with a null payload.
fn action_error(error: Option<String>, what: &str) -> Error {
    match error {
        Some(msg) if msg.to_ascii_lowercase().contains("not found") => {
            Error::NotFound(format!("{what}: {msg}"))
        }
        Some(msg) => Error::Upstream(format!("{what}: {msg}")),
        None => Error::Upstream(format!("{what}: empty response")),
    }
}

macro_rules! dates_read_input {
    ($module:ident, $existing:expr, $u:expr) => {{
        let (existing, u): (Option<&Read>, &ProgressUpdate) = ($existing, $u);
        $module::DatesReadInput {
            id: None,
            action: None,
            action_at: None,
            edition_id: u.edition_id.or(existing.and_then(|r| r.edition_id)),
            started_at: u
                .started_at
                .clone()
                .or(existing.and_then(|r| r.started_at.clone())),
            finished_at: u
                .finished_at
                .clone()
                .or(existing.and_then(|r| r.finished_at.clone())),
            finished_at_precision: None,
            progress_pages: u.pages.or(existing.and_then(|r| r.progress_pages)),
            progress_seconds: u.seconds.or(existing.and_then(|r| r.progress_seconds)),
        }
    }};
}

impl Client {
    /// Shelve a Book. Fails upstream if it is already in the Library.
    pub async fn library_add(
        &self,
        book_id: i64,
        status: ReadingStatus,
        rating: Option<f64>,
    ) -> Result<LibraryEntry> {
        let out = self
            .execute::<InsertUserBook>(insert_user_book::Variables {
                book_id,
                status_id: status.id(),
                rating,
            })
            .await?
            .insert_user_book
            .ok_or_else(|| Error::Upstream("insert_user_book: no payload".into()))?;
        match out.user_book {
            Some(e) => Ok(entry!(e)),
            None => Err(action_error(out.error, "add to library")),
        }
    }

    pub async fn library_set_status(
        &self,
        entry_id: i64,
        status: ReadingStatus,
    ) -> Result<LibraryEntry> {
        let out = self
            .execute::<UpdateUserBookStatus>(update_user_book_status::Variables {
                id: entry_id,
                status_id: status.id(),
            })
            .await?
            .update_user_book
            .ok_or_else(|| Error::Upstream("update_user_book: no payload".into()))?;
        match out.user_book {
            Some(e) => Ok(entry!(e)),
            None => Err(action_error(out.error, "set status")),
        }
    }

    /// `None` clears the rating.
    pub async fn library_set_rating(
        &self,
        entry_id: i64,
        rating: Option<f64>,
    ) -> Result<LibraryEntry> {
        let out = self
            .execute::<UpdateUserBookRating>(update_user_book_rating::Variables {
                id: entry_id,
                rating,
            })
            .await?
            .update_user_book
            .ok_or_else(|| Error::Upstream("update_user_book: no payload".into()))?;
        match out.user_book {
            Some(e) => Ok(entry!(e)),
            None => Err(action_error(out.error, "set rating")),
        }
    }

    /// Write (or replace) the review on a Library entry. Markdown in; Hardcover renders HTML.
    /// Clearing is not supported by the API's update action as far as we've verified.
    pub async fn library_set_review(
        &self,
        entry_id: i64,
        markdown: Option<&str>,
        spoilers: bool,
    ) -> Result<LibraryEntry> {
        let out = self
            .execute::<UpdateUserBookReview>(update_user_book_review::Variables {
                id: entry_id,
                review_markdown: markdown.map(str::to_owned),
                spoilers,
            })
            .await?
            .update_user_book
            .ok_or_else(|| Error::Upstream("update_user_book: no payload".into()))?;
        match out.user_book {
            Some(e) => Ok(entry!(e)),
            None => Err(action_error(out.error, "set review")),
        }
    }

    /// Start a new Read on a Library entry.
    pub async fn read_start(&self, entry_id: i64, update: ProgressUpdate) -> Result<Read> {
        let read = dates_read_input!(insert_user_book_read, None, &update);
        let out = self
            .execute::<InsertUserBookRead>(insert_user_book_read::Variables {
                user_book_id: entry_id,
                read,
            })
            .await?
            .insert_user_book_read
            .ok_or_else(|| Error::Upstream("insert_user_book_read: no payload".into()))?;
        match out.user_book_read {
            Some(r) => Ok(read_from!(r)),
            None => Err(action_error(out.error, "start read")),
        }
    }

    /// Update an existing Read. Upstream replaces the whole record, so unset fields are
    /// carried over from `existing` rather than wiped.
    pub async fn read_update(&self, existing: &Read, update: ProgressUpdate) -> Result<Read> {
        let read = dates_read_input!(update_user_book_read, Some(existing), &update);
        let out = self
            .execute::<UpdateUserBookRead>(update_user_book_read::Variables {
                id: existing.id,
                read,
            })
            .await?
            .update_user_book_read
            .ok_or_else(|| Error::Upstream("update_user_book_read: no payload".into()))?;
        match out.user_book_read {
            Some(r) => Ok(read_from!(r)),
            None => Err(action_error(out.error, "update read")),
        }
    }

    /// Remove a Book from the Library entirely (entry, reads, rating, review).
    pub async fn library_remove(&self, entry_id: i64) -> Result<()> {
        let out = self
            .execute::<DeleteUserBook>(delete_user_book::Variables { id: entry_id })
            .await?;
        match out.delete_user_book.and_then(|d| d.id) {
            Some(_) => Ok(()),
            None => Err(Error::NotFound(format!("library entry {entry_id}"))),
        }
    }
}
