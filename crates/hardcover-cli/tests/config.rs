mod common;
use common::*;
use assert_cmd::Command;

#[tokio::test]
async fn config_file_sets_default_format_and_login_records_username() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"id": 1}}), "book_by_pk_1.json").await;
    respond_op(&s, "Me", "me.json").await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("config.toml"), "format = \"ndjson\"\n").unwrap();
    let cfg = dir.path().to_str().unwrap().to_string();

    // format default comes from the file: bare object, no envelope
    let out = {
        let (cfg, uri) = (cfg.clone(), s.uri());
        tokio::task::spawn_blocking(move || {
            Command::cargo_bin("hardcover").unwrap()
                .env("HARDCOVER_TOKEN", "test-token").env("HARDCOVER_API_URL", uri)
                .env("HARDCOVER_CONFIG_DIR", &cfg).env("HARDCOVER_KEYRING", "mock")
                .args(["book", "show", "1"]).output().unwrap()
        }).await.unwrap()
    };
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["id"], 1);
    assert!(json.get("schema").is_none());

    // login writes username, preserving the existing setting
    let out = {
        let (cfg, uri) = (cfg.clone(), s.uri());
        tokio::task::spawn_blocking(move || {
            Command::cargo_bin("hardcover").unwrap()
                .env_remove("HARDCOVER_TOKEN").env("HARDCOVER_API_URL", uri)
                .env("HARDCOVER_CONFIG_DIR", &cfg).env("HARDCOVER_KEYRING", "mock")
                .args(["login"]).write_stdin("hc_pat_x\n").output().unwrap()
        }).await.unwrap()
    };
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let written = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
    assert!(written.contains("username = \"r0adkll\""), "{written}");
    assert!(written.contains("format = \"ndjson\""), "{written}");
}
