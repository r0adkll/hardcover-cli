mod common;
use common::*;

#[tokio::test]
async fn book_show_accepts_a_slug_and_reports_how_it_resolved() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"slug": "lord-peter-views-the-body"}}), "book_id_by_slug.json").await;
    respond(&s, serde_json::json!({"variables": {"id": 1}}), "book_by_pk_1.json").await;

    let (code, json, stderr) = run(&s, &["book", "show", "lord-peter-views-the-body", "--format", "json"]).await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["id"], 1);
    assert_eq!(json["meta"]["resolved_by"], "slug");
}

#[tokio::test]
async fn book_show_accepts_an_isbn() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"isbn": "9780441172719"}}), "book_id_by_isbn.json").await;
    respond(&s, serde_json::json!({"variables": {"id": 312460}}), "book_by_pk_1.json").await;

    let (code, json, stderr) = run(&s, &["book", "show", "978-0-441-17271-9", "--format", "json"]).await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["meta"]["resolved_by"], "isbn");
}

#[tokio::test]
async fn book_show_by_id_reports_resolved_by_id() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"id": 1}}), "book_by_pk_1.json").await;

    let (_, json, _) = run(&s, &["book", "show", "1", "--format", "json"]).await;

    assert_eq!(json["meta"]["resolved_by"], "id");
}
