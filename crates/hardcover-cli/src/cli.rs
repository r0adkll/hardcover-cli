//! The clap command tree. One noun per domain term (see CONTEXT.md).
use crate::output::Format;
use crate::paging::PageArgs;
use clap::{Parser, Subcommand};
use hardcover_api::model::{BookIdentifier, SearchType};

/// Command-line client for Hardcover.app, built for agents first.
///
/// Use of Hardcover data is subject to Hardcover's API terms:
/// https://docs.hardcover.app/api/getting-started/
#[derive(Parser)]
#[command(name = "hardcover", version, about, long_about)]
pub struct Cli {
    /// Output format. `auto` picks plain text on a terminal, JSON otherwise.
    #[arg(long, global = true, value_enum, default_value = "auto")]
    pub format: Format,

    /// Personal access token. Overrides HARDCOVER_TOKEN and the keychain.
    #[arg(long, global = true, env = "HARDCOVER_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    /// API base URL (testing only).
    #[arg(long, global = true, env = "HARDCOVER_API_URL", hide = true)]
    pub api_url: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
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
    /// Authors and other contributors.
    Author {
        #[command(subcommand)]
        command: AuthorCommand,
    },
    /// Series: ordered groupings of books.
    Series {
        #[command(subcommand)]
        command: SeriesCommand,
    },
    /// Lists: user-curated collections of books.
    List {
        #[command(subcommand)]
        command: ListCommand,
    },
}

#[derive(Subcommand)]
pub enum BookCommand {
    /// Show one book by id, slug, or ISBN (prefix with id:/slug:/isbn: to force a form).
    Show { identifier: BookIdentifier },
    /// List a book's editions.
    Editions {
        identifier: BookIdentifier,
        #[command(flatten)]
        page: PageArgs,
    },
}

#[derive(Subcommand)]
pub enum AuthorCommand {
    /// List an author's books, most-shelved first.
    Books {
        id: i64,
        #[command(flatten)]
        page: PageArgs,
    },
}

#[derive(Subcommand)]
pub enum SeriesCommand {
    /// List the books in a series in position order.
    Books {
        id: i64,
        #[command(flatten)]
        page: PageArgs,
    },
}

#[derive(Subcommand)]
pub enum ListCommand {
    /// List the books in a list in position order.
    Books {
        id: i64,
        #[command(flatten)]
        page: PageArgs,
    },
}
