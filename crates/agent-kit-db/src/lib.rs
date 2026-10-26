mod error;
mod models;

use agent_kit_core::{BlobBackendKind, BlobMeta};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub use error::DbError;
pub use models::{McpClient, NewActionRun, NewBlobMeta, ServiceConnectionRow, User};

const USER_RETURNING: &str = "id, email::text AS email, password_hash, created_at";
const MCP_CLIENT_RETURNING: &str =
    "id, user_id, name, token_prefix, enabled_actions, created_at, revoked_at";
const SERVICE_RETURNING: &str =
    "id, user_id, kind, provider, credentials_ciphertext, created_at, updated_at";
const BLOB_RETURNING: &str =
    "id, owner_user_id, content_type, byte_size, storage_backend, storage_key, created_at";

#[derive(Clone)]
pub struct Db {
    pool: sqlx::PgPool,
}

impl Db {
    pub async fn connect(database_url: &str) -> Result<Self, DbError> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), DbError> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }

    pub async fn create_user(&self, email: &str, password_hash: &str) -> Result<User, DbError> {
        let user = sqlx::query_as::<_, User>(&format!(
            "INSERT INTO users (email, password_hash) VALUES ($1::citext, $2) RETURNING {USER_RETURNING}"
        ))
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn user_by_email(&self, email: &str) -> Result<Option<User>, DbError> {
        let user = sqlx::query_as::<_, User>(&format!(
            "SELECT {USER_RETURNING} FROM users WHERE email = $1::citext"
        ))
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn user_by_id(&self, id: Uuid) -> Result<Option<User>, DbError> {
        let user =
            sqlx::query_as::<_, User>(&format!("SELECT {USER_RETURNING} FROM users WHERE id = $1"))
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(user)
    }

    pub async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query("INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(token_hash)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn session_user(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<User>, DbError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT u.id, u.email::text AS email, u.password_hash, u.created_at
             FROM sessions s
             JOIN users u ON u.id = s.user_id
             WHERE s.token_hash = $1 AND s.expires_at > $2",
        )
        .bind(token_hash)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn delete_session(&self, token_hash: &str) -> Result<(), DbError> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_mcp_client(
        &self,
        user_id: Uuid,
        name: &str,
        token_hash: &str,
        token_prefix: &str,
        enabled_actions: &[String],
    ) -> Result<McpClient, DbError> {
        let client = sqlx::query_as::<_, McpClient>(&format!(
            "INSERT INTO mcp_clients (user_id, name, token_hash, token_prefix, enabled_actions)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING {MCP_CLIENT_RETURNING}"
        ))
        .bind(user_id)
        .bind(name)
        .bind(token_hash)
        .bind(token_prefix)
        .bind(enabled_actions)
        .fetch_one(&self.pool)
        .await?;
        Ok(client)
    }

    pub async fn list_mcp_clients(&self, user_id: Uuid) -> Result<Vec<McpClient>, DbError> {
        let clients = sqlx::query_as::<_, McpClient>(&format!(
            "SELECT {MCP_CLIENT_RETURNING}
             FROM mcp_clients
             WHERE user_id = $1
             ORDER BY created_at DESC, id ASC"
        ))
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(clients)
    }

    pub async fn update_mcp_client(
        &self,
        user_id: Uuid,
        id: Uuid,
        name: &str,
        enabled_actions: &[String],
    ) -> Result<McpClient, DbError> {
        let client = sqlx::query_as::<_, McpClient>(&format!(
            "UPDATE mcp_clients
             SET name = $3, enabled_actions = $4
             WHERE user_id = $1 AND id = $2
             RETURNING {MCP_CLIENT_RETURNING}"
        ))
        .bind(user_id)
        .bind(id)
        .bind(name)
        .bind(enabled_actions)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DbError::NotFound)?;
        Ok(client)
    }

    pub async fn revoke_mcp_client(&self, user_id: Uuid, id: Uuid) -> Result<(), DbError> {
        let result = sqlx::query(
            "UPDATE mcp_clients
             SET revoked_at = COALESCE(revoked_at, now())
             WHERE user_id = $1 AND id = $2",
        )
        .bind(user_id)
        .bind(id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub async fn mcp_client_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<McpClient>, DbError> {
        let client = sqlx::query_as::<_, McpClient>(&format!(
            "SELECT {MCP_CLIENT_RETURNING}
             FROM mcp_clients
             WHERE token_hash = $1 AND revoked_at IS NULL"
        ))
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(client)
    }

    pub async fn upsert_service(
        &self,
        user_id: Uuid,
        kind: &str,
        provider: &str,
        credentials_ciphertext: &[u8],
    ) -> Result<ServiceConnectionRow, DbError> {
        let row = sqlx::query_as::<_, ServiceConnectionRow>(&format!(
            "INSERT INTO service_connections (user_id, kind, provider, credentials_ciphertext)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (user_id, kind) DO UPDATE
             SET provider = EXCLUDED.provider,
                 credentials_ciphertext = EXCLUDED.credentials_ciphertext,
                 updated_at = now()
             RETURNING {SERVICE_RETURNING}"
        ))
        .bind(user_id)
        .bind(kind)
        .bind(provider)
        .bind(credentials_ciphertext)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_services(&self, user_id: Uuid) -> Result<Vec<ServiceConnectionRow>, DbError> {
        let rows = sqlx::query_as::<_, ServiceConnectionRow>(&format!(
            "SELECT {SERVICE_RETURNING}
             FROM service_connections
             WHERE user_id = $1
             ORDER BY kind ASC"
        ))
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn service(
        &self,
        user_id: Uuid,
        kind: &str,
    ) -> Result<Option<ServiceConnectionRow>, DbError> {
        let row = sqlx::query_as::<_, ServiceConnectionRow>(&format!(
            "SELECT {SERVICE_RETURNING}
             FROM service_connections
             WHERE user_id = $1 AND kind = $2"
        ))
        .bind(user_id)
        .bind(kind)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn delete_service(&self, user_id: Uuid, kind: &str) -> Result<(), DbError> {
        sqlx::query("DELETE FROM service_connections WHERE user_id = $1 AND kind = $2")
            .bind(user_id)
            .bind(kind)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert_action_run(&self, run: NewActionRun) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO action_runs (
                mcp_client_id, action_id, status, input_blob_id, output_blob_id, error_code, duration_ms
             ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(run.mcp_client_id)
        .bind(run.action_id)
        .bind(run.status)
        .bind(run.input_blob_id)
        .bind(run.output_blob_id)
        .bind(run.error_code)
        .bind(run.duration_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_blob_meta(&self, meta: NewBlobMeta) -> Result<BlobMeta, DbError> {
        let row = sqlx::query_as::<_, models::BlobMetaRow>(&format!(
            "INSERT INTO blobs (
                id, owner_user_id, content_type, byte_size, storage_backend, storage_key
             ) VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING {BLOB_RETURNING}"
        ))
        .bind(meta.id)
        .bind(meta.owner_user_id)
        .bind(meta.content_type)
        .bind(meta.byte_size)
        .bind(meta.storage_backend.as_str())
        .bind(meta.storage_key)
        .fetch_one(&self.pool)
        .await?;
        blob_meta_from_row(row)
    }

    pub async fn blob_meta(&self, id: Uuid) -> Result<Option<BlobMeta>, DbError> {
        let row = sqlx::query_as::<_, models::BlobMetaRow>(&format!(
            "SELECT {BLOB_RETURNING} FROM blobs WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(blob_meta_from_row).transpose()
    }

    pub async fn delete_blob_meta(&self, id: Uuid) -> Result<(), DbError> {
        let result = sqlx::query("DELETE FROM blobs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}

fn blob_meta_from_row(row: models::BlobMetaRow) -> Result<BlobMeta, DbError> {
    let storage_backend = BlobBackendKind::parse(&row.storage_backend).ok_or_else(|| {
        DbError::Internal(format!("unknown storage_backend {}", row.storage_backend))
    })?;
    Ok(BlobMeta {
        id: row.id,
        owner_user_id: row.owner_user_id,
        content_type: row.content_type,
        byte_size: row.byte_size,
        storage_backend,
        storage_key: row.storage_key,
        created_at: row.created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn postgres_conflict_prefix_and_lookups() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let db = Db::connect(&url).await.expect("connect");
        db.migrate().await.expect("migrate");

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let email = format!("Db-{nonce}@Example.com");
        let user = db.create_user(&email, "hash").await.expect("create user");
        let conflict = db
            .create_user(&email.to_lowercase(), "other")
            .await
            .expect_err("duplicate email");
        assert!(matches!(conflict, DbError::Conflict));

        let by_email = db
            .user_by_email(&email.to_uppercase())
            .await
            .expect("email lookup")
            .expect("user");
        assert_eq!(by_email.id, user.id);
        let by_id = db
            .user_by_id(user.id)
            .await
            .expect("id lookup")
            .expect("user");
        assert_eq!(by_id.email, user.email);

        let prefix = "tokprefx";
        assert_eq!(prefix.len(), 8);
        let token_hash = format!("hash-{nonce}");
        let enabled = vec!["web.search".to_string()];
        let client = db
            .create_mcp_client(user.id, "laptop", &token_hash, prefix, &enabled)
            .await
            .expect("client");
        assert_eq!(client.token_prefix, prefix);
        assert_eq!(client.enabled_actions, enabled);

        let listed = db.list_mcp_clients(user.id).await.expect("list clients");
        assert!(listed.iter().any(|row| row.id == client.id));

        let updated = db
            .update_mcp_client(user.id, client.id, "desktop", &enabled)
            .await
            .expect("update");
        assert_eq!(updated.name, "desktop");
        assert_eq!(updated.token_prefix, prefix);
        let missing = db
            .update_mcp_client(Uuid::from_u128(1), client.id, "nope", &[])
            .await
            .expect_err("foreign client");
        assert!(matches!(missing, DbError::NotFound));

        let loaded = db
            .mcp_client_by_token_hash(&token_hash)
            .await
            .expect("token lookup")
            .expect("client");
        assert_eq!(loaded.id, client.id);
        db.revoke_mcp_client(user.id, client.id)
            .await
            .expect("revoke");
        assert!(
            db.mcp_client_by_token_hash(&token_hash)
                .await
                .expect("revoked lookup")
                .is_none()
        );

        let expired_at = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
            .expect("ts")
            .with_timezone(&Utc);
        let now = DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
            .expect("ts")
            .with_timezone(&Utc);
        let session_hash = format!("sess-{nonce}");
        db.create_session(user.id, &session_hash, expired_at)
            .await
            .expect("expired session");
        assert!(
            db.session_user(&session_hash, now)
                .await
                .expect("expired lookup")
                .is_none()
        );

        let future = DateTime::parse_from_rfc3339("2099-01-01T00:00:00Z")
            .expect("ts")
            .with_timezone(&Utc);
        let live_hash = format!("live-{nonce}");
        db.create_session(user.id, &live_hash, future)
            .await
            .expect("live session");
        let session_user = db
            .session_user(&live_hash, now)
            .await
            .expect("live lookup")
            .expect("user");
        assert_eq!(session_user.id, user.id);
        db.delete_session(&live_hash).await.expect("logout");
        assert!(
            db.session_user(&live_hash, now)
                .await
                .expect("deleted session")
                .is_none()
        );

        let service = db
            .upsert_service(user.id, "shop", "mock", b"cipher")
            .await
            .expect("service");
        assert_eq!(service.provider, "mock");
        assert_eq!(service.credentials_ciphertext, b"cipher");
        let replaced = db
            .upsert_service(user.id, "shop", "mock", b"cipher2")
            .await
            .expect("upsert");
        assert_eq!(replaced.id, service.id);
        assert_eq!(replaced.credentials_ciphertext, b"cipher2");
        let listed_services = db.list_services(user.id).await.expect("list services");
        assert_eq!(listed_services.len(), 1);
        let loaded_service = db
            .service(user.id, "shop")
            .await
            .expect("service lookup")
            .expect("service");
        assert_eq!(loaded_service.credentials_ciphertext, b"cipher2");
        db.delete_service(user.id, "shop")
            .await
            .expect("delete service");
        assert!(db.service(user.id, "shop").await.expect("gone").is_none());

        let blob_id = Uuid::from_u128(nonce);
        let meta = db
            .insert_blob_meta(NewBlobMeta {
                id: blob_id,
                owner_user_id: user.id,
                content_type: "text/plain".to_string(),
                byte_size: 4,
                storage_backend: BlobBackendKind::Pg,
                storage_key: blob_id.to_string(),
            })
            .await
            .expect("blob");
        assert_eq!(meta.storage_backend, BlobBackendKind::Pg);
        assert_eq!(meta.storage_key, blob_id.to_string());
        let loaded_meta = db
            .blob_meta(blob_id)
            .await
            .expect("blob lookup")
            .expect("blob");
        assert_eq!(loaded_meta.id, blob_id);

        db.insert_action_run(NewActionRun {
            mcp_client_id: client.id,
            action_id: "web.search".to_string(),
            status: "ok".to_string(),
            input_blob_id: None,
            output_blob_id: Some(blob_id),
            error_code: None,
            duration_ms: Some(3),
        })
        .await
        .expect("run");

        sqlx::query(
            "DELETE FROM action_runs WHERE mcp_client_id IN (SELECT id FROM mcp_clients WHERE user_id = $1)",
        )
        .bind(user.id)
        .execute(db.pool())
        .await
        .expect("cleanup runs");
        db.delete_blob_meta(blob_id).await.expect("delete blob");
        assert!(db.blob_meta(blob_id).await.expect("blob gone").is_none());
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user.id)
            .execute(db.pool())
            .await
            .expect("cleanup user");
    }
}
