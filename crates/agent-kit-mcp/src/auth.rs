use agent_kit_core::crypto::hash_token;
use agent_kit_db::McpClient;
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use uuid::Uuid;

use crate::server::McpState;

#[derive(Clone)]
pub(crate) struct AuthedClient {
    pub(crate) id: Uuid,
    pub(crate) user_id: Uuid,
    pub(crate) enabled_actions: Vec<String>,
}

impl AuthedClient {
    fn from_mcp(client: &McpClient) -> Self {
        Self {
            id: client.id,
            user_id: client.user_id,
            enabled_actions: client.enabled_actions.clone(),
        }
    }
}

pub(crate) fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    if token.is_empty() { None } else { Some(token) }
}

pub(crate) async fn require_bearer(
    State(state): State<McpState>,
    mut request: axum::extract::Request,
    next: Next,
) -> Response {
    let Some(token) = bearer_token(request.headers()) else {
        return error_response(StatusCode::UNAUTHORIZED, "forbidden");
    };
    let hash = hash_token(token);
    match state.db.mcp_client_by_token_hash(&hash).await {
        Ok(Some(client)) => {
            request
                .extensions_mut()
                .insert(AuthedClient::from_mcp(&client));
            next.run(request).await
        }
        Ok(None) => error_response(StatusCode::UNAUTHORIZED, "forbidden"),
        Err(_) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal"),
    }
}

pub(crate) async fn client_from_request(
    state: &McpState,
    extensions: &rmcp::model::Extensions,
) -> Result<AuthedClient, &'static str> {
    if let Some(parts) = extensions.get::<axum::http::request::Parts>() {
        if let Some(client) = parts.extensions.get::<AuthedClient>() {
            return Ok(client.clone());
        }
        if let Some(token) = bearer_token(&parts.headers) {
            let hash = hash_token(token);
            return match state.db.mcp_client_by_token_hash(&hash).await {
                Ok(Some(client)) => Ok(AuthedClient::from_mcp(&client)),
                _ => Err("forbidden"),
            };
        }
    }
    Err("forbidden")
}

fn error_response(status: StatusCode, code: &'static str) -> Response {
    (
        status,
        Json(json!({
            "error": { "code": code, "message": code }
        })),
    )
        .into_response()
}
