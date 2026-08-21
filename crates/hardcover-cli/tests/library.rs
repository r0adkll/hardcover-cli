mod common;
use common::*;

#[tokio::test]
async fn library_list_with_status_filter() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond(
        &s,
        serde_json::json!({"variables": {"status_ids": [3]}}),
        "library_page_read.json",
    )
    .await;

    let (code, json, stderr) = run(
        &s,
        &[
            "library", "list", "--status", "read", "--limit", "3", "--format", "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"][0]["status"], "read");
    assert_eq!(json["data"][0]["status_id"], 3);
    assert_eq!(json["data"][0]["book"]["title"], "Iron Gold");
    assert_eq!(json["meta"]["status"], "read");
    assert_eq!(json["meta"]["limit"], 3);
}

#[tokio::test]
async fn library_show_by_slug_includes_reads() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond_op(&s, "BookIdBySlug", "book_id_by_slug.json").await;
    // slug fixture resolves to id 1; serve the entry for whatever book id is asked
    respond_op(&s, "LibraryEntryQuery", "library_entry.json").await;

    let (code, json, stderr) = run(
        &s,
        &[
            "library",
            "show",
            "lord-peter-views-the-body",
            "--format",
            "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["reads"].as_array().unwrap().len(), 2);
    assert_eq!(json["meta"]["resolved_by"], "slug");
}

#[tokio::test]
async fn max_rows_zero_means_unlimited() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond(
        &s,
        serde_json::json!({"variables": {"offset": 0}}),
        "library_page.json",
    )
    .await;
    respond(
        &s,
        serde_json::json!({"variables": {"offset": 3}}),
        "library_page.json",
    )
    .await;
    respond(
        &s,
        serde_json::json!({"variables": {"offset": 6}}),
        "library_entry_missing.json",
    )
    .await; // empty user_books

    let (code, json, stderr) = run(
        &s,
        &[
            "library",
            "list",
            "--all",
            "--limit",
            "3",
            "--max-rows",
            "0",
            "--format",
            "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"].as_array().unwrap().len(), 6);
    assert_eq!(json["meta"]["truncated"], false);
}
