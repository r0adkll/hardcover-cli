use serde::Serialize;
use std::process::ExitCode;

#[derive(Debug, Serialize)]
pub struct CliError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip)]
    pub exit: u8,
}

impl CliError {
    pub fn usage(msg: impl Into<String>) -> Self {
        Self { code: "usage_error", message: msg.into(), exit: 2 }
    }
    pub fn keychain(e: keyring::Error) -> Self {
        Self { code: "keychain_error", message: e.to_string(), exit: 1 }
    }
    pub fn auth_required() -> Self {
        Self {
            code: "auth_required",
            message: "no token found; run `hardcover login` or set HARDCOVER_TOKEN".into(),
            exit: 3,
        }
    }
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit)
    }
}

impl From<hardcover_api::Error> for CliError {
    fn from(e: hardcover_api::Error) -> Self {
        use hardcover_api::Error as E;
        match &e {
            E::InvalidToken => Self { code: "invalid_token", message: e.to_string(), exit: 3 },
            E::InsufficientScope(_) => Self { code: "insufficient_scope", message: e.to_string(), exit: 3 },
            E::RateLimited { .. } => Self { code: "rate_limited", message: e.to_string(), exit: 5 },
            E::NotFound(m) => Self { code: "not_found", message: m.clone(), exit: 4 },
            E::Network(err) => Self { code: "network_error", message: err.to_string(), exit: 6 },
            E::Upstream(m) => Self { code: "upstream_error", message: m.clone(), exit: 6 },
        }
    }
}

pub fn report(err: &CliError) {
    eprintln!("{}", serde_json::to_string(&serde_json::json!({ "error": err })).unwrap());
}
