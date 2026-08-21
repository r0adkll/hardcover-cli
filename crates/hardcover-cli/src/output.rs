use serde::Serialize;

pub const SCHEMA_VERSION: &str = "hardcover-cli/1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Auto,
    Json,
    Ndjson,
    Table,
    Plain,
}

impl Format {
    /// Resolve `Auto` based on whether stdout is a terminal.
    pub fn resolve(self) -> Format {
        match self {
            Format::Auto => {
                if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
                    Format::Plain
                } else {
                    Format::Json
                }
            }
            f => f,
        }
    }
}

#[derive(Serialize)]
struct Envelope<'a, T: Serialize> {
    schema: &'static str,
    data: &'a T,
    meta: serde_json::Value,
}

pub fn emit<T: Serialize + std::fmt::Debug>(format: Format, value: &T, plain: impl Fn(&T) -> String) {
    match format.resolve() {
        Format::Json => {
            let env = Envelope { schema: SCHEMA_VERSION, data: value, meta: serde_json::json!({}) };
            println!("{}", serde_json::to_string_pretty(&env).unwrap());
        }
        Format::Ndjson => println!("{}", serde_json::to_string(value).unwrap()),
        Format::Table | Format::Plain => println!("{}", plain(value)),
        Format::Auto => unreachable!(),
    }
}
