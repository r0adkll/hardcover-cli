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

/// Emit a single entity.
pub fn emit<T: Serialize>(
    format: Format,
    value: &T,
    meta: serde_json::Value,
    plain: impl Fn(&T) -> String,
) {
    match format.resolve() {
        Format::Json => print_envelope(value, meta),
        Format::Ndjson => println!("{}", serde_json::to_string(value).unwrap()),
        Format::Table => println!(
            "{}",
            crate::table::render_entity(&serde_json::to_value(value).unwrap())
        ),
        Format::Plain => println!("{}", plain(value)),
        Format::Auto => unreachable!(),
    }
}

/// Emit a collection: JSON wraps the array in the envelope, NDJSON streams one row per line.
pub fn emit_list<T: Serialize>(
    format: Format,
    items: &[T],
    meta: serde_json::Value,
    line: impl Fn(&T) -> String,
) {
    match format.resolve() {
        Format::Json => print_envelope(&items, meta),
        Format::Ndjson => {
            for item in items {
                println!("{}", serde_json::to_string(item).unwrap());
            }
        }
        Format::Table => {
            let rows: Vec<serde_json::Value> = items
                .iter()
                .map(|i| serde_json::to_value(i).unwrap())
                .collect();
            println!("{}", crate::table::render_rows(&rows));
        }
        Format::Plain => {
            for item in items {
                println!("{}", line(item));
            }
        }
        Format::Auto => unreachable!(),
    }
}

fn print_envelope<T: Serialize>(data: &T, meta: serde_json::Value) {
    let env = Envelope {
        schema: SCHEMA_VERSION,
        data,
        meta,
    };
    println!("{}", serde_json::to_string_pretty(&env).unwrap());
}
