#![allow(dead_code)]
use assert_cmd::Command;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/../hardcover-api/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

pub async fn server() -> MockServer {
    MockServer::start().await
}

pub async fn respond(server: &MockServer, body_match: serde_json::Value, fixture_name: &str) {
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .and(body_partial_json(body_match))
        .respond_with(ResponseTemplate::new(200).set_body_raw(fixture(fixture_name), "application/json"))
        .mount(server)
        .await;
}

/// Run the binary with a test token against the mock server. Returns (status, stdout json or null, stderr).
pub async fn run(server: &MockServer, args: &[&str]) -> (Option<i32>, serde_json::Value, String) {
    let uri = server.uri();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let out = tokio::task::spawn_blocking(move || {
        Command::cargo_bin("hardcover")
            .unwrap()
            .env("HARDCOVER_TOKEN", "test-token")
            .env("HARDCOVER_API_URL", uri)
            .env("HARDCOVER_KEYRING", "mock")
            .args(&args)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    let stdout = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    (out.status.code(), stdout, String::from_utf8_lossy(&out.stderr).into_owned())
}
