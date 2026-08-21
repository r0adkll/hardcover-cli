mod credentials;
mod error;
mod output;

use clap::{Parser, Subcommand};
use error::CliError;
use hardcover_api::model::{BookIdentifier, SearchType};
use hardcover_api::Client;
use output::Format;
use std::process::ExitCode;

/// Command-line client for Hardcover.app, built for agents first.
///
/// Use of Hardcover data is subject to https://docs.hardcover.app/api/getting-started/ terms.
#[derive(Parser)]
#[command(name = "hardcover", version, about)]
struct Cli {
    /// Output format. `auto` picks plain text on a terminal, JSON otherwise.
    #[arg(long, global = true, value_enum, default_value = "auto")]
    format: Format,

    /// Personal access token. Overrides HARDCOVER_TOKEN and the keychain.
    #[arg(long, global = true, env = "HARDCOVER_TOKEN", hide_env_values = true)]
    token: Option<String>,

    /// API base URL (testing only).
    #[arg(long, global = true, env = "HARDCOVER_API_URL", hide = true)]
    api_url: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verify a token and store it in the OS keychain. Reads the token from stdin when not a terminal.
    Login,
    /// Remove the stored token from the OS keychain.
    Logout,
    /// Show the authenticated user.
    Whoami,
    /// Full-text search over one entity type.
    Search {
        query: String,
        /// Entity type: book, author, series, character, list, prompt, publisher, user.
        #[arg(long = "type", default_value = "book")]
        query_type: SearchType,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: i64,
        #[arg(long, default_value_t = 25)]
        per_page: i64,
    },
    /// Books: works independent of any particular edition.
    Book {
        #[command(subcommand)]
        command: BookCommand,
    },
}

#[derive(Subcommand)]
enum BookCommand {
    /// Show one book by id, slug, or ISBN (prefix with id:/slug:/isbn: to force a form).
    Show { identifier: BookIdentifier },
}

#[tokio::main]
async fn main() -> ExitCode {
    credentials::init();
    let cli = Cli::parse();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error::report(&e);
            e.exit_code()
        }
    }
}

fn make_client(token: String, api_url: &Option<String>) -> Client {
    let mut builder = Client::builder(token);
    if let Some(url) = api_url {
        builder = builder.base_url(url);
    }
    builder.build()
}

fn user_line(u: &hardcover_api::model::User) -> String {
    format!("{} (#{})", u.username, u.id)
}

async fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Login => {
            let token = credentials::read_login_token()?;
            let user = make_client(token.clone(), &cli.api_url).me().await?;
            credentials::store(&token)?;
            output::emit(cli.format, &user, serde_json::json!({}), |u| format!("Logged in as {}", user_line(u)));
            return Ok(());
        }
        Command::Logout => {
            credentials::clear()?;
            output::emit(cli.format, &serde_json::json!({ "logged_out": true }), serde_json::json!({}), |_| "Logged out".into());
            return Ok(());
        }
        _ => {}
    }

    let token = credentials::resolve(cli.token)?;
    let client = make_client(token, &cli.api_url);

    match cli.command {
        Command::Login | Command::Logout => unreachable!(),
        Command::Search { query, query_type, page, per_page } => {
            let r = client.search(&query, query_type, page, per_page).await?;
            let meta = serde_json::json!({ "page": r.page, "per_page": r.per_page, "found": r.found });
            output::emit(cli.format, &r, meta, |r| {
                let mut lines = vec![format!("{} results for \"{}\" ({})", r.found, r.query, r.query_type.as_str())];
                for h in &r.hits {
                    lines.push(format!("  #{:<8} {}  [{}]", h.id, h.label, h.slug.as_deref().unwrap_or("-")));
                }
                lines.join("\n")
            });
        }
        Command::Whoami => {
            let user = client.me().await?;
            output::emit(cli.format, &user, serde_json::json!({}), user_line);
        }
        Command::Book { command: BookCommand::Show { identifier } } => {
            let resolved = client.resolve_book(&identifier).await?;
            let book = client.book(resolved.id).await?;
            let meta = serde_json::json!({ "resolved_by": resolved.resolved_by });
            output::emit(cli.format, &book, meta, |b| {
                let by = b.contributors.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", ");
                format!("{} — {} (#{}, {})", b.title, by, b.id, b.slug)
            });
        }
    }
    Ok(())
}
