mod common;
use common::*;
use hardcover_api::model::{BookIdentifier, ResolvedBy};
use hardcover_api::Error;

#[tokio::test]
async fn resolves_slug_to_book_id() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"slug": "lord-peter-views-the-body"}}),
        "book_id_by_slug.json",
    )
    .await;

    let r = client(&s)
        .resolve_book(&BookIdentifier::Slug("lord-peter-views-the-body".into()))
        .await
        .unwrap();

    assert_eq!(r.id, 1);
    assert_eq!(r.resolved_by, ResolvedBy::Slug);
}

#[tokio::test]
async fn resolves_isbn_via_edition_to_book_id() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"isbn": "9780441172719"}}),
        "book_id_by_isbn.json",
    )
    .await;

    let r = client(&s)
        .resolve_book(&BookIdentifier::Isbn("9780441172719".into()))
        .await
        .unwrap();

    assert_eq!(r.id, 312460);
    assert_eq!(r.resolved_by, ResolvedBy::Isbn);
}

#[tokio::test]
async fn unknown_slug_is_not_found() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"slug": "no-such-slug-xyz"}}),
        "book_id_by_slug_missing.json",
    )
    .await;

    let err = client(&s)
        .resolve_book(&BookIdentifier::Slug("no-such-slug-xyz".into()))
        .await
        .unwrap_err();

    assert!(matches!(err, Error::NotFound(_)), "{err:?}");
}

#[test]
fn parses_identifier_forms() {
    assert_eq!(
        "42".parse::<BookIdentifier>().unwrap(),
        BookIdentifier::Id(42)
    );
    assert_eq!(
        "dune".parse::<BookIdentifier>().unwrap(),
        BookIdentifier::Slug("dune".into())
    );
    assert_eq!(
        "978-0-441-17271-9".parse::<BookIdentifier>().unwrap(),
        BookIdentifier::Isbn("9780441172719".into())
    );
    assert_eq!(
        "0441172717".parse::<BookIdentifier>().unwrap(),
        BookIdentifier::Isbn("0441172717".into())
    );
    assert_eq!(
        "isbn:0441172717".parse::<BookIdentifier>().unwrap(),
        BookIdentifier::Isbn("0441172717".into())
    );
    assert_eq!(
        "slug:12345".parse::<BookIdentifier>().unwrap(),
        BookIdentifier::Slug("12345".into())
    );
    assert_eq!(
        "id:12345".parse::<BookIdentifier>().unwrap(),
        BookIdentifier::Id(12345)
    );
}
