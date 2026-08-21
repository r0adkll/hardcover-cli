mod common;
use common::*;

#[tokio::test]
async fn writes_a_markdown_review_and_reads_back_rendered_forms() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"id": 17692741, "review_markdown": "A **witty** set of stories.\n\nSecond paragraph.", "spoilers": false}}), "review_write.json").await;

    let e = client(&s)
        .library_set_review(
            17692741,
            Some("A **witty** set of stories.\n\nSecond paragraph."),
            false,
        )
        .await
        .unwrap();

    assert!(e.has_review);
    assert_eq!(
        e.review.as_deref(),
        Some("A **witty** set of stories.\n\nSecond paragraph."),
        "review is the markdown source"
    );
    assert!(e
        .review_html
        .as_deref()
        .unwrap()
        .starts_with("<p>A <strong>witty</strong>"));
    assert_eq!(e.review_has_spoilers, Some(false));
    assert!(e.reviewed_at.is_some());
}

#[tokio::test]
async fn library_entry_detail_exposes_review_fields() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond(
        &s,
        serde_json::json!({"variables": {"book_id": 427678}}),
        "library_entry.json",
    )
    .await;

    let d = client(&s).library_entry(427678).await.unwrap();

    assert_eq!(d.entry.review, None);
    assert!(!d.entry.has_review);
}
