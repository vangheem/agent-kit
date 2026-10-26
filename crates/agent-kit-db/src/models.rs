use agent_kit_core::BlobBackendKind;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("email", &self.email)
            .field("password_hash", &"<redacted>")
            .field("created_at", &self.created_at)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct McpClient {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub token_prefix: String,
    pub enabled_actions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct ServiceConnectionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub provider: String,
    pub credentials_ciphertext: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for ServiceConnectionRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceConnectionRow")
            .field("id", &self.id)
            .field("user_id", &self.user_id)
            .field("kind", &self.kind)
            .field("provider", &self.provider)
            .field("credentials_ciphertext", &"<redacted>")
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewActionRun {
    pub mcp_client_id: Uuid,
    pub action_id: String,
    pub status: String,
    pub input_blob_id: Option<Uuid>,
    pub output_blob_id: Option<Uuid>,
    pub error_code: Option<String>,
    pub duration_ms: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewBlobMeta {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub content_type: String,
    pub byte_size: i64,
    pub storage_backend: BlobBackendKind,
    pub storage_key: String,
}

#[derive(sqlx::FromRow)]
pub(crate) struct BlobMetaRow {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub content_type: String,
    pub byte_size: i64,
    pub storage_backend: String,
    pub storage_key: String,
    pub created_at: DateTime<Utc>,
}
