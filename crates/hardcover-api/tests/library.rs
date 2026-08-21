mod common;
use common::*;
use hardcover_api::model::{LibraryFilter, ReadingStatus};
use hardcover_api::{Error, Page};

#[tokio::test]
async fn lists_my_library_most_recently_updated_first() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond(
        &s,
        serde_json::json!({"variables": {"user_id": 31899, "limit": 3, "offset": 0}}),
        "library_page.json",
    )
    .await;

    let entries = client(&s)
        .library(
            LibraryFilter::default(),
            Page {
                limit: 3,
                offset: 0,
            },
        )
        .await
        .unwrap();

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].book.title, "Dark Age");
    assert_eq!(entries[0].status, ReadingStatus::CurrentlyReading);
    assert_eq!(entries[0].status_id, 2);
    assert_eq!(entries[1].rating, Some(3.5));
    assert_eq!(entries[1].privacy, "public");
    assert!(!entries[0].owned);
}

#[tokio::test]
async fn filters_library_by_status() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond(
        &s,
        serde_json::json!({"variables": {"user_id": 31899, "status_ids": [3]}}),
        "library_page_read.json",
    )
    .await;

    let f = LibraryFilter {
        status: Some(ReadingStatus::Read),
        ..Default::default()
    };
    let entries = client(&s)
        .library(
            f,
            Page {
                limit: 3,
                offset: 0,
            },
        )
        .await
        .unwrap();

    assert!(entries.iter().all(|e| e.status == ReadingStatus::Read));
}

#[tokio::test]
async fn shows_my_entry_for_a_book_with_reads() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond(
        &s,
        serde_json::json!({"variables": {"user_id": 31899, "book_id": 427678}}),
        "library_entry.json",
    )
    .await;

    let e = client(&s).library_entry(427678).await.unwrap();

    assert_eq!(e.entry.book.slug, "iron-gold");
    assert_eq!(e.entry.status, ReadingStatus::Read);
    assert_eq!(e.entry.read_count, 1);
    assert_eq!(e.reads.len(), 2);
    assert_eq!(e.reads[1].finished_at.as_deref(), Some("2026-08-18"));
    assert_eq!(e.reads[1].progress_pages, Some(624));
    assert_eq!(e.reads[1].progress, Some(100.0));
}

#[tokio::test]
async fn book_not_in_library_is_not_found() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond(
        &s,
        serde_json::json!({"variables": {"book_id": 1}}),
        "library_entry_missing.json",
    )
    .await;

    assert!(matches!(
        client(&s).library_entry(1).await.unwrap_err(),
        Error::NotFound(_)
    ));
}

#[tokio::test]
async fn caches_the_current_user_id_across_calls() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond(
        &s,
        serde_json::json!({"variables": {"user_id": 31899}}),
        "library_page.json",
    )
    .await;
    let c = client(&s);

    c.library(
        LibraryFilter::default(),
        Page {
            limit: 3,
            offset: 0,
        },
    )
    .await
    .unwrap();
    c.library(
        LibraryFilter::default(),
        Page {
            limit: 3,
            offset: 3,
        },
    )
    .await
    .unwrap();

    let me_calls = s
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| String::from_utf8_lossy(&r.body).contains("query Me"))
        .count();
    assert_eq!(me_calls, 1);
}

#[test]
fn reading_status_round_trips_names_and_ids() {
    for (id, name) in [
        (1, "want_to_read"),
        (2, "currently_reading"),
        (3, "read"),
        (4, "paused"),
        (5, "did_not_finish"),
        (6, "ignored"),
    ] {
        let s = ReadingStatus::from_id(id).unwrap();
        assert_eq!(s.as_str(), name);
        assert_eq!(s.id(), id);
        assert_eq!(name.parse::<ReadingStatus>().unwrap(), s);
    }
    assert_eq!(
        "reading".parse::<ReadingStatus>().unwrap(),
        ReadingStatus::CurrentlyReading
    );
    assert_eq!(
        "dnf".parse::<ReadingStatus>().unwrap(),
        ReadingStatus::DidNotFinish
    );
    assert_eq!(
        "want".parse::<ReadingStatus>().unwrap(),
        ReadingStatus::WantToRead
    );
    assert!(ReadingStatus::from_id(99).is_none());
}
