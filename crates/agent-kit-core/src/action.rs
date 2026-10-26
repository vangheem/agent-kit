use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::blob::BlobStore;
pub use crate::error::ActionError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Read,
    Mutating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    WebSearch,
    Shop,
    Flights,
    Markets,
}

impl ServiceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WebSearch => "web_search",
            Self::Shop => "shop",
            Self::Flights => "flights",
            Self::Markets => "markets",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "web_search" => Some(Self::WebSearch),
            "shop" => Some(Self::Shop),
            "flights" => Some(Self::Flights),
            "markets" => Some(Self::Markets),
            _ => None,
        }
    }
}

impl std::fmt::Display for ServiceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionDescriptor {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub requires: Vec<ServiceKind>,
    pub effect: Effect,
}

pub struct ServiceConnection {
    pub provider: String,
    pub credentials: serde_json::Value,
}

impl Clone for ServiceConnection {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            credentials: self.credentials.clone(),
        }
    }
}

impl std::fmt::Debug for ServiceConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceConnection")
            .field("provider", &self.provider)
            .field("credentials", &"<redacted>")
            .finish()
    }
}

pub struct ActionContext {
    pub user_id: Uuid,
    pub mcp_client_id: Uuid,
    pub allow_mock: bool,
    pub services: Arc<dyn ServiceDirectory>,
    pub blobs: Arc<dyn BlobStore>,
}

impl std::fmt::Debug for ActionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActionContext")
            .field("user_id", &self.user_id)
            .field("mcp_client_id", &self.mcp_client_id)
            .field("allow_mock", &self.allow_mock)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
pub trait ServiceDirectory: Send + Sync {
    async fn connection(
        &self,
        user_id: Uuid,
        kind: ServiceKind,
    ) -> Result<Option<ServiceConnection>, ActionError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionOutput {
    pub body: serde_json::Value,
    pub input_blob_id: Option<Uuid>,
    pub output_blob_id: Option<Uuid>,
}

#[async_trait::async_trait]
pub trait Action: Send + Sync {
    fn descriptor(&self) -> ActionDescriptor;
    async fn execute(
        &self,
        ctx: &ActionContext,
        input: serde_json::Value,
    ) -> Result<ActionOutput, ActionError>;
}
