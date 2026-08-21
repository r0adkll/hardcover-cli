use crate::cli::*;
use crate::credentials;
use crate::error::CliError;
use crate::output::{self, Format};
use crate::paging::collect;
use hardcover_api::model::{
    BookSummary, LibraryEntryDetail, LibraryFilter, ProgressUpdate, Read, ReadingStatus, Resolved,
    User,
};
use hardcover_api::{Client, RetryPolicy};
use serde::Serialize;

struct Ctx<'a> {
    format: Format,
    dry_run: bool,
    raw: bool,
    no_retry: bool,
    api_url: &'a Option<String>,
}

impl Ctx<'_> {
    fn client(&self, token: String) -> Client {
        let mut b = Client::builder(token).capture_raw(self.raw);
        if let Some(url) = self.api_url {
            b = b.base_url(url);
        }
        if self.no_retry {
            b = b.retry(RetryPolicy::none());
        }
        b.build()
    }

    /// Under --raw, domain output is suppressed; the captured upstream payload is printed at the end.
    fn emit<T: Serialize>(&self, value: &T, meta: serde_json::Value, plain: impl Fn(&T) -> String) {
        if !self.raw {
            output::emit(self.format, value, meta, plain);
        }
    }

    fn emit_list<T: Serialize>(
        &self,
        items: &[T],
        meta: serde_json::Value,
        line: impl Fn(&T) -> String,
    ) {
        if !self.raw {
            output::emit_list(self.format, items, meta, line);
        }
    }

    fn finish(&self, client: &Client) {
        if self.raw {
            let mut payloads = client.take_raw();
            let value = if payloads.len() == 1 {
                payloads.pop().unwrap()
            } else {
                serde_json::Value::Array(payloads)
            };
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
    }
}

/// Every write reports what it did and the entry before and after, so callers can verify.
#[derive(Serialize)]
struct WriteResult {
    action: &'static str,
    dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    planned: Option<serde_json::Value>,
    before: Option<LibraryEntryDetail>,
    after: Option<LibraryEntryDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read: Option<Read>,
}

impl WriteResult {
    fn line(&self) -> String {
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

async fn entry_or_none(
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
    entry: hardcover_api::model::LibraryEntry,
    read: Option<&Read>,
) -> Result<Option<LibraryEntryDetail>, CliError> {
    let fetched = entry_or_none(client, book_id).await?;
    let (mut reads, review) = fetched.map(|f| (f.reads, f.review)).unwrap_or_default();
    if let Some(r) = read {
        match reads.iter_mut().find(|x| x.id == r.id) {
            Some(slot) => *slot = r.clone(),
            None => reads.push(r.clone()),
        }
    }
    Ok(Some(LibraryEntryDetail {
        entry,
        review,
        reads,
    }))
}

fn validate_rating(r: f64) -> Result<Option<f64>, CliError> {
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

fn today() -> String {
    // Date-only, UTC; good enough for "started today" without pulling in chrono.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let days = secs.div_euclid(86_400);
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
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

fn none() -> serde_json::Value {
    serde_json::json!({})
}

fn resolved_meta(r: &Resolved) -> serde_json::Value {
    serde_json::json!({ "resolved_by": r.resolved_by })
}

fn with_resolved(mut meta: serde_json::Value, resolved_by: impl Serialize) -> serde_json::Value {
    meta["resolved_by"] = serde_json::json!(resolved_by);
    meta
}

fn user_line(u: &User) -> String {
    format!("{} (#{})", u.username, u.id)
}

fn summary_line(b: &BookSummary) -> String {
    format!(
        "#{:<9} {}{}",
        b.id,
        b.title,
        b.release_year
            .map(|y| format!(" ({y})"))
            .unwrap_or_default()
    )
}

fn positioned<P: ToString>(position: Option<P>, b: &BookSummary) -> String {
    format!(
        "{:>5}  {}",
        position.map(|p| p.to_string()).unwrap_or_default(),
        summary_line(b)
    )
}

pub async fn run(cli: Cli) -> Result<(), CliError> {
    let mut config = crate::config::load();
    let format = match (cli.format, config.format.as_deref()) {
        (Format::Auto, Some(name)) => <Format as clap::ValueEnum>::from_str(name, true)
            .map_err(|_| CliError::usage(format!("config.toml: unknown format {name:?}")))?,
        (f, _) => f,
    };
    let ctx = Ctx {
        format,
        dry_run: cli.dry_run,
        raw: cli.raw,
        no_retry: cli.no_retry,
        api_url: &cli.api_url,
    };

    match cli.command {
        Command::Schema => {
            ctx.emit(&crate::schema::describe(), none(), |_| {
                crate::schema::plain()
            });
            return Ok(());
        }
        Command::Login => {
            let token = credentials::read_login_token()?;
            let client = ctx.client(token.clone());
            let user = client.me().await?;
            credentials::store(&token)?;
            config.username = Some(user.username.clone());
            crate::config::save(&config)?;
            ctx.emit(&user, none(), |u| format!("Logged in as {}", user_line(u)));
            ctx.finish(&client);
            return Ok(());
        }
        Command::Logout => {
            credentials::clear()?;
            config.username = None;
            crate::config::save(&config)?;
            ctx.emit(&serde_json::json!({ "logged_out": true }), none(), |_| {
                "Logged out".into()
            });
            return Ok(());
        }
        _ => {}
    }

    let client = ctx.client(credentials::resolve(cli.token)?);

    match cli.command {
        Command::Schema | Command::Login | Command::Logout => unreachable!(),
        Command::Whoami => {
            let user = client.me().await?;
            ctx.emit(&user, none(), user_line);
        }
        Command::Search {
            query,
            query_type,
            page,
            per_page,
        } => {
            let r = client.search(&query, query_type, page, per_page).await?;
            let meta =
                serde_json::json!({ "page": r.page, "per_page": r.per_page, "found": r.found });
            ctx.emit(&r, meta, |r| {
                let mut lines = vec![format!(
                    "{} results for \"{}\" ({})",
                    r.found,
                    r.query,
                    r.query_type.as_str()
                )];
                for h in &r.hits {
                    lines.push(format!(
                        "  #{:<8} {}  [{}]",
                        h.id,
                        h.label,
                        h.slug.as_deref().unwrap_or("-")
                    ));
                }
                lines.join("\n")
            });
        }
        Command::Book { command } => match command {
            BookCommand::Show { identifier } => {
                let r = client.resolve_book(&identifier).await?;
                let book = client.book(r.id).await?;
                ctx.emit(
                    &book,
                    serde_json::json!({ "resolved_by": r.resolved_by }),
                    |b| {
                        let by = b
                            .contributors
                            .iter()
                            .map(|c| c.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{} — {} (#{}, {})", b.title, by, b.id, b.slug)
                    },
                );
            }
            BookCommand::Editions { identifier, page } => {
                let r = client.resolve_book(&identifier).await?;
                let c = collect(&page, |p| client.book_editions(r.id, p)).await?;
                ctx.emit_list(&c.items, with_resolved(c.meta(), r.resolved_by), |e| {
                    format!(
                        "#{:<9} {:<10} {:<14} {}",
                        e.id,
                        e.format.as_deref().unwrap_or("-"),
                        e.isbn_13.as_deref().or(e.isbn_10.as_deref()).unwrap_or("-"),
                        e.edition_format.as_deref().unwrap_or("")
                    )
                });
            }
        },
        Command::Author { command } => match command {
            AuthorCommand::Show { identifier } => {
                let r = client.resolve_author(&identifier).await?;
                let a = client.author(r.id).await?;
                ctx.emit(&a, resolved_meta(&r), |a| {
                    format!(
                        "{} (#{}, {}) — {} books",
                        a.name, a.id, a.slug, a.books_count
                    )
                });
            }
            AuthorCommand::Books { identifier, page } => {
                let r = client.resolve_author(&identifier).await?;
                let c = collect(&page, |p| client.author_books(r.id, p)).await?;
                ctx.emit_list(
                    &c.items,
                    with_resolved(c.meta(), r.resolved_by),
                    summary_line,
                );
            }
        },
        Command::Series { command } => match command {
            SeriesCommand::Show { identifier } => {
                let r = client.resolve_series(&identifier).await?;
                let x = client.series(r.id).await?;
                ctx.emit(&x, resolved_meta(&r), |x| {
                    format!(
                        "{} (#{}, {}) — {} books",
                        x.name, x.id, x.slug, x.books_count
                    )
                });
            }
            SeriesCommand::Books { identifier, page } => {
                let r = client.resolve_series(&identifier).await?;
                let c = collect(&page, |p| client.series_books(r.id, p)).await?;
                ctx.emit_list(&c.items, with_resolved(c.meta(), r.resolved_by), |e| {
                    positioned(e.position, &e.book)
                });
            }
        },
        Command::List { command } => match command {
            ListCommand::Show { identifier } => {
                let r = client.resolve_list(&identifier).await?;
                let l = client.list(r.id).await?;
                ctx.emit(&l, resolved_meta(&r), |l| {
                    format!(
                        "{} (#{}) by {} — {} books",
                        l.name, l.id, l.owner.username, l.books_count
                    )
                });
            }
            ListCommand::Books { identifier, page } => {
                let r = client.resolve_list(&identifier).await?;
                let c = collect(&page, |p| client.list_books(r.id, p)).await?;
                ctx.emit_list(&c.items, with_resolved(c.meta(), r.resolved_by), |e| {
                    positioned(e.position, &e.book)
                });
            }
        },
        Command::Edition {
            command: EditionCommand::Show { id },
        } => {
            let e = client.edition(id).await?;
            ctx.emit(&e, none(), |e| {
                format!(
                    "{} (#{}) {} {}",
                    e.title,
                    e.id,
                    e.isbn_13.as_deref().or(e.isbn_10.as_deref()).unwrap_or("-"),
                    e.edition_format.as_deref().unwrap_or("")
                )
            });
        }
        Command::Prompt {
            command: PromptCommand::Show { identifier },
        } => {
            let r = client.resolve_prompt(&identifier).await?;
            let p = client.prompt(r.id).await?;
            ctx.emit(&p, resolved_meta(&r), |p| {
                format!(
                    "{} (#{}, {}) — {} answers",
                    p.question, p.id, p.slug, p.answers_count
                )
            });
        }
        Command::Library { command } => match command {
            LibraryCommand::List {
                status,
                owned,
                page,
            } => {
                let filter = LibraryFilter {
                    status,
                    owned: owned.then_some(true),
                };
                let c = collect(&page, |p| client.library(filter.clone(), p)).await?;
                let mut meta = c.meta();
                meta["status"] = status
                    .map(|s| serde_json::json!(s))
                    .unwrap_or(serde_json::Value::Null);
                meta["owned"] = serde_json::json!(owned);
                ctx.emit_list(&c.items, meta, |e| {
                    format!(
                        "{:<18} {:>4}  {}",
                        e.status.as_str(),
                        e.rating.map(|r| format!("{r}")).unwrap_or_default(),
                        summary_line(&e.book)
                    )
                });
            }
            LibraryCommand::SetStatus { identifier, status } => {
                let r = client.resolve_book(&identifier).await?;
                let before = entry_or_none(&client, r.id).await?;
                let action = if before.is_some() { "updated" } else { "added" };
                let mut out = WriteResult {
                    action,
                    dry_run: ctx.dry_run,
                    planned: None,
                    before,
                    after: None,
                    read: None,
                };
                if ctx.dry_run {
                    out.planned = Some(serde_json::json!({ "status": status }));
                } else {
                    let updated = match &out.before {
                        Some(b) => client.library_set_status(b.entry.id, status).await?,
                        None => client.library_add(r.id, status, None).await?,
                    };
                    out.after = after_write(&client, r.id, updated, None).await?;
                }
                ctx.emit(
                    &out,
                    serde_json::json!({ "resolved_by": r.resolved_by }),
                    WriteResult::line,
                );
            }
            LibraryCommand::Rate { identifier, rating } => {
                let rating = validate_rating(rating)?;
                let r = client.resolve_book(&identifier).await?;
                let before = entry_or_none(&client, r.id).await?;
                let action = if before.is_some() {
                    "rated"
                } else {
                    "added_and_rated"
                };
                let mut out = WriteResult {
                    action,
                    dry_run: ctx.dry_run,
                    planned: None,
                    before,
                    after: None,
                    read: None,
                };
                if ctx.dry_run {
                    out.planned = Some(serde_json::json!({ "rating": rating }));
                } else {
                    match &out.before {
                        Some(b) => client.library_set_rating(b.entry.id, rating).await?,
                        None => {
                            client
                                .library_add(r.id, ReadingStatus::Read, rating)
                                .await?
                        }
                    };
                    out.after = entry_or_none(&client, r.id).await?;
                }
                ctx.emit(
                    &out,
                    serde_json::json!({ "resolved_by": r.resolved_by }),
                    WriteResult::line,
                );
            }
            LibraryCommand::Progress {
                identifier,
                pages,
                seconds,
                started,
                finished,
                edition,
            } => {
                if pages.is_none() && seconds.is_none() && finished.is_none() && started.is_none() {
                    return Err(CliError::usage(
                        "give at least one of --pages, --seconds, --started, --finished",
                    ));
                }
                let r = client.resolve_book(&identifier).await?;
                let before = entry_or_none(&client, r.id).await?;
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
                    pages,
                    seconds,
                    started_at: date_arg(started),
                    finished_at: date_arg(finished),
                    edition_id: edition,
                };
                if open_read.is_none() && update.started_at.is_none() {
                    update.started_at = Some(today());
                }
                let mut out = WriteResult {
                    action,
                    dry_run: ctx.dry_run,
                    planned: None,
                    before,
                    after: None,
                    read: None,
                };
                if ctx.dry_run {
                    out.planned = Some(serde_json::json!({
                        "read_id": open_read.as_ref().map(|x| x.id),
                        "pages": update.pages, "seconds": update.seconds,
                        "started_at": update.started_at, "finished_at": update.finished_at, "edition_id": update.edition_id,
                    }));
                } else {
                    let entry_id = match &out.before {
                        Some(b) => b.entry.id,
                        None => {
                            client
                                .library_add(r.id, ReadingStatus::CurrentlyReading, None)
                                .await?
                                .id
                        }
                    };
                    out.read = Some(match &open_read {
                        Some(existing) => client.read_update(existing, update).await?,
                        None => client.read_start(entry_id, update).await?,
                    });
                    out.after = entry_or_none(&client, r.id).await?;
                }
                ctx.emit(
                    &out,
                    serde_json::json!({ "resolved_by": r.resolved_by }),
                    WriteResult::line,
                );
            }
            LibraryCommand::Remove { identifier } => {
                let r = client.resolve_book(&identifier).await?;
                let before = client.library_entry(r.id).await?;
                let mut out = WriteResult {
                    action: "removed",
                    dry_run: ctx.dry_run,
                    planned: None,
                    before: Some(before),
                    after: None,
                    read: None,
                };
                if ctx.dry_run {
                    out.planned = Some(
                        serde_json::json!({ "delete_entry_id": out.before.as_ref().unwrap().entry.id }),
                    );
                } else {
                    client
                        .library_remove(out.before.as_ref().unwrap().entry.id)
                        .await?;
                }
                ctx.emit(
                    &out,
                    serde_json::json!({ "resolved_by": r.resolved_by }),
                    WriteResult::line,
                );
            }
            LibraryCommand::Show { identifier } => {
                let r = client.resolve_book(&identifier).await?;
                let d = client.library_entry(r.id).await?;
                ctx.emit(
                    &d,
                    serde_json::json!({ "resolved_by": r.resolved_by }),
                    |d| {
                        let mut lines = vec![format!(
                            "{} — {}{} ({} read{})",
                            d.entry.book.title,
                            d.entry.status.as_str(),
                            d.entry
                                .rating
                                .map(|r| format!(", rated {r}"))
                                .unwrap_or_default(),
                            d.entry.read_count,
                            if d.entry.read_count == 1 { "" } else { "s" }
                        )];
                        for read in &d.reads {
                            lines.push(format!(
                                "  read #{}: {} → {}  {}%",
                                read.id,
                                read.started_at.as_deref().unwrap_or("?"),
                                read.finished_at.as_deref().unwrap_or("…"),
                                read.progress.map(|p| p.round() as i64).unwrap_or(0)
                            ));
                        }
                        lines.join("\n")
                    },
                );
            }
        },
        Command::User {
            command: UserCommand::Show { username },
        } => {
            let u = client.user_by_username(&username).await?;
            ctx.emit(&u, none(), |u| {
                format!("{} (#{}) — {} books", u.username, u.id, u.books_count)
            });
        }
    }
    ctx.finish(&client);
    Ok(())
}
