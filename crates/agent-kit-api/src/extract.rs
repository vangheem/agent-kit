use crate::ApiState;
use crate::cookie::read_session_token;
use crate::error::ApiError;
use crate::ext::{User, hash_token};
use axum::Json;
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use chrono::Utc;
use serde::de::DeserializeOwned;

pub(crate) struct AuthUser(pub User);

impl FromRequestParts<ApiState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ApiState,
    ) -> Result<Self, Self::Rejection> {
        let token = read_session_token(&parts.headers)
            .ok_or_else(|| ApiError::unauthorized("authentication required"))?;
        let user = state
            .db
            .session_user(&hash_token(&token), Utc::now())
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(|| ApiError::unauthorized("authentication required"))?;
        Ok(AuthUser(user))
    }
}

pub(crate) struct AppJson<T>(pub T);

impl<S, T> FromRequest<S> for AppJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(req, state)
            .await
            .map(|Json(value)| AppJson(value))
            .map_err(|_| ApiError::invalid("invalid json"))
    }
}
