mod common;
use common::*;

#[tokio::test]
async fn raw_flag_passes_the_upstream_payload_through() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"id": 1}}),
        "book_by_pk_1.json",
    )
    .await;

    let (code, json, stderr) = run(&s, &["book", "show", "1", "--raw"]).await;

    assert_eq!(code, Some(0), "{stderr}");
    // Upstream Hasura shape, not the CLI envelope.
    assert_eq!(json["data"]["books_by_pk"]["id"], 1);
    assert!(json.get("schema").is_none());
}

#[tokio::test]
async fn no_retry_flag_surfaces_rate_limit_with_retry_after() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};
    let s = server().await;
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "7"))
        .mount(&s)
        .await;

    let (code, _, stderr) = run(&s, &["whoami", "--no-retry"]).await;

    assert_eq!(code, Some(5));
    let err: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err["error"]["code"], "rate_limited");
    assert_eq!(err["error"]["retry_after_secs"], 7);
    assert_eq!(s.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn schema_command_describes_commands_and_error_codes_without_auth() {
    let s = server().await;
    let (code, json, stderr) = run(&s, &["schema", "--format", "json"]).await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["schema"], "hardcover-cli/1");
    let cmds = json["data"]["commands"].as_array().unwrap();
    assert!(cmds.iter().any(|c| c["path"] == "book show"), "{cmds:?}");
    assert!(cmds.iter().any(|c| c["path"] == "search"));
    let codes = json["data"]["error_codes"].as_array().unwrap();
    assert!(codes
        .iter()
        .any(|c| c["code"] == "rate_limited" && c["exit"] == 5));
    assert!(json["data"]["formats"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f == "ndjson"));
    assert_eq!(s.received_requests().await.unwrap().len(), 0);
}
