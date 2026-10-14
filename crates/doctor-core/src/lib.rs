pub mod diagnostics;
pub mod models;
pub mod network;
pub mod platform;
pub mod reports;
pub mod rules;
pub mod storage;
pub mod validation;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid input: {0}")]
    Invalid(String),
    #[error("Operation timed out")]
    Timeout,
    #[error("Operation cancelled")]
    Cancelled,
    #[error("Platform operation failed: {0}")]
    Platform(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Network operation failed: {0}")]
    Network(String),
}
pub type Result<T> = std::result::Result<T, Error>;

pub async fn bounded<T>(
    token: &tokio_util::sync::CancellationToken,
    duration: std::time::Duration,
    future: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tokio::select! {
        biased;
        _ = token.cancelled() => Err(Error::Cancelled),
        result = tokio::time::timeout(duration, future) => result.map_err(|_| Error::Timeout)?,
    }
}
