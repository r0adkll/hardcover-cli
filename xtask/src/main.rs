//! Repo maintenance tasks. Run via `cargo xtask <task>`.
use std::path::Path;

const SCHEMA_PATH: &str = "crates/hardcover-api/schema.json";
const ENDPOINT: &str = "https://api.hardcover.app/v1/graphql";
/// Public copy maintained in Hardcover's docs repo. Lags the live API, so it is only a
/// fallback for when no token is available.
const DOCS_SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/hardcoverapp/hardcover-docs/main/schema.json";

fn main() {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "introspect" => introspect(),
        _ => {
            eprintln!(
                "usage: cargo xtask introspect\n\n  introspect   refresh {SCHEMA_PATH} by live introspection (HARDCOVER_TOKEN),\n               or from Hardcover's public docs copy when no token is set"
            );
            std::process::exit(2);
        }
    }
}

fn introspect() {
    let http = reqwest::blocking::Client::builder()
        .user_agent("hardcover-cli xtask (+https://github.com/r0adkll/hardcover-cli)")
        .build()
        .unwrap();
    let (source, resp): (&str, serde_json::Value) = match std::env::var("HARDCOVER_TOKEN") {
        Ok(token) => {
            let resp = http
                .post(ENDPOINT)
                .bearer_auth(token)
                .json(&serde_json::json!({ "query": graphql_client_introspection_query() }))
                .send()
                .expect("request")
                .error_for_status()
                .expect("status")
                .json()
                .expect("json");
            ("live introspection", resp)
        }
        Err(_) => {
            eprintln!(
                "HARDCOVER_TOKEN not set; falling back to {DOCS_SCHEMA_URL} (may lag the live API)"
            );
            let resp = http
                .get(DOCS_SCHEMA_URL)
                .send()
                .expect("request")
                .error_for_status()
                .expect("status")
                .json()
                .expect("json");
            ("docs repo copy", resp)
        }
    };
    assert!(
        resp.get("data").and_then(|d| d.get("__schema")).is_some(),
        "no __schema in response from {source}"
    );
    let out = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(SCHEMA_PATH);
    let before = std::fs::read_to_string(&out).unwrap_or_default();
    let after = serde_json::to_string(&resp).unwrap();
    std::fs::write(&out, &after).expect("write schema");
    println!(
        "{} {} ({source})",
        if before == after {
            "unchanged"
        } else {
            "updated"
        },
        SCHEMA_PATH
    );
}

fn graphql_client_introspection_query() -> &'static str {
    r#"query IntrospectionQuery { __schema { queryType { name } mutationType { name } subscriptionType { name } types { ...FullType } directives { name description locations args { ...InputValue } } } }
fragment FullType on __Type { kind name description fields(includeDeprecated: true) { name description args { ...InputValue } type { ...TypeRef } isDeprecated deprecationReason } inputFields { ...InputValue } interfaces { ...TypeRef } enumValues(includeDeprecated: true) { name description isDeprecated deprecationReason } possibleTypes { ...TypeRef } }
fragment InputValue on __InputValue { name description type { ...TypeRef } defaultValue }
fragment TypeRef on __Type { kind name ofType { kind name ofType { kind name ofType { kind name ofType { kind name ofType { kind name ofType { kind name ofType { kind name } } } } } } } }"#
}
