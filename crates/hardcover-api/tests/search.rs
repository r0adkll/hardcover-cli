mod common;
use common::*;
use hardcover_api::model::SearchType;

#[tokio::test]
async fn searches_books_and_exposes_stable_id_slug_label_plus_raw_document() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"query": "dune", "query_type": "book"}}), "search_book_dune.json").await;

    let r = client(&s).search("dune", SearchType::Book, 1, 3).await.unwrap();

    assert_eq!(r.query, "dune");
    assert_eq!(r.query_type, SearchType::Book);
    assert_eq!(r.page, 1);
    assert_eq!(r.per_page, 3);
    assert_eq!(r.found, 840);
    assert_eq!(r.hits.len(), 3);
    assert_eq!(r.hits[0].id, 312460);
    assert_eq!(r.hits[0].slug.as_deref(), Some("dune"));
    assert_eq!(r.hits[0].label, "Dune");
    assert_eq!(r.hits[0].document["author_names"][0], "Frank Herbert");
}

#[tokio::test]
async fn searches_authors_using_name_as_label() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"query_type": "author"}}), "search_author_sanderson.json").await;

    let r = client(&s).search("sanderson", SearchType::Author, 1, 3).await.unwrap();

    assert_eq!(r.hits[0].id, 204214);
    assert_eq!(r.hits[0].label, "Brandon Sanderson");
}
