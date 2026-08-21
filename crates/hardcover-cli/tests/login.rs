use assert_cmd::Command;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("../hardcover-api/tests/fixtures/{name}")).unwrap()
}

async fn login_with(token: &str, server: &MockServer) -> std::process::Output {
    let uri = server.uri();
    let token = token.to_string();
    tokio::task::spawn_blocking(move || {
        Command::cargo_bin("hardcover")
            .unwrap()
            .env_remove("HARDCOVER_TOKEN")
            .env("HARDCOVER_API_URL", uri)
            .env("HARDCOVER_KEYRING", "mock")
            .args(["login", "--format", "json"])
            .write_stdin(token)
            .output()
            .unwrap()
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn login_reads_token_from_stdin_and_reports_the_verified_user() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .and(header("authorization", "Bearer hc_pat_good"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(fixture("me.json"), "application/json"))
        .mount(&server)
        .await;

    let out = login_with("hc_pat_good\n", &server).await;

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["data"]["username"], "r0adkll");
    assert_eq!(json["data"]["id"], 31899);
}

#[tokio::test]
async fn login_rejects_an_invalid_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .respond_with(ResponseTemplate::new(401).set_body_raw(fixture("me_invalid_token.json"), "application/json"))
        .mount(&server)
        .await;

    let out = login_with("hc_pat_bad\n", &server).await;

    assert_eq!(out.status.code(), Some(3));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "invalid_token");
}
