//! `hardcover agent …`: register the MCP server with agent hosts and install shipped skills.
//!
//! Every host except Codex (TOML) and VS Code (`servers` key) uses the same
//! `mcpServers.<name>.{command,args,env}` JSON object, so one merge routine plus two
//! special cases covers them all. Edits are merges: only the `hardcover` entry is touched,
//! everything else in the file is preserved, and a `.bak` is written before the first change.
use crate::error::CliError;
use crate::skills::{Skill, SKILLS};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const SERVER_NAME: &str = "hardcover";

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Host {
    ClaudeCode,
    ClaudeDesktop,
    Codex,
    Cursor,
    Gemini,
    Vscode,
    Windsurf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    User,
    Project,
}

enum Format {
    /// `{"mcpServers": {name: {...}}}` — Claude Code/Desktop, Cursor, Gemini, Windsurf.
    McpServersJson,
    /// `{"servers": {name: {"type":"stdio", ...}}}` — VS Code.
    VscodeJson,
    /// `[mcp_servers.name]` tables — Codex.
    CodexToml,
}

/// How skills are delivered to a host, if at all.
enum SkillKind {
    /// agentskills.io `SKILL.md` in `<dir>/<name>/SKILL.md` (Claude Code, Codex).
    SkillMd,
    /// Cursor `.mdc` rules in `<dir>/hardcover-<name>.mdc` (project scope only).
    CursorRule,
    None,
}

impl Host {
    pub const ALL: [Host; 7] = [
        Host::ClaudeCode,
        Host::ClaudeDesktop,
        Host::Codex,
        Host::Cursor,
        Host::Gemini,
        Host::Vscode,
        Host::Windsurf,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Host::ClaudeCode => "claude-code",
            Host::ClaudeDesktop => "claude-desktop",
            Host::Codex => "codex",
            Host::Cursor => "cursor",
            Host::Gemini => "gemini",
            Host::Vscode => "vscode",
            Host::Windsurf => "windsurf",
        }
    }

    fn format(self) -> Format {
        match self {
            Host::Codex => Format::CodexToml,
            Host::Vscode => Format::VscodeJson,
            _ => Format::McpServersJson,
        }
    }

    pub fn supports_scope(self, scope: Scope) -> bool {
        match scope {
            Scope::User => !matches!(self, Host::Vscode),
            Scope::Project => matches!(
                self,
                Host::ClaudeCode | Host::Codex | Host::Cursor | Host::Gemini | Host::Vscode
            ),
        }
    }

    /// Where the MCP server config lives for this host and scope.
    pub fn config_path(self, scope: Scope) -> Result<PathBuf, CliError> {
        let home = home_dir()?;
        let cwd = std::env::current_dir().map_err(|e| CliError::config(e.to_string()))?;
        Ok(match (self, scope) {
            (Host::ClaudeCode, Scope::User) => home.join(".claude.json"),
            (Host::ClaudeCode, Scope::Project) => cwd.join(".mcp.json"),
            (Host::ClaudeDesktop, _) => claude_desktop_config(&home),
            (Host::Codex, Scope::User) => codex_home(&home).join("config.toml"),
            (Host::Codex, Scope::Project) => cwd.join(".codex/config.toml"),
            (Host::Cursor, Scope::User) => home.join(".cursor/mcp.json"),
            (Host::Cursor, Scope::Project) => cwd.join(".cursor/mcp.json"),
            (Host::Gemini, Scope::User) => home.join(".gemini/settings.json"),
            (Host::Gemini, Scope::Project) => cwd.join(".gemini/settings.json"),
            (Host::Vscode, _) => cwd.join(".vscode/mcp.json"),
            (Host::Windsurf, _) => home.join(".codeium/windsurf/mcp_config.json"),
        })
    }

    fn skills(self, scope: Scope) -> Result<(SkillKind, PathBuf), CliError> {
        let home = home_dir()?;
        let cwd = std::env::current_dir().map_err(|e| CliError::config(e.to_string()))?;
        Ok(match (self, scope) {
            (Host::ClaudeCode, Scope::User) => (SkillKind::SkillMd, home.join(".claude/skills")),
            (Host::ClaudeCode, Scope::Project) => (SkillKind::SkillMd, cwd.join(".claude/skills")),
            (Host::Codex, Scope::User) => (SkillKind::SkillMd, home.join(".agents/skills")),
            (Host::Codex, Scope::Project) => (SkillKind::SkillMd, cwd.join(".agents/skills")),
            (Host::Cursor, Scope::Project) => (SkillKind::CursorRule, cwd.join(".cursor/rules")),
            _ => (SkillKind::None, PathBuf::new()),
        })
    }

    /// A cheap "is this host present on this machine?" signal.
    pub fn detected(self) -> bool {
        let Ok(home) = home_dir() else { return false };
        let probe = match self {
            Host::ClaudeCode => home.join(".claude"),
            Host::ClaudeDesktop => claude_desktop_config(&home)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
            Host::Codex => codex_home(&home),
            Host::Cursor => home.join(".cursor"),
            Host::Gemini => home.join(".gemini"),
            Host::Vscode => home.join(".vscode"),
            Host::Windsurf => home.join(".codeium/windsurf"),
        };
        probe.exists()
    }
}

fn home_dir() -> Result<PathBuf, CliError> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()))
        .ok_or_else(|| CliError::config("cannot determine home directory"))
}

fn codex_home(home: &Path) -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"))
}

fn claude_desktop_config(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Claude/claude_desktop_config.json")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData/Roaming"))
            .join("Claude/claude_desktop_config.json")
    } else {
        home.join(".config/Claude/claude_desktop_config.json")
    }
}

// ---- results ------------------------------------------------------------------

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Created,
    Updated,
    Unchanged,
    Removed,
}

#[derive(Debug, Serialize)]
pub struct SkillOutcome {
    pub name: &'static str,
    pub path: PathBuf,
    pub action: Action,
}

#[derive(Debug, Serialize)]
pub struct SetupResult {
    pub host: Host,
    pub scope: Scope,
    pub config_path: PathBuf,
    pub action: Action,
    pub command: String,
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup: Option<PathBuf>,
    pub skills: Vec<SkillOutcome>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HostStatus {
    pub host: Host,
    pub detected: bool,
    pub config_path: PathBuf,
    pub configured: bool,
    pub command: Option<String>,
    pub skills_installed: usize,
    pub skills_available: usize,
}

pub struct SetupOptions {
    pub scope: Scope,
    pub command: String,
    pub install_skills: bool,
    pub dry_run: bool,
}

/// The absolute path of this binary — GUI hosts often don't see the shell PATH.
pub fn default_command() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "hardcover".into())
}

fn server_object(command: &str, vscode: bool) -> serde_json::Value {
    let mut o = serde_json::json!({ "command": command, "args": ["mcp", "serve"], "env": {} });
    if vscode {
        o["type"] = "stdio".into();
    }
    o
}

// ---- setup --------------------------------------------------------------------

pub fn setup(host: Host, opts: &SetupOptions) -> Result<SetupResult, CliError> {
    if !host.supports_scope(opts.scope) {
        return Err(CliError::usage(
            format!("{} does not support --scope {:?}", host.name(), opts.scope).to_lowercase(),
        ));
    }
    let path = host.config_path(opts.scope)?;
    let mut notes = Vec::new();
    let (action, backup) = match host.format() {
        Format::McpServersJson => write_json(
            &path,
            "mcpServers",
            server_object(&opts.command, false),
            opts.dry_run,
        )?,
        Format::VscodeJson => write_json(
            &path,
            "servers",
            server_object(&opts.command, true),
            opts.dry_run,
        )?,
        Format::CodexToml => write_codex_toml(&path, &opts.command, opts.dry_run)?,
    };
    let mut skills = Vec::new();
    if opts.install_skills {
        let (kind, dir) = host.skills(opts.scope)?;
        match kind {
            SkillKind::None => {
                if matches!(host, Host::Cursor) {
                    notes.push("Cursor only reads rules from a project's .cursor/rules; re-run with --scope project inside a repo to install skills.".into());
                } else {
                    notes.push(format!(
                        "{} has no skills/rules mechanism; only the MCP server was configured.",
                        host.name()
                    ));
                }
            }
            SkillKind::SkillMd => {
                for s in SKILLS {
                    let p = dir.join(s.name).join("SKILL.md");
                    skills.push(SkillOutcome {
                        name: s.name,
                        action: write_file(&p, s.body, opts.dry_run)?,
                        path: p,
                    });
                }
            }
            SkillKind::CursorRule => {
                for s in SKILLS {
                    let p = dir.join(format!("{}.mdc", rule_name(s)));
                    skills.push(SkillOutcome {
                        name: s.name,
                        action: write_file(&p, &s.as_cursor_rule(), opts.dry_run)?,
                        path: p,
                    });
                }
            }
        }
    }
    match host {
        Host::ClaudeCode if opts.scope == Scope::Project => notes.push(
            "Claude Code asks for approval the first time a project .mcp.json is used.".into(),
        ),
        Host::ClaudeDesktop => notes.push("Restart Claude Desktop to load the server.".into()),
        _ => {}
    }
    Ok(SetupResult {
        host,
        scope: opts.scope,
        config_path: path,
        action,
        command: opts.command.clone(),
        dry_run: opts.dry_run,
        backup,
        skills,
        notes,
    })
}

fn rule_name(s: &Skill) -> String {
    if s.name == "hardcover" {
        "hardcover".into()
    } else {
        format!("hardcover-{}", s.name)
    }
}

pub fn remove(host: Host, scope: Scope, dry_run: bool) -> Result<SetupResult, CliError> {
    let path = host.config_path(scope)?;
    let action = match host.format() {
        Format::McpServersJson => remove_json(&path, "mcpServers", dry_run)?,
        Format::VscodeJson => remove_json(&path, "servers", dry_run)?,
        Format::CodexToml => remove_codex_toml(&path, dry_run)?,
    };
    let mut skills = Vec::new();
    let (kind, dir) = host.skills(scope)?;
    for s in SKILLS {
        let target = match kind {
            SkillKind::SkillMd => dir.join(s.name),
            SkillKind::CursorRule => dir.join(format!("{}.mdc", rule_name(s))),
            SkillKind::None => continue,
        };
        let act = if target.exists() {
            if !dry_run {
                if target.is_dir() {
                    std::fs::remove_dir_all(&target)
                } else {
                    std::fs::remove_file(&target)
                }
                .map_err(|e| {
                    CliError::config(format!("cannot remove {}: {e}", target.display()))
                })?;
            }
            Action::Removed
        } else {
            Action::Unchanged
        };
        skills.push(SkillOutcome {
            name: s.name,
            path: target,
            action: act,
        });
    }
    Ok(SetupResult {
        host,
        scope,
        config_path: path,
        action,
        command: String::new(),
        dry_run,
        backup: None,
        skills,
        notes: vec![],
    })
}

pub fn status() -> Vec<HostStatus> {
    Host::ALL
        .iter()
        .map(|&host| {
            let path = host.config_path(Scope::User).unwrap_or_default();
            let command = match host.format() {
                Format::CodexToml => read_codex_command(&path),
                Format::McpServersJson => read_json_command(&path, "mcpServers"),
                Format::VscodeJson => read_json_command(&path, "servers"),
            };
            let installed = host
                .skills(Scope::User)
                .ok()
                .map(|(kind, dir)| match kind {
                    SkillKind::SkillMd => SKILLS
                        .iter()
                        .filter(|s| dir.join(s.name).join("SKILL.md").exists())
                        .count(),
                    SkillKind::CursorRule => SKILLS
                        .iter()
                        .filter(|s| dir.join(format!("{}.mdc", rule_name(s))).exists())
                        .count(),
                    SkillKind::None => 0,
                })
                .unwrap_or(0);
            HostStatus {
                host,
                detected: host.detected(),
                config_path: path,
                configured: command.is_some(),
                command,
                skills_installed: installed,
                skills_available: SKILLS.len(),
            }
        })
        .collect()
}

pub fn detected_hosts() -> Vec<Host> {
    Host::ALL.into_iter().filter(|h| h.detected()).collect()
}

// ---- file editing -------------------------------------------------------------

fn read_optional(path: &Path) -> Result<Option<String>, CliError> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(CliError::config(format!(
            "cannot read {}: {e}",
            path.display()
        ))),
    }
}

/// Back up (once) and write. Returns the backup path if one was made.
fn commit(
    path: &Path,
    existing: Option<&str>,
    new: &str,
    dry_run: bool,
) -> Result<Option<PathBuf>, CliError> {
    if dry_run {
        return Ok(None);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| CliError::config(format!("cannot create {}: {e}", dir.display())))?;
    }
    let mut backup = None;
    if let Some(old) = existing {
        let bak = path.with_extension(format!(
            "{}bak",
            path.extension()
                .map(|e| format!("{}.", e.to_string_lossy()))
                .unwrap_or_default()
        ));
        if !bak.exists() {
            std::fs::write(&bak, old).map_err(|e| {
                CliError::config(format!("cannot write backup {}: {e}", bak.display()))
            })?;
            backup = Some(bak);
        }
    }
    std::fs::write(path, new)
        .map_err(|e| CliError::config(format!("cannot write {}: {e}", path.display())))?;
    Ok(backup)
}

fn write_json(
    path: &Path,
    key: &str,
    server: serde_json::Value,
    dry_run: bool,
) -> Result<(Action, Option<PathBuf>), CliError> {
    let existing = read_optional(path)?;
    let mut root: serde_json::Value = match &existing {
        Some(s) if !s.trim().is_empty() => serde_json::from_str(s).map_err(|e| {
            CliError::config(format!(
                "{} is not valid JSON ({e}); fix or move it and retry",
                path.display()
            ))
        })?,
        _ => serde_json::json!({}),
    };
    if !root.is_object() {
        return Err(CliError::config(format!(
            "{} is not a JSON object",
            path.display()
        )));
    }
    let servers = root[key].as_object().cloned().unwrap_or_default();
    if servers.get(SERVER_NAME) == Some(&server) {
        return Ok((Action::Unchanged, None));
    }
    let action = if existing.is_some() {
        Action::Updated
    } else {
        Action::Created
    };
    if !root[key].is_object() {
        root[key] = serde_json::json!({});
    }
    root[key][SERVER_NAME] = server;
    let text = serde_json::to_string_pretty(&root).unwrap() + "\n";
    let backup = commit(path, existing.as_deref(), &text, dry_run)?;
    Ok((action, backup))
}

fn remove_json(path: &Path, key: &str, dry_run: bool) -> Result<Action, CliError> {
    let Some(existing) = read_optional(path)? else {
        return Ok(Action::Unchanged);
    };
    let mut root: serde_json::Value = serde_json::from_str(&existing)
        .map_err(|e| CliError::config(format!("{} is not valid JSON ({e})", path.display())))?;
    let Some(servers) = root.get_mut(key).and_then(|v| v.as_object_mut()) else {
        return Ok(Action::Unchanged);
    };
    if servers.remove(SERVER_NAME).is_none() {
        return Ok(Action::Unchanged);
    }
    let text = serde_json::to_string_pretty(&root).unwrap() + "\n";
    commit(path, Some(&existing), &text, dry_run)?;
    Ok(Action::Removed)
}

fn read_json_command(path: &Path, key: &str) -> Option<String> {
    let root: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    root[key][SERVER_NAME]["command"]
        .as_str()
        .map(str::to_owned)
}

fn write_codex_toml(
    path: &Path,
    command: &str,
    dry_run: bool,
) -> Result<(Action, Option<PathBuf>), CliError> {
    let existing = read_optional(path)?;
    let mut doc: toml_edit::DocumentMut =
        existing.as_deref().unwrap_or("").parse().map_err(|e| {
            CliError::config(format!(
                "{} is not valid TOML ({e}); fix or move it and retry",
                path.display()
            ))
        })?;
    let mut server = toml_edit::Table::new();
    server["command"] = toml_edit::value(command);
    let mut args = toml_edit::Array::new();
    args.push("mcp");
    args.push("serve");
    server["args"] = toml_edit::value(args);
    if doc
        .get("mcp_servers")
        .and_then(|t| t.get(SERVER_NAME))
        .map(|t| t.to_string().trim() == server.to_string().trim())
        .unwrap_or(false)
    {
        return Ok((Action::Unchanged, None));
    }
    let action = if existing.is_some() {
        Action::Updated
    } else {
        Action::Created
    };
    if !doc.contains_table("mcp_servers") {
        let mut t = toml_edit::Table::new();
        t.set_implicit(true);
        doc["mcp_servers"] = toml_edit::Item::Table(t);
    }
    doc["mcp_servers"][SERVER_NAME] = toml_edit::Item::Table(server);
    let backup = commit(path, existing.as_deref(), &doc.to_string(), dry_run)?;
    Ok((action, backup))
}

fn remove_codex_toml(path: &Path, dry_run: bool) -> Result<Action, CliError> {
    let Some(existing) = read_optional(path)? else {
        return Ok(Action::Unchanged);
    };
    let mut doc: toml_edit::DocumentMut = existing
        .parse()
        .map_err(|e| CliError::config(format!("{} is not valid TOML ({e})", path.display())))?;
    let Some(servers) = doc.get_mut("mcp_servers").and_then(|t| t.as_table_mut()) else {
        return Ok(Action::Unchanged);
    };
    if servers.remove(SERVER_NAME).is_none() {
        return Ok(Action::Unchanged);
    }
    commit(path, Some(&existing), &doc.to_string(), dry_run)?;
    Ok(Action::Removed)
}

fn read_codex_command(path: &Path) -> Option<String> {
    let doc: toml_edit::DocumentMut = std::fs::read_to_string(path).ok()?.parse().ok()?;
    doc.get("mcp_servers")?
        .get(SERVER_NAME)?
        .get("command")?
        .as_str()
        .map(str::to_owned)
}

fn write_file(path: &Path, content: &str, dry_run: bool) -> Result<Action, CliError> {
    let existing = read_optional(path)?;
    if existing.as_deref() == Some(content) {
        return Ok(Action::Unchanged);
    }
    let action = if existing.is_some() {
        Action::Updated
    } else {
        Action::Created
    };
    if !dry_run {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| CliError::config(format!("cannot create {}: {e}", dir.display())))?;
        }
        std::fs::write(path, content)
            .map_err(|e| CliError::config(format!("cannot write {}: {e}", path.display())))?;
    }
    Ok(action)
}
