mod common;
use common::*;

#[tokio::test]
async fn review_from_flag_reports_before_and_after() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond_op(&s, "LibraryEntryQuery", "library_entry.json").await;
    respond_mut(&s, "UpdateUserBookReview", "review_write.json").await;

    let (code, json, stderr) = run(
        &s,
        &[
            "library",
            "review",
            "427678",
            "--text",
            "A **witty** set of stories.\n\nSecond paragraph.",
            "--format",
            "json",
        ],
    )
    .await;

    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["action"], "reviewed");
    assert_eq!(json["data"]["before"]["has_review"], false);
    assert_eq!(json["data"]["after"]["has_review"], true);
    assert!(json["data"]["after"]["review"]
        .as_str()
        .unwrap()
        .contains("**witty**"));
}

#[tokio::test]
async fn review_reads_stdin_when_no_text_flag() {
    use assert_cmd::Command;
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond_op(&s, "LibraryEntryQuery", "library_entry.json").await;
    respond_mut(&s, "UpdateUserBookReview", "review_write.json").await;
    let uri = s.uri();
    let out = tokio::task::spawn_blocking(move || {
        Command::cargo_bin("hardcover")
            .unwrap()
            .env("HARDCOVER_TOKEN", "test-token")
            .env("HARDCOVER_API_URL", uri)
            .env("HARDCOVER_KEYRING", "mock")
            .args([
                "library",
                "review",
                "427678",
                "--spoilers",
                "--format",
                "json",
            ])
            .write_stdin("From stdin.\n")
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = s
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|r| String::from_utf8_lossy(&r.body).into_owned())
        .find(|b| b.contains("UpdateUserBookReview"))
        .unwrap();
    assert!(body.contains("From stdin."), "{body}");
    assert!(body.contains("\"spoilers\":true"), "{body}");
}

#[tokio::test]
async fn review_requires_an_existing_library_entry_and_supports_dry_run() {
    let s = server().await;
    respond_op(&s, "Me", "me.json").await;
    respond_op(&s, "LibraryEntryQuery", "library_entry_missing.json").await;

    let (code, _, stderr) = run(&s, &["library", "review", "1", "--text", "x"]).await;
    assert_eq!(code, Some(4), "{stderr}");

    let s2 = server().await;
    respond_op(&s2, "Me", "me.json").await;
    respond_op(&s2, "LibraryEntryQuery", "library_entry.json").await;
    let (code, json, stderr) = run(
        &s2,
        &[
            "library",
            "review",
            "427678",
            "--text",
            "x",
            "--dry-run",
            "--format",
            "json",
        ],
    )
    .await;
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(json["data"]["planned"]["review"], "x");
    assert!(!s2
        .received_requests()
        .await
        .unwrap()
        .iter()
        .any(|r| String::from_utf8_lossy(&r.body).contains("mutation")));
}
