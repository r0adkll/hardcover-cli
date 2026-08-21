#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("invalid or expired token")]
    InvalidToken,
    #[error("token lacks required scope: {0}")]
    InsufficientScope(String),
    #[error("rate limited; retry after {retry_after_secs:?}s")]
    RateLimited { retry_after_secs: Option<u64> },
    #[error("not found: {0}")]
    NotFound(String),
    #[error("upstream error: {0}")]
    Upstream(String),
}

pub type Result<T> = std::result::Result<T, Error>;
