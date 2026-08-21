mod common;
use common::*;
use rmcp::model::CallToolRequestParams;
use rmcp::service::ServiceExt;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::RmcpError;
use tokio::process::Command;

async fn mcp_client(
    server: &wiremock::MockServer,
) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    let uri = server.uri();
    ().serve(
        TokioChildProcess::new(
            Command::new(env!("CARGO_BIN_EXE_hardcover")).configure(|c| {
                c.args(["mcp", "serve"])
                    .env("HARDCOVER_TOKEN", "test-token")
                    .env("HARDCOVER_API_URL", uri)
                    .env("HARDCOVER_KEYRING", "mock");
            }),
        )
        .map_err(RmcpError::transport_creation::<TokioChildProcess>)
        .unwrap(),
    )
    .await
    .unwrap()
}

fn args(v: serde_json::Value) -> rmcp::model::JsonObject {
    v.as_object().unwrap().clone()
}

#[tokio::test]
async fn lists_one_tool_per_cli_command_and_serves_book_show() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"id": 1}}),
        "book_by_pk_1.json",
    )
    .await;
    let client = mcp_client(&s).await;

    let tools = client.list_all_tools().await.unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    for expected in [
        "search",
        "book_show",
        "book_editions",
        "author_show",
        "author_books",
        "series_show",
        "series_books",
        "list_show",
        "list_books",
        "edition_show",
        "prompt_show",
        "user_show",
        "whoami",
        "library_list",
        "library_show",
        "library_set_status",
        "library_rate",
        "library_progress",
        "library_remove",
    ] {
        assert!(
            names.contains(&expected),
            "missing tool {expected}; have {names:?}"
        );
    }
    let remove = tools.iter().find(|t| t.name == "library_remove").unwrap();
    assert_eq!(
        remove.annotations.as_ref().and_then(|a| a.destructive_hint),
        Some(true)
    );

    let r = client
        .call_tool(
            CallToolRequestParams::new("book_show")
                .with_arguments(args(serde_json::json!({"identifier": "1"}))),
        )
        .await
        .unwrap();
    let data = r.structured_content.expect("structured content");
    assert_eq!(data["data"]["slug"], "lord-peter-views-the-body");
    assert_eq!(data["meta"]["resolved_by"], "id");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn write_tools_support_dry_run_and_errors_carry_cli_codes() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond_op(&s, "LibraryEntryQuery", "library_entry_missing.json").await;
    let client = mcp_client(&s).await;

    let r = client
        .call_tool(
            CallToolRequestParams::new("library_set_status").with_arguments(args(
                serde_json::json!({
                    "identifier": "1", "status": "currently_reading", "dry_run": true
                }),
            )),
        )
        .await
        .unwrap();
    let data = r.structured_content.unwrap();
    assert_eq!(data["data"]["dry_run"], true);
    assert_eq!(data["data"]["planned"]["status"], "currently_reading");
    assert!(!s
        .received_requests()
        .await
        .unwrap()
        .iter()
        .any(|q| String::from_utf8_lossy(&q.body).contains("mutation")));

    // not-in-library → tool error with the CLI's stable code
    let r = client
        .call_tool(
            CallToolRequestParams::new("library_show")
                .with_arguments(args(serde_json::json!({"identifier": "1"}))),
        )
        .await
        .unwrap();
    assert_eq!(r.is_error, Some(true));
    let err = r.structured_content.unwrap();
    assert_eq!(err["error"]["code"], "not_found");
    client.cancel().await.unwrap();
}
