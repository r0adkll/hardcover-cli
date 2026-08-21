use crate::cli::*;
use crate::credentials;
use crate::error::CliError;
use crate::output::{self, Format};
use crate::paging::collect;
use hardcover_api::model::{BookSummary, Resolved, User};
use hardcover_api::{Client, RetryPolicy};
use serde::Serialize;

struct Ctx<'a> {
    format: Format,
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
