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
    let cli = Cli::parse();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error::report(&e);
            e.exit_code()
        }
    }
}

async fn run(cli: Cli) -> Result<(), CliError> {
    let token = cli.token.ok_or_else(CliError::auth_required)?;
    let mut builder = Client::builder(token);
    if let Some(url) = cli.api_url {
        builder = builder.base_url(url);
    }
    let client = builder.build();

    match cli.command {
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
