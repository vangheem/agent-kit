use std::fmt;

use agent_kit_core::blob::BlobError;

#[derive(Debug)]
pub(crate) enum StoreError {
    TooLarge,
    NotFound,
    Storage(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => write!(f, "blob exceeds 10 MiB"),
            Self::NotFound => write!(f, "blob not found"),
            Self::Storage(message) => write!(f, "{message}"),
        }
    }
}

impl From<sqlx::Error> for StoreError {
    fn from(err: sqlx::Error) -> Self {
        Self::Storage(err.to_string())
    }
}

impl From<std::io::Error> for StoreError {
    fn from(err: std::io::Error) -> Self {
        Self::Storage(err.to_string())
    }
}

impl From<StoreError> for BlobError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::TooLarge => BlobError::TooLarge,
            StoreError::NotFound => BlobError::NotFound,
            StoreError::Storage(message) => BlobError::Internal { message },
        }
    }
}
