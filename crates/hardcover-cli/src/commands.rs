use crate::cli::*;
use crate::error::CliError;
use crate::output::{emit, emit_list, Format};
use crate::paging::collect;
use crate::credentials;
use hardcover_api::model::{BookSummary, Resolved, User};
use hardcover_api::Client;

fn make_client(token: String, api_url: &Option<String>) -> Client {
    let mut builder = Client::builder(token);
    if let Some(url) = api_url {
        builder = builder.base_url(url);
    }
    builder.build()
}

fn user_line(u: &User) -> String {
    format!("{} (#{})", u.username, u.id)
}

fn resolved_meta(r: &Resolved) -> serde_json::Value {
    serde_json::json!({ "resolved_by": r.resolved_by })
}

fn with_resolved(mut meta: serde_json::Value, r: &Resolved) -> serde_json::Value {
    meta["resolved_by"] = serde_json::json!(r.resolved_by);
    meta
}

fn summary_line(b: &BookSummary) -> String {
    format!("#{:<9} {}{}", b.id, b.title, b.release_year.map(|y| format!(" ({y})")).unwrap_or_default())
}

pub async fn run(cli: Cli) -> Result<(), CliError> {
    let format: Format = cli.format;
    let none = || serde_json::json!({});

    match cli.command {
        Command::Login => {
            let token = credentials::read_login_token()?;
            let user = make_client(token.clone(), &cli.api_url).me().await?;
            credentials::store(&token)?;
            emit(format, &user, none(), |u| format!("Logged in as {}", user_line(u)));
            return Ok(());
        }
        Command::Logout => {
            credentials::clear()?;
            emit(format, &serde_json::json!({ "logged_out": true }), none(), |_| "Logged out".into());
            return Ok(());
        }
        _ => {}
    }

    let token = credentials::resolve(cli.token)?;
    let client = make_client(token, &cli.api_url);

    match cli.command {
        Command::Login | Command::Logout => unreachable!(),
        Command::Whoami => {
            let user = client.me().await?;
            emit(format, &user, none(), user_line);
        }
        Command::Search { query, query_type, page, per_page } => {
            let r = client.search(&query, query_type, page, per_page).await?;
            let meta = serde_json::json!({ "page": r.page, "per_page": r.per_page, "found": r.found });
            emit(format, &r, meta, |r| {
                let mut lines = vec![format!("{} results for \"{}\" ({})", r.found, r.query, r.query_type.as_str())];
                for h in &r.hits {
                    lines.push(format!("  #{:<8} {}  [{}]", h.id, h.label, h.slug.as_deref().unwrap_or("-")));
                }
                lines.join("\n")
            });
        }
        Command::Book { command } => match command {
            BookCommand::Show { identifier } => {
                let resolved = client.resolve_book(&identifier).await?;
                let book = client.book(resolved.id).await?;
                let meta = serde_json::json!({ "resolved_by": resolved.resolved_by });
                emit(format, &book, meta, |b| {
                    let by = b.contributors.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", ");
                    format!("{} — {} (#{}, {})", b.title, by, b.id, b.slug)
                });
            }
            BookCommand::Editions { identifier, page } => {
                let resolved = client.resolve_book(&identifier).await?;
                let c = collect(&page, |p| client.book_editions(resolved.id, p)).await?;
                let mut meta = c.meta();
                meta["resolved_by"] = serde_json::json!(resolved.resolved_by);
                emit_list(format, &c.items, meta, |e| {
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
                emit(format, &a, resolved_meta(&r), |a| format!("{} (#{}, {}) — {} books", a.name, a.id, a.slug, a.books_count));
            }
            AuthorCommand::Books { identifier, page } => {
                let r = client.resolve_author(&identifier).await?;
                let c = collect(&page, |p| client.author_books(r.id, p)).await?;
                emit_list(format, &c.items, with_resolved(c.meta(), &r), summary_line);
            }
        },
        Command::Series { command } => match command {
            SeriesCommand::Show { identifier } => {
                let r = client.resolve_series(&identifier).await?;
                let x = client.series(r.id).await?;
                emit(format, &x, resolved_meta(&r), |x| format!("{} (#{}, {}) — {} books", x.name, x.id, x.slug, x.books_count));
            }
            SeriesCommand::Books { identifier, page } => {
                let r = client.resolve_series(&identifier).await?;
                let c = collect(&page, |p| client.series_books(r.id, p)).await?;
                emit_list(format, &c.items, with_resolved(c.meta(), &r), |e| {
                    format!("{:>5}  {}", e.position.map(|p| p.to_string()).unwrap_or_default(), summary_line(&e.book))
                });
            }
        },
        Command::List { command } => match command {
            ListCommand::Show { identifier } => {
                let r = client.resolve_list(&identifier).await?;
                let l = client.list(r.id).await?;
                emit(format, &l, resolved_meta(&r), |l| format!("{} (#{}) by {} — {} books", l.name, l.id, l.owner.username, l.books_count));
            }
            ListCommand::Books { identifier, page } => {
                let r = client.resolve_list(&identifier).await?;
                let c = collect(&page, |p| client.list_books(r.id, p)).await?;
                emit_list(format, &c.items, with_resolved(c.meta(), &r), |e| {
                    format!("{:>5}  {}", e.position.map(|p| p.to_string()).unwrap_or_default(), summary_line(&e.book))
                });
            }
        },
        Command::Edition { command: EditionCommand::Show { id } } => {
            let e = client.edition(id).await?;
            emit(format, &e, none(), |e| {
                format!("{} (#{}) {} {}", e.title, e.id, e.isbn_13.as_deref().or(e.isbn_10.as_deref()).unwrap_or("-"), e.edition_format.as_deref().unwrap_or(""))
            });
        }
        Command::Prompt { command: PromptCommand::Show { identifier } } => {
            let r = client.resolve_prompt(&identifier).await?;
            let p = client.prompt(r.id).await?;
            emit(format, &p, resolved_meta(&r), |p| format!("{} (#{}, {}) — {} answers", p.question, p.id, p.slug, p.answers_count));
        }
        Command::User { command: UserCommand::Show { username } } => {
            let u = client.user_by_username(&username).await?;
            emit(format, &u, none(), |u| format!("{} (#{}) — {} books", u.username, u.id, u.books_count));
        }
    }
    Ok(())
}
