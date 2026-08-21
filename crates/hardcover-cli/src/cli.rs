//! The clap command tree. One noun per domain term (see CONTEXT.md).
use crate::output::Format;
use crate::paging::PageArgs;
use clap::{Parser, Subcommand};
use hardcover_api::model::{BookIdentifier, Identifier, ReadingStatus, SearchType};

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

    /// Emit the upstream API payload(s) instead of the CLI's own output shape.
    #[arg(long, global = true)]
    pub raw: bool,

    /// Do not retry rate-limited requests; fail immediately with `rate_limited`.
    #[arg(long, global = true)]
    pub no_retry: bool,

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
    /// Editions: specific published forms of a book.
    Edition {
        #[command(subcommand)]
        command: EditionCommand,
    },
    /// Prompts: community questions answered with books.
    Prompt {
        #[command(subcommand)]
        command: PromptCommand,
    },
    /// Users: public profiles.
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    /// Your library: the books you've shelved, with status, rating and reads.
    Library {
        #[command(subcommand)]
        command: LibraryCommand,
    },
    /// Describe this CLI for programmatic consumers: commands, arguments, formats, error codes.
    Schema,
}

#[derive(Subcommand)]
pub enum EditionCommand {
    /// Show one edition by id.
    Show { id: i64 },
}

#[derive(Subcommand)]
pub enum PromptCommand {
    /// Show one prompt by id or slug.
    Show { identifier: Identifier },
}

#[derive(Subcommand)]
pub enum UserCommand {
    /// Show a user's public profile by username.
    Show { username: String },
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
    /// Show an author by id or slug.
    Show { identifier: Identifier },
    /// List an author's books, most-shelved first.
    Books {
        identifier: Identifier,
        #[command(flatten)]
        page: PageArgs,
    },
}

#[derive(Subcommand)]
pub enum SeriesCommand {
    /// Show a series by id or slug.
    Show { identifier: Identifier },
    /// List the books in a series in position order.
    Books {
        identifier: Identifier,
        #[command(flatten)]
        page: PageArgs,
    },
}

#[derive(Subcommand)]
pub enum ListCommand {
    /// Show a list by id or slug.
    Show { identifier: Identifier },
    /// List the books in a list in position order.
    Books {
        identifier: Identifier,
        #[command(flatten)]
        page: PageArgs,
    },
}

#[derive(Subcommand)]
pub enum LibraryCommand {
    /// List your library, most recently updated first.
    List {
        /// Only this reading status: want_to_read, currently_reading, read, paused, did_not_finish, ignored.
        #[arg(long)]
        status: Option<ReadingStatus>,
        /// Only books you own.
        #[arg(long)]
        owned: bool,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Your entry for one book (by id, slug, or ISBN), including reads and review.
    Show { identifier: BookIdentifier },
}
