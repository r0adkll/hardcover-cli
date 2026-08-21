mod common;
use common::*;

#[tokio::test]
async fn author_show_by_slug() {
    let s = server().await;
    respond_op(&s, "AuthorIdBySlug", "author_id_by_slug.json").await;
    respond_op(&s, "AuthorById", "author_by_pk.json").await;

    let (code, json, stderr) = run(&s, &["author", "show", "dorothy-l-sayers", "--format", "json"]).await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["name"], "Dorothy L. Sayers");
    assert_eq!(json["meta"]["resolved_by"], "slug");
}

#[tokio::test]
async fn series_edition_list_prompt_and_user_show() {
    let s = server().await;
    respond_op(&s, "SeriesById", "series_by_pk.json").await;
    respond_op(&s, "EditionById", "edition_by_pk.json").await;
    respond_op(&s, "ListById", "list_by_pk.json").await;
    respond_op(&s, "PromptById", "prompt_by_pk.json").await;
    respond_op(&s, "UserByUsername", "user_by_username.json").await;

    let (_, j, e) = run(&s, &["series", "show", "6572", "--format", "json"]).await;
    assert_eq!(j["data"]["name"], "Lord Peter Wimsey", "{e}");
    let (_, j, e) = run(&s, &["edition", "show", "25224958", "--format", "json"]).await;
    assert_eq!(j["data"]["isbn_13"], "9780450564741", "{e}");
    let (_, j, e) = run(&s, &["list", "show", "301791", "--format", "json"]).await;
    assert_eq!(j["data"]["name"], "Hardcover Author Spotlight", "{e}");
    let (_, j, e) = run(&s, &["prompt", "show", "1", "--format", "json"]).await;
    assert_eq!(j["data"]["slug"], "what-are-your-favorite-books-of-all-time", "{e}");
    let (_, j, e) = run(&s, &["user", "show", "r0adkll", "--format", "json"]).await;
    assert_eq!(j["data"]["id"], 31899, "{e}");
}
