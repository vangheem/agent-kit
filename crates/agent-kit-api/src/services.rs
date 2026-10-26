use crate::ApiState;
use crate::error::ApiError;
use crate::ext::{ServiceKind, seal};
use crate::extract::{AppJson, AuthUser};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(crate) struct PutServiceBody {
    provider: String,
    credentials: serde_json::Value,
}

#[derive(Serialize)]
pub(crate) struct ServiceView {
    kind: String,
    provider: String,
    configured: bool,
}

fn parse_kind(kind: &str) -> Result<ServiceKind, ApiError> {
    ServiceKind::parse(kind).ok_or_else(|| ApiError::invalid("unknown service"))
}

pub(crate) async fn list_services(
    State(state): State<ApiState>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = state
        .db
        .list_services(user.id)
        .await
        .map_err(|_| ApiError::internal())?;
    let services: Vec<ServiceView> = rows
        .into_iter()
        .map(|row| ServiceView {
            kind: row.kind,
            provider: row.provider,
            configured: true,
        })
        .collect();
    Ok(Json(serde_json::json!({ "services": services })))
}

pub(crate) async fn put_service(
    State(state): State<ApiState>,
    AuthUser(user): AuthUser,
    Path(kind): Path<String>,
    AppJson(body): AppJson<PutServiceBody>,
) -> Result<Json<ServiceView>, ApiError> {
    let kind = parse_kind(&kind)?;
    let provider = body.provider.trim();
    if provider.is_empty() {
        return Err(ApiError::invalid("provider is required"));
    }
    if !body.credentials.is_object() {
        return Err(ApiError::invalid("credentials must be an object"));
    }
    let plaintext = serde_json::to_vec(&body.credentials).map_err(|_| ApiError::internal())?;
    let ciphertext =
        seal(&state.config.app_secret, &plaintext).map_err(|_| ApiError::internal())?;
    let row = state
        .db
        .upsert_service(user.id, kind.as_str(), provider, &ciphertext)
        .await
        .map_err(|_| ApiError::internal())?;
    Ok(Json(ServiceView {
        kind: row.kind,
        provider: row.provider,
        configured: true,
    }))
}

pub(crate) async fn delete_service(
    State(state): State<ApiState>,
    AuthUser(user): AuthUser,
    Path(kind): Path<String>,
) -> Result<StatusCode, ApiError> {
    let kind = parse_kind(&kind)?;
    state
        .db
        .delete_service(user.id, kind.as_str())
        .await
        .map_err(|_| ApiError::internal())?;
    Ok(StatusCode::NO_CONTENT)
}
