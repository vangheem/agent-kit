mod error;
mod fs;
mod pg;
mod sql;

use std::sync::Arc;

use agent_kit_core::blob::{BlobError, BlobStore};
use agent_kit_core::config::{AppConfig, BlobBackendKind};

use crate::error::StoreError;

pub async fn connect(
    config: &AppConfig,
    pool: &sqlx::PgPool,
) -> Result<Arc<dyn BlobStore>, BlobError> {
    match &config.blob_storage_type {
        BlobBackendKind::Pg => Ok(Arc::new(pg::PgBlobStore::new(pool.clone()))),
        BlobBackendKind::Fs => {
            tokio::fs::create_dir_all(&config.blob_fs_root)
                .await
                .map_err(StoreError::from)?;
            Ok(Arc::new(fs::FsBlobStore::new(
                pool.clone(),
                config.blob_fs_root.clone(),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fs::FsBlobStore;
    use super::pg::PgBlobStore;

    use agent_kit_core::blob::{BlobError, BlobStore, MAX_BLOB_BYTES, PutBlob};
    use uuid::Uuid;

    fn lazy_pool() -> sqlx::PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://agentkit:agentkit@127.0.0.1:1/agentkit")
            .expect("lazy pool")
    }

    fn oversized() -> PutBlob {
        PutBlob {
            owner_user_id: Uuid::nil(),
            content_type: "application/octet-stream".to_string(),
            bytes: vec![0; MAX_BLOB_BYTES + 1],
        }
    }

    #[tokio::test]
    async fn put_rejects_payloads_over_10_mib_before_storage() {
        let pool = lazy_pool();
        let dir = std::env::temp_dir().join(format!("agent-kit-blob-{}", Uuid::new_v4()));
        assert!(!dir.exists());

        let fs_err = FsBlobStore::new(pool.clone(), dir.clone())
            .put(oversized())
            .await
            .expect_err("oversize fs put");
        assert!(matches!(fs_err, BlobError::TooLarge));
        assert!(!dir.exists());

        let pg_err = PgBlobStore::new(pool)
            .put(oversized())
            .await
            .expect_err("oversize pg put");
        assert!(matches!(pg_err, BlobError::TooLarge));
    }
}
