use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use agent_kit_core::blob::{BlobError, BlobMeta, BlobObject, BlobStore, PutBlob, check_blob_size};
use agent_kit_core::config::BlobBackendKind;

use crate::error::StoreError;
use crate::sql::{self, InsertBlob};

pub(crate) struct PgBlobStore {
    pool: sqlx::PgPool,
}

impl PgBlobStore {
    pub(crate) fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    async fn put_inner(&self, request: PutBlob) -> Result<BlobMeta, StoreError> {
        check_blob_size(request.bytes.len()).map_err(|_| StoreError::TooLarge)?;
        let id = Uuid::new_v4();
        let storage_key = id.to_string();
        let byte_size = i64::try_from(request.bytes.len()).map_err(|_| StoreError::TooLarge)?;
        let mut tx = self.pool.begin().await?;
        let meta = sql::insert_meta(
            &mut *tx,
            &InsertBlob {
                id,
                owner_user_id: request.owner_user_id,
                content_type: &request.content_type,
                byte_size,
                storage_backend: BlobBackendKind::Pg.as_str(),
                storage_key: &storage_key,
            },
        )
        .await?;
        sqlx::query("INSERT INTO blob_payloads (blob_id, data) VALUES ($1, $2)")
            .bind(id)
            .bind(&request.bytes)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(meta)
    }

    async fn get_inner(&self, id: Uuid) -> Result<BlobObject, StoreError> {
        let row = sqlx::query(
            "SELECT b.id, b.owner_user_id, b.content_type, b.byte_size, b.storage_backend, b.storage_key, b.created_at, p.data
             FROM blobs b
             INNER JOIN blob_payloads p ON p.blob_id = b.id
             WHERE b.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Err(StoreError::NotFound);
        };
        let bytes: Vec<u8> = row.try_get("data")?;
        let meta = sql::MetaRow::from_row(&row)?.into_meta()?;
        Ok(sql::into_object(meta, bytes))
    }

    async fn delete_inner(&self, id: Uuid) -> Result<(), StoreError> {
        if sql::delete_meta(&self.pool, id).await? {
            Ok(())
        } else {
            Err(StoreError::NotFound)
        }
    }
}

#[async_trait]
impl BlobStore for PgBlobStore {
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
