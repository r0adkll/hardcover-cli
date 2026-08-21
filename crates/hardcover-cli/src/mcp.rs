//! MCP server: one tool per CLI command, same JSON shapes, same error codes.
//! The wire is stdout, so nothing here may print; diagnostics go to stderr.
use crate::error::CliError;
use crate::ops;
use crate::output::SCHEMA_VERSION;
use crate::paging::{collect, PageArgs};
use hardcover_api::model::{BookIdentifier, Identifier, LibraryFilter, ReadingStatus, SearchType};
use hardcover_api::Client;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};

type ToolResult = Result<CallToolResult, McpError>;
type Inner = Result<CallToolResult, CliError>;

fn ok<T: Serialize>(data: &T, meta: serde_json::Value) -> Inner {
    let v = serde_json::json!({ "schema": SCHEMA_VERSION, "data": data, "meta": meta });
    Ok(CallToolResult::structured(v))
}

/// Domain failures are tool results (is_error) carrying the CLI's stable error codes,
/// not protocol errors — the model should see and act on them.
fn fail(e: CliError) -> CallToolResult {
    CallToolResult::structured_error(serde_json::json!({ "error": e }))
}

macro_rules! tool_body {
    ($e:expr) => {{
        let inner: Inner = $e;
        Ok(inner.unwrap_or_else(fail))
    }};
}

fn page(
    limit: Option<i64>,
    offset: Option<i64>,
    all: Option<bool>,
    max_rows: Option<i64>,
) -> PageArgs {
    PageArgs {
        limit: limit.unwrap_or(25),
        offset: offset.unwrap_or(0),
        all: all.unwrap_or(false),
        max_rows: max_rows.unwrap_or(1000),
    }
}

fn parse<T: std::str::FromStr<Err = String>>(s: &str) -> Result<T, CliError> {
    s.parse().map_err(CliError::usage)
}

// ---- parameter types --------------------------------------------------------

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// Free-text query.
    pub query: String,
    /// Entity type: book (default), author, series, character, list, prompt, publisher, user.
    pub query_type: Option<String>,
    /// 1-based page.
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct BookParams {
    /// Numeric id, slug, or ISBN-10/13. Prefix id:/slug:/isbn: to force a form.
    pub identifier: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct IdentParams {
    /// Numeric id or slug. Prefix id:/slug: to force a form.
    pub identifier: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct IdParams {
    pub id: i64,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct UsernameParams {
    pub username: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct Paging {
    /// Rows per page (default 25).
    pub limit: Option<i64>,
    /// 0-based offset (default 0).
    pub offset: Option<i64>,
    /// Page until exhausted or max_rows.
    pub all: Option<bool>,
    /// Cap for `all` (default 1000; 0 = unlimited).
    pub max_rows: Option<i64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct BookPaged {
    /// Numeric id, slug, or ISBN-10/13.
    pub identifier: String,
    #[serde(flatten)]
    pub paging: Paging,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct IdentPaged {
    /// Numeric id or slug.
    pub identifier: String,
    #[serde(flatten)]
    pub paging: Paging,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct LibraryListParams {
    /// want_to_read, currently_reading, read, paused, did_not_finish, ignored.
    pub status: Option<String>,
    /// Only owned books.
    pub owned: Option<bool>,
    #[serde(flatten)]
    pub paging: Paging,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SetStatusParams {
    /// Numeric id, slug, or ISBN-10/13.
    pub identifier: String,
    /// want_to_read, currently_reading, read, paused, did_not_finish, ignored.
    pub status: String,
    /// Report what would change without writing.
    pub dry_run: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct RateParams {
    pub identifier: String,
    /// 0.5–5 in half-star steps; 0 clears.
    pub rating: f64,
    pub dry_run: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ProgressParams {
    pub identifier: String,
    /// Pages read so far.
    pub pages: Option<i64>,
    /// Seconds listened so far.
    pub seconds: Option<i64>,
    /// Start date YYYY-MM-DD (defaults to today for a new read).
    pub started: Option<String>,
    /// Finish date YYYY-MM-DD or "today".
    pub finished: Option<String>,
    pub edition_id: Option<i64>,
    pub dry_run: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct RemoveParams {
    pub identifier: String,
    pub dry_run: Option<bool>,
}

// ---- server -----------------------------------------------------------------

#[derive(Clone)]
pub struct Hardcover {
    client: Client,
    // Read by the #[tool_handler] expansion; rustc's dead-code pass doesn't see through it.
    #[allow(dead_code)]
    tool_router: ToolRouter<Hardcover>,
}

macro_rules! paged_tool {
    ($self:ident, $resolve:ident, $fetch:ident, $ident:expr, $paging:expr) => {
        tool_body!(
            async {
                let ident: Identifier = parse(&$ident)?;
                let r = $self.client.$resolve(&ident).await?;
                let args = page($paging.limit, $paging.offset, $paging.all, $paging.max_rows);
                let c = collect(&args, |p| $self.client.$fetch(r.id, p)).await?;
                let mut meta = c.meta();
                meta["resolved_by"] = serde_json::json!(r.resolved_by);
                ok(&c.items, meta)
            }
            .await
        )
    };
}

macro_rules! show_tool {
    ($self:ident, $resolve:ident, $fetch:ident, $ident:expr) => {
        tool_body!(async {
            let ident: Identifier = parse(&$ident)?;
            let r = $self.client.$resolve(&ident).await?;
            let x = $self.client.$fetch(r.id).await?;
            ok(&x, serde_json::json!({ "resolved_by": r.resolved_by }))
        }
        .await)
    };
}

macro_rules! write_tool {
    ($self:ident, $ident:expr, $dry:expr, |$c:ident, $i:ident, $d:ident| $call:expr) => {
        tool_body!(async {
            let $i: BookIdentifier = parse(&$ident)?;
            let $d = $dry.unwrap_or(false);
            let $c = &$self.client;
            let (out, r) = $call.await?;
            ok(&out, serde_json::json!({ "resolved_by": r.resolved_by }))
        }
        .await)
    };
}

#[tool_router]
impl Hardcover {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Full-text search over one entity type (book by default). Hits carry stable id/slug/label plus the raw search document."
    )]
    async fn search(&self, Parameters(p): Parameters<SearchParams>) -> ToolResult {
        tool_body!(
            async {
                let t: SearchType = parse(p.query_type.as_deref().unwrap_or("book"))?;
                let r = self
                    .client
                    .search(&p.query, t, p.page.unwrap_or(1), p.per_page.unwrap_or(25))
                    .await?;
                let meta =
                    serde_json::json!({ "page": r.page, "per_page": r.per_page, "found": r.found });
                ok(&r, meta)
            }
            .await
        )
    }

    #[tool(
        description = "Show one book (a work) with contributors, series membership, counts and cover URL."
    )]
    async fn book_show(&self, Parameters(p): Parameters<BookParams>) -> ToolResult {
        tool_body!(
            async {
                let ident: BookIdentifier = parse(&p.identifier)?;
                let r = self.client.resolve_book(&ident).await?;
                let b = self.client.book(r.id).await?;
                ok(&b, serde_json::json!({ "resolved_by": r.resolved_by }))
            }
            .await
        )
    }

    #[tool(description = "List a book's editions (ISBNs, format, publisher, language).")]
    async fn book_editions(&self, Parameters(p): Parameters<BookPaged>) -> ToolResult {
        tool_body!(
            async {
                let ident: BookIdentifier = parse(&p.identifier)?;
                let r = self.client.resolve_book(&ident).await?;
                let args = page(
                    p.paging.limit,
                    p.paging.offset,
                    p.paging.all,
                    p.paging.max_rows,
                );
                let c = collect(&args, |pg| self.client.book_editions(r.id, pg)).await?;
                let mut meta = c.meta();
                meta["resolved_by"] = serde_json::json!(r.resolved_by);
                ok(&c.items, meta)
            }
            .await
        )
    }

    #[tool(description = "Show an author.")]
    async fn author_show(&self, Parameters(p): Parameters<IdentParams>) -> ToolResult {
        show_tool!(self, resolve_author, author, p.identifier)
    }

    #[tool(description = "List an author's books, most-shelved first.")]
    async fn author_books(&self, Parameters(p): Parameters<IdentPaged>) -> ToolResult {
        paged_tool!(self, resolve_author, author_books, p.identifier, p.paging)
    }

    #[tool(description = "Show a series.")]
    async fn series_show(&self, Parameters(p): Parameters<IdentParams>) -> ToolResult {
        show_tool!(self, resolve_series, series, p.identifier)
    }

    #[tool(description = "List the books in a series in position order.")]
    async fn series_books(&self, Parameters(p): Parameters<IdentPaged>) -> ToolResult {
        paged_tool!(self, resolve_series, series_books, p.identifier, p.paging)
    }

    #[tool(description = "Show a user-curated list.")]
    async fn list_show(&self, Parameters(p): Parameters<IdentParams>) -> ToolResult {
        show_tool!(self, resolve_list, list, p.identifier)
    }

    #[tool(description = "List the books in a list in position order.")]
    async fn list_books(&self, Parameters(p): Parameters<IdentPaged>) -> ToolResult {
        paged_tool!(self, resolve_list, list_books, p.identifier, p.paging)
    }

    #[tool(description = "Show one edition by id.")]
    async fn edition_show(&self, Parameters(p): Parameters<IdParams>) -> ToolResult {
        tool_body!(async { ok(&self.client.edition(p.id).await?, serde_json::json!({})) }.await)
    }

    #[tool(description = "Show a community prompt.")]
    async fn prompt_show(&self, Parameters(p): Parameters<IdentParams>) -> ToolResult {
        show_tool!(self, resolve_prompt, prompt, p.identifier)
    }

    #[tool(description = "Show a user's public profile by username.")]
    async fn user_show(&self, Parameters(p): Parameters<UsernameParams>) -> ToolResult {
        tool_body!(
            async {
                ok(
                    &self.client.user_by_username(&p.username).await?,
                    serde_json::json!({}),
                )
            }
            .await
        )
    }

    #[tool(description = "The authenticated user.")]
    async fn whoami(&self) -> ToolResult {
        tool_body!(async { ok(&self.client.me().await?, serde_json::json!({})) }.await)
    }

    #[tool(
        description = "The authenticated user's own library, most recently updated first. Optional status/owned filters."
    )]
    async fn library_list(&self, Parameters(p): Parameters<LibraryListParams>) -> ToolResult {
        tool_body!(
            async {
                let status: Option<ReadingStatus> = p.status.as_deref().map(parse).transpose()?;
                let filter = LibraryFilter {
                    status,
                    owned: p.owned.filter(|o| *o),
                };
                let args = page(
                    p.paging.limit,
                    p.paging.offset,
                    p.paging.all,
                    p.paging.max_rows,
                );
                let c = collect(&args, |pg| self.client.library(filter.clone(), pg)).await?;
                let mut meta = c.meta();
                meta["status"] = serde_json::json!(status);
                ok(&c.items, meta)
            }
            .await
        )
    }

    #[tool(
        description = "The user's library entry for one book, including every read (dates, progress) and review."
    )]
    async fn library_show(&self, Parameters(p): Parameters<BookParams>) -> ToolResult {
        tool_body!(
            async {
                let ident: BookIdentifier = parse(&p.identifier)?;
                let r = self.client.resolve_book(&ident).await?;
                let d = self.client.library_entry(r.id).await?;
                ok(&d, serde_json::json!({ "resolved_by": r.resolved_by }))
            }
            .await
        )
    }

    #[tool(
        description = "Shelve a book or move it to another reading status (adds it if absent). Returns before/after; supports dry_run."
    )]
    async fn library_set_status(&self, Parameters(p): Parameters<SetStatusParams>) -> ToolResult {
        let status: ReadingStatus = match parse(&p.status) {
            Ok(s) => s,
            Err(e) => return Ok(fail(e)),
        };
        write_tool!(self, p.identifier, p.dry_run, |c, i, d| ops::set_status(
            c, &i, status, d
        ))
    }

    #[tool(
        description = "Rate a book 0.5–5 in half stars (0 clears). Adds it as read if absent. Note: Hardcover marks rated books as read. Supports dry_run."
    )]
    async fn library_rate(&self, Parameters(p): Parameters<RateParams>) -> ToolResult {
        write_tool!(self, p.identifier, p.dry_run, |c, i, d| ops::rate(
            c, &i, p.rating, d
        ))
    }

    #[tool(
        description = "Record reading progress: updates the open read or starts one. Supports dry_run."
    )]
    async fn library_progress(&self, Parameters(p): Parameters<ProgressParams>) -> ToolResult {
        let prog = ops::Progress {
            pages: p.pages,
            seconds: p.seconds,
            started: p.started,
            finished: p.finished,
            edition: p.edition_id,
        };
        write_tool!(self, p.identifier, p.dry_run, |c, i, d| ops::progress(
            c, &i, prog, d
        ))
    }

    #[tool(
        description = "Remove a book from the user's library entirely (status, reads, rating, review). Destructive; prefer dry_run first.",
        annotations(destructive_hint = true, idempotent_hint = false)
    )]
    async fn library_remove(&self, Parameters(p): Parameters<RemoveParams>) -> ToolResult {
        write_tool!(self, p.identifier, p.dry_run, |c, i, d| ops::remove(
            c, &i, d
        ))
    }
}

#[tool_handler]
impl ServerHandler for Hardcover {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(server_identity())
            .with_instructions(
                "Hardcover.app book data. Results are {schema, data, meta}; errors are {error: {code, message}} with \
                 codes: auth_required, invalid_token, not_found, rate_limited, usage_error, upstream_error. \
                 Identifiers accept ids, slugs, or (for books) ISBNs. library_* tools act only on the token owner's \
                 own library; pass dry_run=true to preview any write."
                    .to_string(),
            )
    }
}

fn server_identity() -> Implementation {
    let mut i = Implementation::from_build_env();
    i.name = "hardcover".into();
    i.version = env!("CARGO_PKG_VERSION").into();
    i
}

pub async fn serve(client: Client) -> Result<(), CliError> {
    let running = Hardcover::new(client)
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| CliError::usage(format!("mcp: {e}")))?;
    running
        .waiting()
        .await
        .map_err(|e| CliError::usage(format!("mcp: {e}")))?;
    Ok(())
}
