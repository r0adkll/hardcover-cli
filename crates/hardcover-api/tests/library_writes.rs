mod common;
use common::*;
use hardcover_api::model::{ProgressUpdate, ReadingStatus};
use hardcover_api::Error;

#[tokio::test]
async fn adds_a_book_to_the_library_with_a_status() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond(
        &s,
        serde_json::json!({"variables": {"book_id": 1, "status_id": 1}}),
        "write_insert_user_book.json",
    )
    .await;

    let e = client(&s)
        .library_add(1, ReadingStatus::WantToRead, None)
        .await
        .unwrap();

    assert_eq!(e.id, 17692741);
    assert_eq!(e.status, ReadingStatus::WantToRead);
    assert_eq!(e.book.slug, "lord-peter-views-the-body");
}

#[tokio::test]
async fn updates_status_and_rating_on_an_existing_entry() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"id": 17692741, "status_id": 2}}),
        "write_update_status.json",
    )
    .await;
    respond(
        &s,
        serde_json::json!({"variables": {"id": 17692741, "rating": 4.0}}),
        "write_rate.json",
    )
    .await;
    let c = client(&s);

    let e = c
        .library_set_status(17692741, ReadingStatus::CurrentlyReading)
        .await
        .unwrap();
    assert_eq!(e.status, ReadingStatus::CurrentlyReading);

    let e = c.library_set_rating(17692741, Some(4.0)).await.unwrap();
    assert_eq!(e.rating, Some(4.0));
    assert_eq!(
        e.status,
        ReadingStatus::Read,
        "upstream marks rated books as read"
    );
}

#[tokio::test]
async fn upstream_error_string_becomes_not_found() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"id": 999999999}}),
        "write_update_not_owned.json",
    )
    .await;

    let err = client(&s)
        .library_set_rating(999999999, Some(4.0))
        .await
        .unwrap_err();

    assert!(matches!(err, Error::NotFound(_)), "{err:?}");
}

#[tokio::test]
async fn starts_a_read_with_progress() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"user_book_id": 17692741, "read": {"progress_pages": 50, "started_at": "2026-08-21"}}}), "write_insert_read.json").await;

    let r = client(&s)
        .read_start(
            17692741,
            ProgressUpdate {
                pages: Some(50),
                seconds: None,
                started_at: Some("2026-08-21".into()),
                finished_at: None,
                edition_id: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(r.id, 6413991);
    assert_eq!(r.progress_pages, Some(50));
    assert!((r.progress.unwrap() - 17.36).abs() < 0.01);
}

#[tokio::test]
async fn updating_a_read_resends_existing_dates_so_upstream_does_not_wipe_them() {
    let s = server().await;
    // The test asserts the *request*: started_at from the existing read must be included.
    respond(&s, serde_json::json!({"variables": {"id": 6413991, "read": {"progress_pages": 120, "started_at": "2026-08-21"}}}), "write_update_read.json").await;
    let existing = hardcover_api::model::Read {
        id: 6413991,
        started_at: Some("2026-08-21".into()),
        finished_at: None,
        paused_at: None,
        progress: Some(17.36),
        progress_pages: Some(50),
        progress_seconds: None,
        edition_id: Some(25224958),
    };

    let r = client(&s)
        .read_update(
            &existing,
            ProgressUpdate {
                pages: Some(120),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(r.progress_pages, Some(120));
    assert_eq!(
        s.received_requests().await.unwrap().len(),
        1,
        "no match means dates were dropped"
    );
}

#[tokio::test]
async fn removes_an_entry() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"id": 17692741}}),
        "write_delete.json",
    )
    .await;
    client(&s).library_remove(17692741).await.unwrap();
}
