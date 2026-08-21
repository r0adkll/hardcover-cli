use hardcover_api::Client;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn server_with(fixture: &str) -> MockServer {
    let body = std::fs::read_to_string(format!("tests/fixtures/{fixture}")).unwrap();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn fetches_a_book_by_id_with_contributors_and_series() {
    let server = server_with("book_by_pk_1.json").await;
    let client = Client::builder("test-token").base_url(server.uri()).build();

    let book = client.book(1).await.unwrap();

    assert_eq!(book.id, 1);
    assert_eq!(book.slug, "lord-peter-views-the-body");
    assert_eq!(book.title, "Lord Peter Views the Body");
    assert_eq!(book.pages, Some(288));
    assert_eq!(book.users_count, 55);
    assert_eq!(
        book.cover_url.as_deref(),
        Some("https://assets.hardcover.app/external_data/60951398/fcce59736371385c33cab5dd4b5fc478fac17dd6.jpeg")
    );
    assert_eq!(book.contributors.len(), 1);
    assert_eq!(book.contributors[0].name, "Dorothy L. Sayers");
    assert_eq!(book.contributors[0].slug, "dorothy-l-sayers");
    assert_eq!(book.series.len(), 1);
    assert_eq!(book.series[0].name, "Lord Peter Wimsey");
    assert_eq!(book.series[0].position, Some(0.0));
}
