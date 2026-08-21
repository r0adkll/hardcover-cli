mod common;
use common::*;

#[tokio::test]
async fn collection_commands_report_limit_and_offset() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"author_id": 154428, "limit": 3, "offset": 0}}),
        "author_books_page.json",
    )
    .await;

    let (code, json, stderr) = run(
        &s,
        &[
            "author", "books", "154428", "--limit", "3", "--format", "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"].as_array().unwrap().len(), 3);
    assert_eq!(json["data"][0]["slug"], "whose-body");
    assert_eq!(json["meta"]["limit"], 3);
    assert_eq!(json["meta"]["offset"], 0);
    assert_eq!(json["meta"]["truncated"], false);
}

#[tokio::test]
async fn all_flag_pages_until_a_short_page() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"author_id": 154428, "offset": 0}}),
        "author_books_page.json",
    )
    .await;
    respond(
        &s,
        serde_json::json!({"variables": {"author_id": 154428, "offset": 3}}),
        "author_books_empty.json",
    )
    .await;

    let (code, json, stderr) = run(
        &s,
        &[
            "author", "books", "154428", "--all", "--limit", "3", "--format", "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"].as_array().unwrap().len(), 3);
    assert_eq!(json["meta"]["truncated"], false);
}

#[tokio::test]
async fn all_flag_marks_truncation_at_the_safety_cap() {
    let s = server().await;
    // Every page returns 3 rows; with --max-rows 6 we must stop after two pages and flag it.
    respond(
        &s,
        serde_json::json!({"variables": {"author_id": 154428}}),
        "author_books_page.json",
    )
    .await;

    let (code, json, stderr) = run(
        &s,
        &[
            "author",
            "books",
            "154428",
            "--all",
            "--limit",
            "3",
            "--max-rows",
            "6",
            "--format",
            "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"].as_array().unwrap().len(), 6);
    assert_eq!(json["meta"]["truncated"], true);
}

#[tokio::test]
async fn ndjson_streams_one_object_per_line() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"author_id": 154428}}),
        "author_books_page.json",
    )
    .await;

    let (code, _, stderr) = run(
        &s,
        &[
            "author", "books", "154428", "--limit", "3", "--format", "ndjson",
        ],
    )
    .await;
    assert_eq!(code, Some(0), "{stderr}");

    let raw = run_raw(
        &s,
        &[
            "author", "books", "154428", "--limit", "3", "--format", "ndjson",
        ],
    )
    .await;
    let lines: Vec<&str> = raw.lines().collect();
    assert_eq!(lines.len(), 3);
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["slug"], "whose-body");
}
