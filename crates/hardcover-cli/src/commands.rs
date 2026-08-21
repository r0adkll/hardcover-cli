use crate::cli::*;
use crate::credentials;
use crate::error::CliError;
use crate::ops::{self, WriteResult};
use crate::output::{self, Format};
use crate::paging::collect;
use hardcover_api::model::{BookSummary, LibraryFilter, Resolved, User};
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
        Command::Agent { command } => {
            run_agent(&ctx, command, cli.api_url.clone()).await?;
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
        Command::Schema | Command::Login | Command::Logout | Command::Agent { .. } => {
            unreachable!()
        }
        Command::Mcp {
            command: McpCommand::Serve,
        } => {
            crate::mcp::serve(client.clone()).await?;
        }
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
                let (out, r) = ops::set_status(&client, &identifier, status, ctx.dry_run).await?;
                ctx.emit(
                    &out,
                    serde_json::json!({ "resolved_by": r.resolved_by }),
                    WriteResult::line,
                );
            }
            LibraryCommand::Rate { identifier, rating } => {
                let (out, r) = ops::rate(&client, &identifier, rating, ctx.dry_run).await?;
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
                let p = ops::Progress {
                    pages,
                    seconds,
                    started,
                    finished,
                    edition,
                };
                let (out, r) = ops::progress(&client, &identifier, p, ctx.dry_run).await?;
                ctx.emit(
                    &out,
                    serde_json::json!({ "resolved_by": r.resolved_by }),
                    WriteResult::line,
                );
            }
            LibraryCommand::Remove { identifier } => {
                let (out, r) = ops::remove(&client, &identifier, ctx.dry_run).await?;
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

async fn run_agent(
    ctx: &Ctx<'_>,
    command: AgentCommand,
    api_url: Option<String>,
) -> Result<(), CliError> {
    use crate::agent::{self, SetupOptions};
    match command {
        AgentCommand::Skills => {
            let list: Vec<_> = crate::skills::SKILLS.iter().map(|s| s.info()).collect();
            ctx.emit_list(&list, none(), |s| {
                format!("{:<14} {}", s.name, s.description)
            });
        }
        AgentCommand::Status => {
            let st = agent::status();
            ctx.emit_list(&st, none(), |h| {
                format!(
                    "{:<15} {:<9} {:<12} skills {}/{}  {}",
                    h.host.name(),
                    if h.detected { "detected" } else { "-" },
                    if h.configured { "configured" } else { "-" },
                    h.skills_installed,
                    h.skills_available,
                    h.config_path.display()
                )
            });
        }
        AgentCommand::Remove { host, scope } => {
            let r = agent::remove(host, scope, ctx.dry_run)?;
            ctx.emit(&r, none(), setup_line);
        }
        AgentCommand::Setup {
            host,
            scope,
            command,
            no_skills,
        } => {
            let host = match host {
                Some(h) => h,
                None => choose_host()?,
            };
            let opts = SetupOptions {
                scope,
                command: command.unwrap_or_else(agent::default_command),
                install_skills: !no_skills,
                dry_run: ctx.dry_run,
            };
            let mut r = agent::setup(host, &opts)?;
            // Make sure the server will actually be able to authenticate. Dry runs don't
            // touch the keychain (it can prompt on macOS); env var is checked first because it's free.
            if !ctx.dry_run {
                if std::env::var_os("HARDCOVER_TOKEN").is_some() {
                    r.notes.push("Token comes from HARDCOVER_TOKEN in this shell; GUI hosts won't see it — run `hardcover login` to store it in the keychain.".into());
                } else if credentials::stored()?.is_none() {
                    if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                        let token = credentials::read_login_token()?;
                        let user = ctx.client(token.clone()).me().await?;
                        credentials::store(&token)?;
                        r.notes.push(format!(
                            "Logged in as {} and stored the token in the keychain.",
                            user.username
                        ));
                    } else {
                        r.notes.push(
                            "No token stored; run `hardcover login` before using the server."
                                .into(),
                        );
                    }
                }
            }
            let _ = api_url;
            ctx.emit(&r, none(), setup_line);
        }
    }
    Ok(())
}

fn choose_host() -> Result<crate::agent::Host, CliError> {
    use std::io::{BufRead, IsTerminal, Write};
    let detected = crate::agent::detected_hosts();
    let names: Vec<&str> = detected.iter().map(|h| h.name()).collect();
    if !std::io::stdin().is_terminal() {
        return Err(CliError::usage(format!(
            "no host given; detected: {}. Run `hardcover agent setup <host>`.",
            if names.is_empty() {
                "none".to_string()
            } else {
                names.join(", ")
            }
        )));
    }
    if detected.is_empty() {
        return Err(CliError::usage("no supported agent hosts detected; pass one explicitly (claude-code, claude-desktop, codex, cursor, gemini, vscode, windsurf)"));
    }
    eprintln!("Detected agent hosts:");
    for (i, h) in detected.iter().enumerate() {
        eprintln!("  {}) {}", i + 1, h.name());
    }
    eprint!("Set up which? [1-{}] ", detected.len());
    std::io::stderr().flush().ok();
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| CliError::usage(e.to_string()))?;
    let n: usize = line
        .trim()
        .parse()
        .map_err(|_| CliError::usage("not a number"))?;
    detected
        .get(n.wrapping_sub(1))
        .copied()
        .ok_or_else(|| CliError::usage("out of range"))
}

fn setup_line(r: &crate::agent::SetupResult) -> String {
    let lc = |d: &dyn std::fmt::Debug| format!("{d:?}").to_lowercase();
    let mut lines = vec![format!(
        "{}{} ({} scope): {} {}",
        if r.dry_run { "[dry-run] " } else { "" },
        r.host.name(),
        lc(&r.scope),
        lc(&r.action),
        r.config_path.display()
    )];
    for s in &r.skills {
        lines.push(format!(
            "  skill {:<14} {:<9} {}",
            s.name,
            lc(&s.action),
            s.path.display()
        ));
    }
    for n in &r.notes {
        lines.push(format!("  note: {n}"));
    }
    lines.join("\n")
}
