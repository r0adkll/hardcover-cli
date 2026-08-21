mod credentials;
mod error;
mod output;

use clap::{Parser, Subcommand};
use error::CliError;
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
    /// Books: works independent of any particular edition.
    Book {
        #[command(subcommand)]
        command: BookCommand,
    },
}

#[derive(Subcommand)]
enum BookCommand {
    /// Show one book by id.
    Show { id: i64 },
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
            output::emit(cli.format, &user, |u| format!("Logged in as {}", user_line(u)));
            return Ok(());
        }
        Command::Logout => {
            credentials::clear()?;
            output::emit(cli.format, &serde_json::json!({ "logged_out": true }), |_| "Logged out".into());
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
            output::emit(cli.format, &user, user_line);
        }
        Command::Book { command: BookCommand::Show { id } } => {
            let book = client.book(id).await?;
            output::emit(cli.format, &book, |b| {
                let by = b.contributors.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", ");
                format!("{} — {} (#{}, {})", b.title, by, b.id, b.slug)
            });
        }
    }
    Ok(())
}
