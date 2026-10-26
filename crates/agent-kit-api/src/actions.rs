use crate::ApiState;
use crate::error::ApiError;
use crate::extract::AuthUser;
use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

pub(crate) async fn list_actions(
    State(state): State<ApiState>,
    AuthUser(_user): AuthUser,
) -> Result<Json<Value>, ApiError> {
    let actions = serde_json::to_value(state.catalog.list()).map_err(|_| ApiError::internal())?;
    Ok(Json(json!({ "actions": actions })))
}
