use assert_cmd::Command;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn server_with(fixture: &str) -> MockServer {
    let body =
        std::fs::read_to_string(format!("../hardcover-api/tests/fixtures/{fixture}")).unwrap();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn book_show_emits_json_envelope() {
    let server = server_with("book_by_pk_1.json").await;
    let out = tokio::task::spawn_blocking(move || {
        Command::cargo_bin("hardcover")
            .unwrap()
            .env("HARDCOVER_TOKEN", "test-token")
            .env("HARDCOVER_API_URL", server.uri())
            .args(["book", "show", "1", "--format", "json"])
            .output()
            .unwrap()
    })
    .await
    .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["schema"], "hardcover-cli/1");
    assert_eq!(json["data"]["id"], 1);
    assert_eq!(json["data"]["slug"], "lord-peter-views-the-body");
    assert_eq!(json["data"]["title"], "Lord Peter Views the Body");
    assert_eq!(json["data"]["contributors"][0]["name"], "Dorothy L. Sayers");
}
