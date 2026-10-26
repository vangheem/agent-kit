use chrono::{DateTime, Utc};
use uuid::Uuid;

pub use crate::error::BlobError;

pub const MAX_BLOB_BYTES: usize = 10 * 1024 * 1024;

pub fn check_blob_size(len: usize) -> Result<(), BlobError> {
    if len > MAX_BLOB_BYTES {
        Err(BlobError::TooLarge)
    } else {
        Ok(())
    }
}

pub struct PutBlob {
    pub owner_user_id: Uuid,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for PutBlob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PutBlob")
            .field("owner_user_id", &self.owner_user_id)
            .field("content_type", &self.content_type)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobMeta {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub content_type: String,
    pub byte_size: i64,
    pub storage_backend: crate::config::BlobBackendKind,
    pub storage_key: String,
    pub created_at: DateTime<Utc>,
}

pub struct BlobObject {
    pub meta: BlobMeta,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for BlobObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobObject")
            .field("meta", &self.meta)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

#[async_trait::async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, request: PutBlob) -> Result<BlobMeta, BlobError>;
    async fn get(&self, id: Uuid) -> Result<BlobObject, BlobError>;
    async fn delete(&self, id: Uuid) -> Result<(), BlobError>;
}
