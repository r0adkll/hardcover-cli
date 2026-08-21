mod common;
use common::*;

#[tokio::test]
async fn table_format_renders_aligned_columns_for_collections() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"author_id": 154428}}),
        "author_books_page.json",
    )
    .await;

    let out = run_raw(
        &s,
        &[
            "author", "books", "154428", "--limit", "3", "--format", "table",
        ],
    )
    .await;

    let lines: Vec<&str> = out.lines().collect();
    // header row names the columns in struct order, then one row per item
    let header = lines.iter().find(|l| l.contains("title")).unwrap();
    assert!(
        header.contains("id") && header.contains("release_year"),
        "{out}"
    );
    assert!(
        header.find("id").unwrap() < header.find("title").unwrap(),
        "field order preserved: {header}"
    );
    assert!(out.contains("Whose Body?"), "{out}");
    assert!(out.contains("442321"), "{out}");
    // nested objects are flattened to their most useful scalar, not dumped as JSON
    assert!(!out.contains('{'), "no raw json in table:\n{out}");
    // exactly 3 data rows
    let data_rows = lines
        .iter()
        .filter(|l| l.contains("#") || l.chars().any(|c| c.is_ascii_digit()))
        .count();
    assert!(data_rows >= 3, "{out}");
}

#[tokio::test]
async fn table_format_renders_a_single_entity_as_key_value_rows() {
    let s = server().await;
    respond(
        &s,
        serde_json::json!({"variables": {"id": 1}}),
        "book_by_pk_1.json",
    )
    .await;

    let out = run_raw(&s, &["book", "show", "1", "--format", "table"]).await;

    assert!(
        out.contains("title") && out.contains("Lord Peter Views the Body"),
        "{out}"
    );
    assert!(
        out.contains("contributors") && out.contains("Dorothy L. Sayers"),
        "{out}"
    );
    assert!(!out.contains("{\"id\""), "no raw json:\n{out}");
}
