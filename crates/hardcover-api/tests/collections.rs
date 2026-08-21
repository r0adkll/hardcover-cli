mod common;
use common::*;
use hardcover_api::Page;

#[tokio::test]
async fn lists_an_authors_books_as_summaries() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"author_id": 154428, "limit": 3, "offset": 0}}), "author_books_page.json").await;

    let books = client(&s).author_books(154428, Page { limit: 3, offset: 0 }).await.unwrap();

    assert_eq!(books.len(), 3);
    assert_eq!(books[0].id, 442321);
    assert_eq!(books[0].slug, "whose-body");
    assert_eq!(books[0].title, "Whose Body?");
    assert_eq!(books[0].release_year, Some(1923));
    assert!(books[0].cover_url.as_deref().unwrap_or("").starts_with("https://"));
}

#[tokio::test]
async fn lists_series_books_with_positions() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"series_id": 6572}}), "series_books_page.json").await;

    let entries = client(&s).series_books(6572, Page { limit: 3, offset: 0 }).await.unwrap();

    assert_eq!(entries[0].position, Some(0.0));
    assert_eq!(entries[0].book.id, 1);
    assert_eq!(entries[0].book.title, "Lord Peter Views the Body");
}

#[tokio::test]
async fn lists_a_books_editions() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"book_id": 1}}), "book_editions_page.json").await;

    let editions = client(&s).book_editions(1, Page { limit: 3, offset: 0 }).await.unwrap();

    assert_eq!(editions[0].id, 759160);
    assert_eq!(editions[0].book_id, 1);
    assert_eq!(editions[0].isbn_10.as_deref(), Some("0060923954"));
    assert_eq!(editions[0].format.as_deref(), Some("physical"));
}

#[tokio::test]
async fn lists_a_lists_books() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"list_id": 301791}}), "list_books_page.json").await;

    let entries = client(&s).list_books(301791, Page { limit: 3, offset: 0 }).await.unwrap();

    assert_eq!(entries[0].book.id, 644433);
}
