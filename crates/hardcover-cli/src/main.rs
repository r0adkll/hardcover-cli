mod cli;
mod commands;
mod credentials;
mod error;
mod output;
mod paging;

use clap::Parser;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    credentials::init();
    let cli = cli::Cli::parse();
    match commands::run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error::report(&e);
            e.exit_code()
        }
    }
}
