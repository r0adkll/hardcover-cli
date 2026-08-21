mod common;
use common::*;
use hardcover_api::model::Identifier;

#[tokio::test]
async fn shows_an_author() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"id": 154428}}), "author_by_pk.json").await;

    let a = client(&s).author(154428).await.unwrap();

    assert_eq!(a.slug, "dorothy-l-sayers");
    assert_eq!(a.name, "Dorothy L. Sayers");
    assert_eq!(a.books_count, 129);
    assert!(a.bio.unwrap().starts_with("An English crime writer"));
    assert_eq!(a.image_url.as_deref(), Some("https://assets.hardcover.app/authors/154428/6460155-L.jpg"));
}

#[tokio::test]
async fn shows_a_series_with_its_author() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"id": 6572}}), "series_by_pk.json").await;

    let x = client(&s).series(6572).await.unwrap();

    assert_eq!(x.name, "Lord Peter Wimsey");
    assert_eq!(x.books_count, 17);
    assert_eq!(x.primary_books_count, Some(15));
    assert_eq!(x.author.unwrap().slug, "dorothy-l-sayers");
}

#[tokio::test]
async fn shows_an_edition() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"id": 25224958}}), "edition_by_pk.json").await;

    let e = client(&s).edition(25224958).await.unwrap();

    assert_eq!(e.book_id, 1);
    assert_eq!(e.isbn_13.as_deref(), Some("9780450564741"));
    assert_eq!(e.language.as_deref(), Some("English"));
    assert_eq!(e.publisher.unwrap().name.as_deref(), Some("New English Library"));
}

#[tokio::test]
async fn shows_a_list_with_owner() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"id": 301791}}), "list_by_pk.json").await;

    let l = client(&s).list(301791).await.unwrap();

    assert_eq!(l.name, "Hardcover Author Spotlight");
    assert_eq!(l.slug.as_deref(), Some("hardcover-author-spotlight"));
    assert_eq!(l.books_count, 1);
    assert!(!l.owner.username.is_empty());
}

#[tokio::test]
async fn shows_a_prompt() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"id": 1}}), "prompt_by_pk.json").await;

    let p = client(&s).prompt(1).await.unwrap();

    assert_eq!(p.question, "What are your favorite books of all time?");
    assert_eq!(p.slug, "what-are-your-favorite-books-of-all-time");
}

#[tokio::test]
async fn shows_a_user_by_username() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"username": "r0adkll"}}), "user_by_username.json").await;

    let u = client(&s).user_by_username("r0adkll").await.unwrap();

    assert_eq!(u.id, 31899);
    assert_eq!(u.location.as_deref(), Some("South Carolina"));
    assert_eq!(u.books_count, 68);
}

#[tokio::test]
async fn unknown_username_is_not_found() {
    let s = server().await;
    respond(&s, serde_json::json!({"variables": {"username": "nobody"}}), "user_by_username_missing.json").await;
    assert!(matches!(client(&s).user_by_username("nobody").await.unwrap_err(), hardcover_api::Error::NotFound(_)));
}

#[tokio::test]
async fn resolves_author_series_list_and_prompt_slugs() {
    let s = server().await;
    respond_op(&s, "AuthorIdBySlug", "author_id_by_slug.json").await;
    respond_op(&s, "SeriesIdBySlug", "series_id_by_slug.json").await;
    respond_op(&s, "ListIdBySlug", "list_id_by_slug.json").await;
    respond_op(&s, "PromptIdBySlug", "prompt_id_by_slug.json").await;
    let c = client(&s);

    assert_eq!(c.resolve_author(&Identifier::Slug("dorothy-l-sayers".into())).await.unwrap().id, 154428);
    assert_eq!(c.resolve_series(&Identifier::Slug("lord-peter-wimsey".into())).await.unwrap().id, 6572);
    assert_eq!(c.resolve_list(&Identifier::Slug("hardcover-author-spotlight".into())).await.unwrap().id, 301791);
    assert_eq!(c.resolve_prompt(&Identifier::Slug("what-are-your-favorite-books-of-all-time".into())).await.unwrap().id, 1);
    assert_eq!(c.resolve_author(&Identifier::Id(7)).await.unwrap().id, 7);
}

#[test]
fn generic_identifier_parses_ids_and_slugs() {
    assert_eq!("42".parse::<Identifier>().unwrap(), Identifier::Id(42));
    assert_eq!("dune".parse::<Identifier>().unwrap(), Identifier::Slug("dune".into()));
    assert_eq!("slug:42".parse::<Identifier>().unwrap(), Identifier::Slug("42".into()));
}
