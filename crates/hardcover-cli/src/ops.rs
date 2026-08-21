//! Library write operations shared by the CLI commands and the MCP tools.
//! Every write resolves the book, captures `before`, performs (or plans) the mutation,
//! and reports `after` from the mutation response — see README "Writes".
use crate::error::CliError;
use hardcover_api::model::{
    BookIdentifier, LibraryEntry, LibraryEntryDetail, ProgressUpdate, Read, ReadingStatus,
    ResolvedBook,
};
use hardcover_api::Client;
use serde::Serialize;

/// What a write did, with the entry before and after so callers can verify.
#[derive(Debug, Serialize)]
pub struct WriteResult {
    pub action: &'static str,
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned: Option<serde_json::Value>,
    pub before: Option<LibraryEntryDetail>,
    pub after: Option<LibraryEntryDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read: Option<Read>,
}

impl WriteResult {
    fn new(action: &'static str, dry_run: bool, before: Option<LibraryEntryDetail>) -> Self {
        Self {
            action,
            dry_run,
            planned: None,
            before,
            after: None,
            read: None,
        }
    }

    pub fn line(&self) -> String {
        let name = |e: &Option<LibraryEntryDetail>| {
            e.as_ref()
                .map(|e| e.entry.book.title.clone())
                .unwrap_or_default()
        };
        let title = if self.before.is_some() {
            name(&self.before)
        } else {
            name(&self.after)
        };
        let state = |e: &Option<LibraryEntryDetail>| {
            e.as_ref()
                .map(|e| {
                    format!(
                        "{}{}",
                        e.entry.status.as_str(),
                        e.entry.rating.map(|r| format!(" ★{r}")).unwrap_or_default()
                    )
                })
                .unwrap_or_else(|| "not in library".into())
        };
        let mut s = format!(
            "{}{}: {} → {}",
            if self.dry_run { "[dry-run] " } else { "" },
            self.action,
            state(&self.before),
            state(&self.after)
        );
        if let Some(p) = &self.planned {
            s = format!("{s}  planned {p}");
        }
        if let Some(r) = &self.read {
            s.push_str(&format!(
                "  read #{} {}%",
                r.id,
                r.progress.map(|p| p.round() as i64).unwrap_or(0)
            ));
        }
        if !title.is_empty() {
            s = format!("{title} — {s}");
        }
        s
    }
}

pub struct Progress {
    pub pages: Option<i64>,
    pub seconds: Option<i64>,
    pub started: Option<String>,
    pub finished: Option<String>,
    pub edition: Option<i64>,
}

pub async fn entry_or_none(
    client: &Client,
    book_id: i64,
) -> Result<Option<LibraryEntryDetail>, CliError> {
    match client.library_entry(book_id).await {
        Ok(e) => Ok(Some(e)),
        Err(hardcover_api::Error::NotFound(_)) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Hardcover's reads lag its writes, so a re-fetch right after a mutation can be stale.
/// The mutation response is authoritative for the entry (and the touched read); the
/// re-fetch only supplies what the mutation didn't return (other reads, review).
async fn after_write(
    client: &Client,
    book_id: i64,
    entry: LibraryEntry,
    read: Option<&Read>,
) -> Result<Option<LibraryEntryDetail>, CliError> {
    let fetched = entry_or_none(client, book_id).await?;
    let mut reads = fetched.map(|f| f.reads).unwrap_or_default();
    if let Some(r) = read {
        match reads.iter_mut().find(|x| x.id == r.id) {
            Some(slot) => *slot = r.clone(),
            None => reads.push(r.clone()),
        }
    }
    Ok(Some(LibraryEntryDetail { entry, reads }))
}

pub fn validate_rating(r: f64) -> Result<Option<f64>, CliError> {
    if r == 0.0 {
        return Ok(None);
    }
    if !(0.5..=5.0).contains(&r) || (r * 2.0).fract() != 0.0 {
        return Err(CliError::usage(format!(
            "rating must be 0.5–5 in half-star steps, or 0 to clear (got {r})"
        )));
    }
    Ok(Some(r))
}

pub fn today() -> String {
    // Date-only, UTC; civil-from-days (Howard Hinnant) to avoid pulling in chrono.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn date_arg(v: Option<String>) -> Option<String> {
    v.map(|d| {
        if d.eq_ignore_ascii_case("today") {
            today()
        } else {
            d
        }
    })
}

pub async fn set_status(
    client: &Client,
    ident: &BookIdentifier,
    status: ReadingStatus,
    dry_run: bool,
) -> Result<(WriteResult, ResolvedBook), CliError> {
    let r = client.resolve_book(ident).await?;
    let before = entry_or_none(client, r.id).await?;
    let mut out = WriteResult::new(
        if before.is_some() { "updated" } else { "added" },
        dry_run,
        before,
    );
    if dry_run {
        out.planned = Some(serde_json::json!({ "status": status }));
    } else {
        let updated = match &out.before {
            Some(b) => client.library_set_status(b.entry.id, status).await?,
            None => client.library_add(r.id, status, None).await?,
        };
        out.after = after_write(client, r.id, updated, None).await?;
    }
    Ok((out, r))
}

pub async fn rate(
    client: &Client,
    ident: &BookIdentifier,
    rating: f64,
    dry_run: bool,
) -> Result<(WriteResult, ResolvedBook), CliError> {
    let rating = validate_rating(rating)?;
    let r = client.resolve_book(ident).await?;
    let before = entry_or_none(client, r.id).await?;
    let mut out = WriteResult::new(
        if before.is_some() {
            "rated"
        } else {
            "added_and_rated"
        },
        dry_run,
        before,
    );
    if dry_run {
        out.planned = Some(serde_json::json!({ "rating": rating }));
    } else {
        let updated = match &out.before {
            Some(b) => client.library_set_rating(b.entry.id, rating).await?,
            None => {
                client
                    .library_add(r.id, ReadingStatus::Read, rating)
                    .await?
            }
        };
        out.after = after_write(client, r.id, updated, None).await?;
    }
    Ok((out, r))
}

pub async fn progress(
    client: &Client,
    ident: &BookIdentifier,
    p: Progress,
    dry_run: bool,
) -> Result<(WriteResult, ResolvedBook), CliError> {
    if p.pages.is_none() && p.seconds.is_none() && p.finished.is_none() && p.started.is_none() {
        return Err(CliError::usage(
            "give at least one of pages, seconds, started, finished",
        ));
    }
    let r = client.resolve_book(ident).await?;
    let before = entry_or_none(client, r.id).await?;
    let open_read = before.as_ref().and_then(|b| {
        b.reads
            .iter()
            .rev()
            .find(|x| x.finished_at.is_none())
            .cloned()
    });
    let action = match (&before, &open_read) {
        (None, _) => "added_and_started_read",
        (Some(_), None) => "started_read",
        (Some(_), Some(_)) => "updated_read",
    };
    let mut update = ProgressUpdate {
        pages: p.pages,
        seconds: p.seconds,
        started_at: date_arg(p.started),
        finished_at: date_arg(p.finished),
        edition_id: p.edition,
    };
    if open_read.is_none() && update.started_at.is_none() {
        update.started_at = Some(today());
    }
    let mut out = WriteResult::new(action, dry_run, before);
    if dry_run {
        out.planned = Some(serde_json::json!({
            "read_id": open_read.as_ref().map(|x| x.id),
            "pages": update.pages, "seconds": update.seconds,
            "started_at": update.started_at, "finished_at": update.finished_at, "edition_id": update.edition_id,
        }));
    } else {
        let entry = match &out.before {
            Some(b) => b.entry.clone(),
            None => {
                client
                    .library_add(r.id, ReadingStatus::CurrentlyReading, None)
                    .await?
            }
        };
        let read = match &open_read {
            Some(existing) => client.read_update(existing, update).await?,
            None => client.read_start(entry.id, update).await?,
        };
        out.after = after_write(client, r.id, entry, Some(&read)).await?;
        out.read = Some(read);
    }
    Ok((out, r))
}

pub async fn remove(
    client: &Client,
    ident: &BookIdentifier,
    dry_run: bool,
) -> Result<(WriteResult, ResolvedBook), CliError> {
    let r = client.resolve_book(ident).await?;
    let before = client.library_entry(r.id).await?;
    let entry_id = before.entry.id;
    let mut out = WriteResult::new("removed", dry_run, Some(before));
    if dry_run {
        out.planned = Some(serde_json::json!({ "delete_entry_id": entry_id }));
    } else {
        client.library_remove(entry_id).await?;
    }
    Ok((out, r))
}

pub async fn review(
    client: &Client,
    ident: &BookIdentifier,
    markdown: &str,
    spoilers: bool,
    dry_run: bool,
) -> Result<(WriteResult, ResolvedBook), CliError> {
    if markdown.trim().is_empty() {
        return Err(CliError::usage("review text is empty"));
    }
    let r = client.resolve_book(ident).await?;
    let before = client.library_entry(r.id).await?; // must already be shelved
    let entry_id = before.entry.id;
    let mut out = WriteResult::new("reviewed", dry_run, Some(before));
    if dry_run {
        out.planned = Some(serde_json::json!({ "review": markdown, "spoilers": spoilers }));
    } else {
        let updated = client
            .library_set_review(entry_id, Some(markdown), spoilers)
            .await?;
        out.after = after_write(client, r.id, updated, None).await?;
    }
    Ok((out, r))
}
