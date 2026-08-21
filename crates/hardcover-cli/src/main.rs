mod agent;
mod cli;
mod commands;
mod config;
mod credentials;
mod error;
mod mcp;
mod ops;
mod output;
mod paging;
mod schema;
mod skills;

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
