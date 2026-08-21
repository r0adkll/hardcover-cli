use serde::Serialize;
use std::process::ExitCode;

/// Every stable error code the CLI can emit, with its exit code. Surfaced by `hardcover schema`.
pub const CATALOGUE: &[(&str, u8, &str)] = &[
    ("usage_error", 2, "Invalid arguments or input"),
    ("auth_required", 3, "No token available; run `hardcover login` or set HARDCOVER_TOKEN"),
    ("invalid_token", 3, "Token rejected or expired; run `hardcover login` again"),
    ("insufficient_scope", 3, "Token lacks a scope the operation needs"),
    ("not_found", 4, "No entity matched the identifier"),
    ("rate_limited", 5, "Upstream rate limit hit; `retry_after_secs` says when to retry"),
    ("network_error", 6, "Could not reach the API"),
    ("upstream_error", 6, "The API returned an error"),
    ("keychain_error", 1, "The OS keychain could not be used"),
];

#[derive(Debug, Serialize)]
pub struct CliError {
    pub code: &'static str,
    pub message: String,
    #[serde(flatten)]
    pub details: serde_json::Map<String, serde_json::Value>,
    #[serde(skip)]
    pub exit: u8,
}

fn exit_for(code: &str) -> u8 {
    CATALOGUE.iter().find(|(c, ..)| *c == code).map(|(_, e, _)| *e).unwrap_or(1)
}

impl CliError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), details: Default::default(), exit: exit_for(code) }
    }
    pub fn usage(msg: impl Into<String>) -> Self {
        Self::new("usage_error", msg)
    }
    pub fn keychain(e: keyring::Error) -> Self {
        Self::new("keychain_error", e.to_string())
    }
    pub fn auth_required() -> Self {
        Self::new("auth_required", "no token found; run `hardcover login` or set HARDCOVER_TOKEN")
    }
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit)
    }
}

impl From<hardcover_api::Error> for CliError {
    fn from(e: hardcover_api::Error) -> Self {
        use hardcover_api::Error as E;
        match &e {
            E::InvalidToken => Self::new("invalid_token", e.to_string()),
            E::InsufficientScope(scope) => {
                let mut err = Self::new("insufficient_scope", e.to_string());
                err.details.insert("scope".into(), scope.clone().into());
                err
            }
            E::RateLimited { retry_after_secs } => {
                let mut err = Self::new("rate_limited", e.to_string());
                err.details.insert("retry_after_secs".into(), (*retry_after_secs).into());
                err
            }
            E::NotFound(m) => Self::new("not_found", m.clone()),
            E::Network(err) => Self::new("network_error", err.to_string()),
            E::Upstream(m) => Self::new("upstream_error", m.clone()),
        }
    }
}

pub fn report(err: &CliError) {
    eprintln!("{}", serde_json::to_string(&serde_json::json!({ "error": err })).unwrap());
}
