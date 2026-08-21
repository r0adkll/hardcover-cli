mod common;
use common::*;

#[tokio::test]
async fn search_defaults_to_books_and_reports_paging_meta() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"query": "dune", "query_type": "book"}}),
        "search_book_dune.json",
    )
    .await;

    let (code, json, stderr) = run(
        &s,
        &["search", "dune", "--per-page", "3", "--format", "json"],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["hits"][0]["id"], 312460);
    assert_eq!(json["data"]["hits"][0]["label"], "Dune");
    assert_eq!(json["meta"]["page"], 1);
    assert_eq!(json["meta"]["per_page"], 3);
    assert_eq!(json["meta"]["found"], 840);
}

#[tokio::test]
async fn search_type_flag_selects_entity() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"query_type": "author"}}),
        "search_author_sanderson.json",
    )
    .await;

    let (code, json, stderr) = run(
        &s,
        &[
            "search",
            "sanderson",
            "--type",
            "author",
            "--format",
            "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["hits"][0]["label"], "Brandon Sanderson");
}
