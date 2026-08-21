#![allow(dead_code)]
use hardcover_api::Client;
use wiremock::matchers::{body_partial_json, body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

pub async fn server() -> MockServer {
    MockServer::start().await
}

/// Serve `fixture` for any POST /v1/graphql whose JSON body contains `body_match`.
pub async fn respond(server: &MockServer, body_match: serde_json::Value, fixture_name: &str) {
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .and(header("authorization", "Bearer test-token"))
        .and(body_partial_json(body_match))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(fixture(fixture_name), "application/json"),
        )
        .mount(server)
        .await;
}

pub fn client(server: &MockServer) -> Client {
    Client::builder("test-token").base_url(server.uri()).build()
}

/// Serve `fixture` for any request whose body mentions the named GraphQL operation.
pub async fn respond_op(server: &MockServer, op: &str, fixture_name: &str) {
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .and(body_string_contains(format!("query {op}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(fixture(fixture_name), "application/json"),
        )
        .mount(server)
        .await;
}
