use std::path::{Path, PathBuf};

use async_trait::async_trait;
use uuid::Uuid;

use agent_kit_core::blob::{BlobError, BlobMeta, BlobObject, BlobStore, PutBlob, check_blob_size};
use agent_kit_core::config::BlobBackendKind;

use crate::error::StoreError;
use crate::sql::{self, InsertBlob};

pub(crate) struct FsBlobStore {
    pool: sqlx::PgPool,
    root: PathBuf,
}

impl FsBlobStore {
    pub(crate) fn new(pool: sqlx::PgPool, root: PathBuf) -> Self {
        Self { pool, root }
    }

    async fn put_inner(&self, request: PutBlob) -> Result<BlobMeta, StoreError> {
        check_blob_size(request.bytes.len()).map_err(|_| StoreError::TooLarge)?;
        let id = Uuid::new_v4();
        let storage_key = id.to_string();
        let path = fs_blob_path(&self.root, &storage_key)?;
        let byte_size = i64::try_from(request.bytes.len()).map_err(|_| StoreError::TooLarge)?;
        tokio::fs::create_dir_all(&self.root).await?;
        if let Err(err) = tokio::fs::write(&path, &request.bytes).await {
            let _ = tokio::fs::remove_file(&path).await;
            return Err(err.into());
        }
        match sql::insert_meta(
            &self.pool,
            &InsertBlob {
                id,
                owner_user_id: request.owner_user_id,
                content_type: &request.content_type,
                byte_size,
                storage_backend: BlobBackendKind::Fs.as_str(),
                storage_key: &storage_key,
            },
        )
        .await
        {
            Ok(meta) => Ok(meta),
            Err(err) => {
                let _ = tokio::fs::remove_file(&path).await;
                Err(err)
            }
        }
    }

    async fn get_inner(&self, id: Uuid) -> Result<BlobObject, StoreError> {
        let row = sql::load_meta(&self.pool, id).await?;
        if row.storage_backend != BlobBackendKind::Fs.as_str() {
            return Err(StoreError::Storage(format!(
                "blob {id} is stored in {}",
                row.storage_backend
            )));
        }
        let path = fs_blob_path(&self.root, &row.storage_key)?;
        let bytes = match tokio::fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(StoreError::Storage("blob file missing".to_string()));
            }
            Err(err) => return Err(err.into()),
        };
        Ok(sql::into_object(row.into_meta()?, bytes))
    }

    async fn delete_inner(&self, id: Uuid) -> Result<(), StoreError> {
        let removed = sql::delete_meta(&self.pool, id).await?;
        let path = fs_blob_path(&self.root, &id.to_string())?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        if removed {
            Ok(())
        } else {
            Err(StoreError::NotFound)
        }
    }
}

pub(crate) fn fs_blob_path(root: &Path, storage_key: &str) -> Result<PathBuf, StoreError> {
    let id = Uuid::parse_str(storage_key)
        .map_err(|_| StoreError::Storage("invalid blob storage key".to_string()))?;
    Ok(root.join(id.to_string()))
}

#[async_trait]
impl BlobStore for FsBlobStore {
    async fn put(&self, request: PutBlob) -> Result<BlobMeta, BlobError> {
        Ok(self.put_inner(request).await?)
    }

    async fn get(&self, id: Uuid) -> Result<BlobObject, BlobError> {
        Ok(self.get_inner(id).await?)
    }

    async fn delete(&self, id: Uuid) -> Result<(), BlobError> {
        Ok(self.delete_inner(id).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::fs_blob_path;

    #[test]
    fn fs_blob_path_joins_uuid_under_root() {
        let root = Path::new("/var/lib/agent-kit/blobs");
        let id = "11111111-2222-3333-4444-555555555555";
        let path = fs_blob_path(root, id).unwrap();
        assert_eq!(path, root.join(id));
        assert!(path.starts_with(root));

        let urn = "urn:uuid:11111111-2222-3333-4444-555555555555";
        assert_eq!(fs_blob_path(root, urn).unwrap(), root.join(id));

        assert!(fs_blob_path(root, "../etc/passwd").is_err());
        assert!(fs_blob_path(root, &format!("{id}/x")).is_err());
        assert!(fs_blob_path(root, "").is_err());
    }
}
