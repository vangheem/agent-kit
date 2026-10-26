use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use agent_kit_core::blob::{BlobMeta, BlobObject};
use agent_kit_core::config::BlobBackendKind;

use crate::error::StoreError;

pub(crate) struct InsertBlob<'a> {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub content_type: &'a str,
    pub byte_size: i64,
    pub storage_backend: &'a str,
    pub storage_key: &'a str,
}

pub(crate) struct MetaRow {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub content_type: String,
    pub byte_size: i64,
    pub storage_backend: String,
    pub storage_key: String,
    pub created_at: DateTime<Utc>,
}

impl MetaRow {
    pub(crate) fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: row.try_get("id")?,
            owner_user_id: row.try_get("owner_user_id")?,
            content_type: row.try_get("content_type")?,
            byte_size: row.try_get("byte_size")?,
            storage_backend: row.try_get("storage_backend")?,
            storage_key: row.try_get("storage_key")?,
            created_at: row.try_get("created_at")?,
        })
    }

    pub(crate) fn into_meta(self) -> Result<BlobMeta, StoreError> {
        let storage_backend = BlobBackendKind::parse(&self.storage_backend).ok_or_else(|| {
            StoreError::Storage(format!("unknown storage backend {}", self.storage_backend))
        })?;
        Ok(BlobMeta {
            id: self.id,
            owner_user_id: self.owner_user_id,
            content_type: self.content_type,
            byte_size: self.byte_size,
            storage_backend,
            storage_key: self.storage_key,
            created_at: self.created_at,
        })
    }
}

pub(crate) fn into_object(meta: BlobMeta, bytes: Vec<u8>) -> BlobObject {
    BlobObject { meta, bytes }
}

const INSERT_META: &str = "\
INSERT INTO blobs (id, owner_user_id, content_type, byte_size, storage_backend, storage_key)
VALUES ($1, $2, $3, $4, $5, $6)
RETURNING id, owner_user_id, content_type, byte_size, storage_backend, storage_key, created_at";

pub(crate) async fn insert_meta<'e, E>(
    executor: E,
    blob: &InsertBlob<'_>,
) -> Result<BlobMeta, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query(INSERT_META)
        .bind(blob.id)
        .bind(blob.owner_user_id)
        .bind(blob.content_type)
        .bind(blob.byte_size)
        .bind(blob.storage_backend)
        .bind(blob.storage_key)
        .fetch_one(executor)
        .await?;
    MetaRow::from_row(&row)?.into_meta()
}

pub(crate) async fn load_meta(pool: &sqlx::PgPool, id: Uuid) -> Result<MetaRow, StoreError> {
    let row = sqlx::query(
        "SELECT id, owner_user_id, content_type, byte_size, storage_backend, storage_key, created_at
         FROM blobs
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(row) => MetaRow::from_row(&row),
        None => Err(StoreError::NotFound),
    }
}

pub(crate) async fn delete_meta(pool: &sqlx::PgPool, id: Uuid) -> Result<bool, StoreError> {
    let result = sqlx::query("DELETE FROM blobs WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
