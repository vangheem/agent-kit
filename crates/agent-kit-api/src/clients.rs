use crate::ApiState;
use crate::error::ApiError;
use crate::ext::{DbError, generate_token, hash_token, token_prefix};
use crate::extract::{AppJson, AuthUser};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub(crate) struct ClientBody {
    name: String,
    enabled_actions: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct ClientView {
    id: Uuid,
    name: String,
    token_prefix: String,
    enabled_actions: Vec<String>,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub(crate) struct CreatedClient {
    #[serde(flatten)]
    client: ClientView,
    token: String,
}

fn parse_body(body: ClientBody) -> Result<(String, Vec<String>), ApiError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::invalid("name is required"));
    }
    if body.enabled_actions.iter().any(|id| id.is_empty()) {
        return Err(ApiError::invalid(
            "enabled_actions cannot contain an empty id",
        ));
    }
    Ok((name.to_string(), body.enabled_actions))
}

fn map_client_write(err: DbError) -> ApiError {
    match err {
        DbError::NotFound => ApiError::not_found(),
        _ => ApiError::internal(),
    }
}

fn view_from_client(client: crate::ext::McpClient) -> ClientView {
    ClientView {
        id: client.id,
        name: client.name,
        token_prefix: client.token_prefix,
        enabled_actions: client.enabled_actions,
        created_at: client.created_at,
    }
}

pub(crate) async fn list_clients(
    State(state): State<ApiState>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let clients = state
        .db
        .list_mcp_clients(user.id)
        .await
        .map_err(|_| ApiError::internal())?;
    let clients: Vec<ClientView> = clients
        .into_iter()
        .filter(|client| client.revoked_at.is_none())
        .map(view_from_client)
        .collect();
    Ok(Json(serde_json::json!({ "clients": clients })))
}

pub(crate) async fn create_client(
    State(state): State<ApiState>,
    AuthUser(user): AuthUser,
    AppJson(body): AppJson<ClientBody>,
) -> Result<(StatusCode, Json<CreatedClient>), ApiError> {
    let (name, enabled_actions) = parse_body(body)?;
    let token = generate_token();
    let client = state
        .db
        .create_mcp_client(
            user.id,
            &name,
            &hash_token(&token),
            &token_prefix(&token),
            &enabled_actions,
        )
        .await
        .map_err(|_| ApiError::internal())?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedClient {
            client: view_from_client(client),
            token,
        }),
    ))
}

async fn owned_client(state: &ApiState, user_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let clients = state
        .db
        .list_mcp_clients(user_id)
        .await
        .map_err(|_| ApiError::internal())?;
    if clients
        .iter()
        .any(|client| client.id == id && client.revoked_at.is_none())
    {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}

pub(crate) async fn update_client(
    State(state): State<ApiState>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
    AppJson(body): AppJson<ClientBody>,
) -> Result<Json<ClientView>, ApiError> {
    let id = Uuid::parse_str(&id).map_err(|_| ApiError::invalid("invalid id"))?;
    let (name, enabled_actions) = parse_body(body)?;
    owned_client(&state, user.id, id).await?;
    let client = state
        .db
        .update_mcp_client(user.id, id, &name, &enabled_actions)
        .await
        .map_err(map_client_write)?;
    Ok(Json(view_from_client(client)))
}

pub(crate) async fn revoke_client(
    State(state): State<ApiState>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = Uuid::parse_str(&id).map_err(|_| ApiError::invalid("invalid id"))?;
    owned_client(&state, user.id, id).await?;
    state
        .db
        .revoke_mcp_client(user.id, id)
        .await
        .map_err(map_client_write)?;
    Ok(StatusCode::NO_CONTENT)
}
