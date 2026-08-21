//! Non-secret preferences in `config.toml`. The token never lives here (see credentials.rs).
//!
//! Location: `$HARDCOVER_CONFIG_DIR/config.toml` if set, otherwise the platform config dir
//! (`~/.config/hardcover` on Linux, `~/Library/Application Support/hardcover` on macOS).
use crate::error::CliError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default `--format` when none is given on the command line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Username verified at the last `login`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

pub fn path() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("HARDCOVER_CONFIG_DIR") {
        return Some(PathBuf::from(dir).join("config.toml"));
    }
    directories::ProjectDirs::from("", "", "hardcover").map(|d| d.config_dir().join("config.toml"))
}

/// Missing or unreadable config is not an error: defaults apply.
pub fn load() -> Config {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(cfg: &Config) -> Result<(), CliError> {
    let Some(p) = path() else { return Ok(()) };
    let io = |e: std::io::Error| CliError::usage(format!("cannot write {}: {e}", p.display()));
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    std::fs::write(&p, toml::to_string(cfg).expect("config serialises")).map_err(io)
}
