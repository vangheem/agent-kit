use crate::ext::{ActionError, Db, ServiceConnection, ServiceDirectory, ServiceKind, open};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

struct DbServiceDirectory {
    db: Db,
    app_secret: String,
}

pub fn service_directory(db: Db, app_secret: String) -> Arc<dyn ServiceDirectory> {
    Arc::new(DbServiceDirectory { db, app_secret })
}

#[async_trait]
impl ServiceDirectory for DbServiceDirectory {
    async fn connection(
        &self,
        user_id: Uuid,
        kind: ServiceKind,
    ) -> Result<Option<ServiceConnection>, ActionError> {
        let row =
            self.db
                .service(user_id, kind.as_str())
                .await
                .map_err(|_| ActionError::Internal {
                    message: "service lookup failed".into(),
                })?;
        let Some(row) = row else {
            return Ok(None);
        };
        let plaintext = open(&self.app_secret, &row.credentials_ciphertext).map_err(|_| {
            ActionError::Internal {
                message: "failed to decrypt credentials".into(),
            }
        })?;
        let credentials =
            serde_json::from_slice(&plaintext).map_err(|_| ActionError::Internal {
                message: "credentials are not valid json".into(),
            })?;
        Ok(Some(ServiceConnection {
            provider: row.provider,
            credentials,
        }))
    }
}
