mod common;
use common::*;

async fn entry_absent_then_present(s: &wiremock::MockServer) {
    respond_op(s, "Me", "me.json").await;
    // `library_entry` is asked before and after; first answer: not shelved, later: shelved.
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, ResponseTemplate};
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .and(body_string_contains("query LibraryEntryQuery"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(fixture("library_entry_missing.json"), "application/json"),
        )
        .up_to_n_times(1)
        .mount(s)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .and(body_string_contains("query LibraryEntryQuery"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(fixture("library_entry.json"), "application/json"),
        )
        .mount(s)
        .await;
}

#[tokio::test]
async fn set_status_adds_when_absent_and_reports_before_and_after() {
    let s = server().await;
    entry_absent_then_present(&s).await;
    respond_mut(&s, "InsertUserBook", "write_insert_user_book.json").await;

    let (code, json, stderr) = run(
        &s,
        &[
            "library",
            "set-status",
            "1",
            "want_to_read",
            "--format",
            "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["action"], "added");
    assert!(json["data"]["before"].is_null());
    assert_eq!(json["data"]["after"]["status"], "want_to_read"); // from the mutation response, not the (possibly stale) re-fetch
    assert!(s
        .received_requests()
        .await
        .unwrap()
        .iter()
        .any(|r| String::from_utf8_lossy(&r.body).contains("mutation InsertUserBook")));
}

#[tokio::test]
async fn dry_run_sends_no_mutation() {
    let s = server().await;
    entry_absent_then_present(&s).await;

    let (code, json, stderr) = run(
        &s,
        &[
            "library",
            "set-status",
            "1",
            "reading",
            "--dry-run",
            "--format",
            "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["dry_run"], true);
    assert_eq!(json["data"]["action"], "added");
    assert_eq!(json["data"]["planned"]["status"], "currently_reading");
    assert!(!s
        .received_requests()
        .await
        .unwrap()
        .iter()
        .any(|r| String::from_utf8_lossy(&r.body).contains("mutation")));
}

#[tokio::test]
async fn rate_validates_half_star_range() {
    let s = server().await;
    let (code, _, stderr) = run(&s, &["library", "rate", "1", "4.3"]).await;
    assert_eq!(code, Some(2));
    assert!(stderr.contains("usage_error"), "{stderr}");
    let (code, _, stderr) = run(&s, &["library", "rate", "1", "6"]).await;
    assert_eq!(code, Some(2), "{stderr}");
}

#[tokio::test]
async fn progress_updates_the_open_read_when_one_exists() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond_op(&s, "LibraryEntryQuery", "library_entry.json").await; // has an unfinished read #2626228 at 12 pages
    respond(
        &s,
        serde_json::json!({"variables": {"id": 2626228, "read": {"progress_pages": 120}}}),
        "write_update_read.json",
    )
    .await;

    let (code, json, stderr) = run(
        &s,
        &[
            "library", "progress", "427678", "--pages", "120", "--format", "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["action"], "updated_read");
    assert_eq!(json["data"]["read"]["progress_pages"], 120);
}

#[tokio::test]
async fn remove_reports_the_deleted_entry() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond_op(&s, "BookIdBySlug", "book_id_by_slug.json").await;
    respond_op(&s, "LibraryEntryQuery", "library_entry.json").await;
    respond_mut(&s, "DeleteUserBook", "write_delete.json").await;

    let (code, json, stderr) = run(
        &s,
        &[
            "library",
            "remove",
            "lord-peter-views-the-body",
            "--format",
            "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["action"], "removed");
    assert_eq!(json["data"]["before"]["book"]["slug"], "iron-gold");
    assert!(json["data"]["after"].is_null());
}
