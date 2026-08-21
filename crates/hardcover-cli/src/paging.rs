//! `--limit/--offset/--all` handling shared by every collection command.
use crate::error::CliError;
use hardcover_api::Page;
use serde::Serialize;
use std::future::Future;

#[derive(Debug, Clone, clap::Args)]
pub struct PageArgs {
    /// Rows per page (per request when combined with --all).
    #[arg(long, default_value_t = 25)]
    pub limit: i64,
    /// 0-based row offset to start from.
    #[arg(long, default_value_t = 0)]
    pub offset: i64,
    /// Keep paging until the collection is exhausted or --max-rows is reached.
    #[arg(long)]
    pub all: bool,
    /// Safety cap for --all; `meta.truncated` is true when it is hit.
    #[arg(long, default_value_t = 1000)]
    pub max_rows: i64,
}

#[derive(Debug, Serialize)]
pub struct Collected<T> {
    pub items: Vec<T>,
    pub limit: i64,
    pub offset: i64,
    pub truncated: bool,
}

impl<T> Collected<T> {
    pub fn meta(&self) -> serde_json::Value {
        serde_json::json!({
            "limit": self.limit,
            "offset": self.offset,
            "count": self.items.len(),
            "truncated": self.truncated,
        })
    }
}

/// Fetch one page, or with `--all` keep fetching until a short page or the cap.
pub async fn collect<T, F, Fut>(args: &PageArgs, fetch: F) -> Result<Collected<T>, CliError>
where
    F: Fn(Page) -> Fut,
    Fut: Future<Output = hardcover_api::Result<Vec<T>>>,
{
    if args.limit < 1 {
        return Err(CliError::usage("--limit must be at least 1"));
    }
    let mut items = Vec::new();
    let mut offset = args.offset;
    let mut truncated = false;
    loop {
        let remaining_cap = if args.all { args.max_rows - items.len() as i64 } else { args.limit };
        let limit = args.limit.min(remaining_cap.max(0));
        if limit == 0 {
            truncated = true;
            break;
        }
        let page = fetch(Page { limit, offset }).await?;
        let got = page.len() as i64;
        items.extend(page);
        if !args.all || got < limit {
            break;
        }
        if items.len() as i64 >= args.max_rows {
            truncated = true;
            break;
        }
        offset += got;
    }
    Ok(Collected { items, limit: args.limit, offset: args.offset, truncated })
}
