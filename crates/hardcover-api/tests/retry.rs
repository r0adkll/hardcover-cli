mod common;
use common::*;
use hardcover_api::{Client, Error, RetryPolicy};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn rate_limited_then_ok(s: &MockServer, failures: u64) {
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
        .up_to_n_times(failures)
        .mount(s)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(fixture("me.json"), "application/json"),
        )
        .mount(s)
        .await;
}

#[tokio::test]
async fn retries_after_429_honouring_retry_after() {
    let s = server().await;
    rate_limited_then_ok(&s, 2).await;
    let c = Client::builder("test-token")
        .base_url(s.uri())
        .retry(RetryPolicy {
            max_attempts: 3,
            ..RetryPolicy::default()
        })
        .build();

    let me = c.me().await.unwrap();

    assert_eq!(me.username, "r0adkll");
    assert_eq!(s.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn gives_up_after_max_attempts() {
    let s = server().await;
    rate_limited_then_ok(&s, 5).await;
    let c = Client::builder("test-token")
        .base_url(s.uri())
        .retry(RetryPolicy {
            max_attempts: 2,
            ..RetryPolicy::default()
        })
        .build();

    let err = c.me().await.unwrap_err();

    assert!(
        matches!(
            err,
            Error::RateLimited {
                retry_after_secs: Some(0)
            }
        ),
        "{err:?}"
    );
    assert_eq!(s.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn retry_can_be_disabled() {
    let s = server().await;
    rate_limited_then_ok(&s, 1).await;
    let c = Client::builder("test-token")
        .base_url(s.uri())
        .retry(RetryPolicy::none())
        .build();

    assert!(matches!(
        c.me().await.unwrap_err(),
        Error::RateLimited { .. }
    ));
    assert_eq!(s.received_requests().await.unwrap().len(), 1);
}
