//! `hardcover agent …`: wiring the MCP server and shipped skills into agent hosts.
//! All file I/O is redirected via HOME / cwd into a temp dir; no network.
use assert_cmd::Command;
use std::path::Path;

struct Sandbox {
    home: tempfile::TempDir,
    project: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            home: tempfile::tempdir().unwrap(),
            project: tempfile::tempdir().unwrap(),
        }
    }
    fn home(&self) -> &Path {
        self.home.path()
    }
    fn run(&self, args: &[&str]) -> (Option<i32>, serde_json::Value, String) {
        let out = Command::cargo_bin("hardcover")
            .unwrap()
            .current_dir(self.project.path())
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path().join(".config"))
            .env("HARDCOVER_TOKEN", "test-token")
            .env("HARDCOVER_KEYRING", "mock")
            .env("HARDCOVER_CONFIG_DIR", self.home.path().join("hc"))
            .args(args)
            .arg("--format")
            .arg("json")
            .output()
            .unwrap();
        let json = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
        (
            out.status.code(),
            json,
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.home().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
    }
}

#[test]
fn claude_code_user_setup_merges_server_and_installs_skills() {
    let sb = Sandbox::new();
    std::fs::write(
        sb.home().join(".claude.json"),
        r#"{"mcpServers":{"other":{"command":"x","args":[]}},"theme":"dark"}"#,
    )
    .unwrap();

    let (code, json, stderr) = sb.run(&["agent", "setup", "claude-code"]);

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["host"], "claude-code");
    assert_eq!(json["data"]["scope"], "user");
    assert_eq!(json["data"]["action"], "updated");
    let cfg: serde_json::Value = serde_json::from_str(&sb.read(".claude.json")).unwrap();
    assert_eq!(cfg["theme"], "dark", "unrelated keys preserved");
    assert_eq!(
        cfg["mcpServers"]["other"]["command"], "x",
        "other servers preserved"
    );
    let hc = &cfg["mcpServers"]["hardcover"];
    assert_eq!(hc["args"], serde_json::json!(["mcp", "serve"]));
    assert!(
        Path::new(hc["command"].as_str().unwrap()).is_absolute(),
        "command is absolute: {hc}"
    );
    assert!(
        sb.home().join(".claude.json.bak").exists(),
        "backup written"
    );
    for skill in ["hardcover", "reading-log", "book-research"] {
        let body = sb.read(&format!(".claude/skills/{skill}/SKILL.md"));
        assert!(body.starts_with("---\nname: "), "{skill} has frontmatter");
    }
    assert_eq!(json["data"]["skills"].as_array().unwrap().len(), 3);
}

#[test]
fn codex_setup_edits_toml_preserving_comments_and_other_servers() {
    let sb = Sandbox::new();
    std::fs::create_dir_all(sb.home().join(".codex")).unwrap();
    std::fs::write(
        sb.home().join(".codex/config.toml"),
        "# my codex config\nmodel = \"gpt-5\"\n\n[mcp_servers.other]\ncommand = \"x\"\nargs = []\n",
    )
    .unwrap();

    let (code, json, stderr) = sb.run(&["agent", "setup", "codex"]);

    assert_eq!(code, Some(0), "{stderr}");
    let toml = sb.read(".codex/config.toml");
    assert!(
        toml.contains("# my codex config"),
        "comments preserved:\n{toml}"
    );
    assert!(toml.contains("model = \"gpt-5\""));
    assert!(toml.contains("[mcp_servers.other]"));
    assert!(toml.contains("[mcp_servers.hardcover]"));
    assert!(toml.contains("args = [\"mcp\", \"serve\"]"));
    // skills land in the agentskills.io location Codex reads
    assert!(sb.home().join(".agents/skills/hardcover/SKILL.md").exists());
    assert_eq!(
        json["data"]["config_path"].as_str().unwrap(),
        sb.home().join(".codex/config.toml").to_str().unwrap()
    );
}

#[test]
fn cursor_project_scope_writes_into_the_current_directory() {
    let sb = Sandbox::new();

    let (code, json, stderr) = sb.run(&["agent", "setup", "cursor", "--scope", "project"]);

    assert_eq!(code, Some(0), "{stderr}");
    let proj = sb.project.path();
    let cfg: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(proj.join(".cursor/mcp.json")).unwrap())
            .unwrap();
    assert_eq!(cfg["mcpServers"]["hardcover"]["args"][0], "mcp");
    let rule = std::fs::read_to_string(proj.join(".cursor/rules/hardcover.mdc")).unwrap();
    assert!(rule.starts_with("---\ndescription:"), "{rule}");
    assert!(rule.contains("alwaysApply: false"));
    assert_eq!(json["data"]["skills"].as_array().unwrap().len(), 3);
}

#[test]
fn claude_desktop_gets_no_skills_and_uses_its_platform_path() {
    let sb = Sandbox::new();

    let (code, json, stderr) = sb.run(&["agent", "setup", "claude-desktop"]);

    assert_eq!(code, Some(0), "{stderr}");
    let path = json["data"]["config_path"].as_str().unwrap();
    assert!(path.ends_with("claude_desktop_config.json"), "{path}");
    assert!(path.starts_with(sb.home().to_str().unwrap()));
    assert_eq!(json["data"]["skills"].as_array().unwrap().len(), 0);
    assert_eq!(json["data"]["action"], "created");
}

#[test]
fn remove_deletes_only_our_entry_and_our_skills() {
    let sb = Sandbox::new();
    std::fs::write(
        sb.home().join(".claude.json"),
        r#"{"mcpServers":{"other":{"command":"x"}}}"#,
    )
    .unwrap();
    std::fs::create_dir_all(sb.home().join(".claude/skills/mine")).unwrap();
    std::fs::write(
        sb.home().join(".claude/skills/mine/SKILL.md"),
        "---\nname: mine\n---\n",
    )
    .unwrap();
    sb.run(&["agent", "setup", "claude-code"]);

    let (code, json, stderr) = sb.run(&["agent", "remove", "claude-code"]);

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["action"], "removed");
    let cfg: serde_json::Value = serde_json::from_str(&sb.read(".claude.json")).unwrap();
    assert!(cfg["mcpServers"].get("hardcover").is_none());
    assert_eq!(cfg["mcpServers"]["other"]["command"], "x");
    assert!(!sb.home().join(".claude/skills/hardcover").exists());
    assert!(
        sb.home().join(".claude/skills/mine/SKILL.md").exists(),
        "foreign skills untouched"
    );
}

#[test]
fn unparseable_config_is_refused_not_clobbered() {
    let sb = Sandbox::new();
    std::fs::write(sb.home().join(".claude.json"), "{ not json").unwrap();

    let (code, _, stderr) = sb.run(&["agent", "setup", "claude-code"]);

    assert_eq!(code, Some(1), "{stderr}");
    let err: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err["error"]["code"], "config_error");
    assert_eq!(sb.read(".claude.json"), "{ not json");
}

#[test]
fn setup_is_idempotent_and_status_reports_what_is_configured() {
    let sb = Sandbox::new();
    sb.run(&["agent", "setup", "claude-code"]);
    let (_, second, _) = sb.run(&["agent", "setup", "claude-code"]);
    assert_eq!(second["data"]["action"], "unchanged");

    let (code, json, stderr) = sb.run(&["agent", "status"]);
    assert_eq!(code, Some(0), "{stderr}");
    let hosts = json["data"].as_array().unwrap();
    let cc = hosts.iter().find(|h| h["host"] == "claude-code").unwrap();
    assert_eq!(cc["configured"], true);
    assert_eq!(cc["skills_installed"], 3);
    let cx = hosts.iter().find(|h| h["host"] == "codex").unwrap();
    assert_eq!(cx["configured"], false);
}

#[test]
fn skills_command_lists_shipped_skills() {
    let sb = Sandbox::new();
    let (code, json, stderr) = sb.run(&["agent", "skills"]);
    assert_eq!(code, Some(0), "{stderr}");
    let names: Vec<&str> = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["hardcover", "reading-log", "book-research"]);
    assert!(json["data"][0]["description"].as_str().unwrap().len() > 20);
}

#[test]
fn setup_without_host_on_non_tty_lists_detected_hosts_as_usage_error() {
    let sb = Sandbox::new();
    std::fs::create_dir_all(sb.home().join(".cursor")).unwrap();
    let (code, _, stderr) = sb.run(&["agent", "setup"]);
    assert_eq!(code, Some(2), "{stderr}");
    assert!(stderr.contains("cursor"), "{stderr}");
}
