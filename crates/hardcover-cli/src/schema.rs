//! `hardcover schema`: a machine-readable description of this CLI, derived from the
//! clap tree so it can never drift from `--help`.
use crate::error::CATALOGUE;
use crate::output::SCHEMA_VERSION;
use clap::CommandFactory;
use serde::Serialize;

#[derive(Serialize)]
pub struct Description {
    pub version: &'static str,
    pub output_schema: &'static str,
    pub formats: Vec<&'static str>,
    pub commands: Vec<CommandDesc>,
    pub error_codes: Vec<ErrorDesc>,
    pub notes: Vec<&'static str>,
}

#[derive(Serialize)]
pub struct CommandDesc {
    pub path: String,
    pub about: String,
    pub requires_auth: bool,
    pub args: Vec<ArgDesc>,
}

#[derive(Serialize)]
pub struct ArgDesc {
    pub name: String,
    pub kind: &'static str,
    pub required: bool,
    pub help: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
}

#[derive(Serialize)]
pub struct ErrorDesc {
    pub code: &'static str,
    pub exit: u8,
    pub meaning: &'static str,
}

const NO_AUTH: &[&str] = &["login", "logout", "schema"];

pub fn describe() -> Description {
    let root = crate::cli::Cli::command();
    let mut commands = Vec::new();
    walk(&root, &[], &mut commands);
    Description {
        version: env!("CARGO_PKG_VERSION"),
        output_schema: SCHEMA_VERSION,
        formats: vec!["auto", "json", "ndjson", "table", "plain"],
        commands,
        error_codes: CATALOGUE.iter().map(|(code, exit, meaning)| ErrorDesc { code, exit: *exit, meaning }).collect(),
        notes: vec![
            "JSON output is an envelope {schema, data, meta}; ndjson emits bare objects, one per line.",
            "Errors are written to stderr as {\"error\": {code, message, ...}} with a non-zero exit code.",
            "Identifiers accept numeric ids or slugs; books also accept ISBN-10/13. Prefix id:/slug:/isbn: to force a form.",
            "Collections take --limit/--offset; --all pages until exhausted or --max-rows, setting meta.truncated.",
            "--raw replaces the output with the upstream Hardcover API payload(s).",
        ],
    }
}

fn walk(cmd: &clap::Command, prefix: &[String], out: &mut Vec<CommandDesc>) {
    let subs: Vec<_> = cmd.get_subcommands().filter(|c| c.get_name() != "help").collect();
    if subs.is_empty() {
        let path = prefix.join(" ");
        out.push(CommandDesc {
            path: path.clone(),
            about: cmd.get_about().map(|a| a.to_string()).unwrap_or_default(),
            requires_auth: !NO_AUTH.contains(&path.as_str()),
            args: cmd
                .get_arguments()
                .filter(|a| !a.is_global_set() && !matches!(a.get_id().as_str(), "help" | "version"))
                .map(arg_desc)
                .collect(),
        });
        return;
    }
    for sub in subs {
        let mut p = prefix.to_vec();
        p.push(sub.get_name().to_string());
        walk(sub, &p, out);
    }
}

fn arg_desc(a: &clap::Arg) -> ArgDesc {
    let name = a.get_long().map(|l| format!("--{l}")).unwrap_or_else(|| a.get_id().to_string());
    ArgDesc {
        name,
        kind: if a.is_positional() {
            "positional"
        } else if matches!(a.get_action(), clap::ArgAction::SetTrue | clap::ArgAction::SetFalse) {
            "flag"
        } else {
            "option"
        },
        required: a.is_required_set(),
        help: a.get_help().map(|h| h.to_string()).unwrap_or_default(),
        default: a.get_default_values().first().map(|v| v.to_string_lossy().into_owned()),
        values: a.get_possible_values().iter().map(|v| v.get_name().to_string()).collect(),
    }
}

pub fn plain() -> String {
    let d = describe();
    let mut lines = vec![format!("hardcover-cli {} (output schema {})", d.version, d.output_schema), String::new()];
    for c in &d.commands {
        let args: Vec<String> = c
            .args
            .iter()
            .map(|a| if a.kind == "positional" { format!("<{}>", a.name) } else { a.name.clone() })
            .collect();
        lines.push(format!("  {:<22} {}  {}", c.path, args.join(" "), c.about));
    }
    lines.push(String::new());
    lines.push("Error codes:".into());
    for e in &d.error_codes {
        lines.push(format!("  {:<20} exit {}  {}", e.code, e.exit, e.meaning));
    }
    lines.join("\n")
}
