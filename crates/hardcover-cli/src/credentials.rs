//! Token resolution: `--token` flag > `HARDCOVER_TOKEN` env > OS keychain.
use crate::error::CliError;

const SERVICE: &str = "hardcover-cli";
const ACCOUNT: &str = "token";

/// Select the mock in-memory store when `HARDCOVER_KEYRING=mock` (tests/CI).
pub fn init() {
    if std::env::var("HARDCOVER_KEYRING").as_deref() == Ok("mock") {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    }
}

fn entry() -> Result<keyring::Entry, CliError> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(CliError::keychain)
}

pub fn stored() -> Result<Option<String>, CliError> {
    match entry()?.get_password() {
        Ok(t) => Ok(Some(t)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(CliError::keychain(e)),
    }
}

pub fn store(token: &str) -> Result<(), CliError> {
    entry()?.set_password(token).map_err(CliError::keychain)
}

pub fn clear() -> Result<(), CliError> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(CliError::keychain(e)),
    }
}

pub fn resolve(flag_or_env: Option<String>) -> Result<String, CliError> {
    if let Some(t) = flag_or_env {
        return Ok(t);
    }
    stored()?.ok_or_else(CliError::auth_required)
}

/// Read a token for `login`: prompt on a TTY, otherwise take the first line of stdin.
pub fn read_login_token() -> Result<String, CliError> {
    use std::io::{BufRead, IsTerminal, Write};
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        eprint!("Paste your Hardcover API token (hardcover.app/account/api): ");
        std::io::stderr().flush().ok();
    }
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| CliError::usage(e.to_string()))?;
    let token = line.trim().to_string();
    if token.is_empty() {
        return Err(CliError::usage("no token provided on stdin"));
    }
    Ok(token)
}
