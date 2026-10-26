use crate::ApiState;
use crate::cookie::{clear_session_cookie, read_session_token, set_session_cookie};
use crate::error::ApiError;
use crate::ext::{DbError, generate_token, hash_token};
use crate::extract::{AppJson, AuthUser};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub(crate) struct Credentials {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub(crate) struct UserBody {
    id: Uuid,
    email: String,
}

pub(crate) fn validate_registration(email: &str, password: &str) -> Result<String, ApiError> {
    let email = email.trim();
    if !email.contains('@') {
        return Err(ApiError::invalid("email must contain @"));
    }
    if password.chars().count() < 8 {
        return Err(ApiError::invalid("password must be at least 8 characters"));
    }
    Ok(email.to_string())
}

fn argon() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    argon()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| ApiError::internal())
        .map(|hash| hash.to_string())
}

fn verify_password(password: &str, password_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(password_hash) else {
        return false;
    };
    argon()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn map_register(err: DbError) -> ApiError {
    match err {
        DbError::Conflict => ApiError::conflict("email already registered"),
        _ => ApiError::internal(),
    }
}

pub(crate) async fn register(
    State(state): State<ApiState>,
    AppJson(body): AppJson<Credentials>,
) -> Result<(StatusCode, Json<UserBody>), ApiError> {
    let email = validate_registration(&body.email, &body.password)?;
    let password_hash = hash_password(&body.password)?;
    let user = state
        .db
        .create_user(&email, &password_hash)
        .await
        .map_err(map_register)?;
    Ok((
        StatusCode::CREATED,
        Json(UserBody {
            id: user.id,
            email: user.email,
        }),
    ))
}

pub(crate) async fn login(
    State(state): State<ApiState>,
    AppJson(body): AppJson<Credentials>,
) -> Result<
    (
        StatusCode,
        [(header::HeaderName, HeaderValue); 1],
        Json<UserBody>,
    ),
    ApiError,
> {
    let email = body.email.trim();
    let user = state
        .db
        .user_by_email(email)
        .await
        .map_err(|_| ApiError::internal())?;
    let valid = user
        .as_ref()
        .is_some_and(|user| verify_password(&body.password, &user.password_hash));
    let Some(user) = user.filter(|_| valid) else {
        return Err(ApiError::unauthorized("invalid email or password"));
    };
    let token = generate_token();
    let expires_at = Utc::now()
        + Duration::seconds(
            i64::try_from(state.config.session_ttl.as_secs()).unwrap_or(168 * 3600),
        );
    state
        .db
        .create_session(user.id, &hash_token(&token), expires_at)
        .await
        .map_err(|_| ApiError::internal())?;
    let cookie = set_session_cookie(&token, &state.config)?;
    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(UserBody {
            id: user.id,
            email: user.email,
        }),
    ))
}

pub(crate) async fn logout(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<(StatusCode, [(header::HeaderName, HeaderValue); 1]), ApiError> {
    if let Some(token) = read_session_token(&headers) {
        state
            .db
            .delete_session(&hash_token(&token))
            .await
            .map_err(|_| ApiError::internal())?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, clear_session_cookie(&state.config)?)],
    ))
}

pub(crate) async fn me(AuthUser(user): AuthUser) -> Json<UserBody> {
    Json(UserBody {
        id: user.id,
        email: user.email,
    })
}

#[cfg(test)]
mod tests {
    use super::validate_registration;
    use axum::http::StatusCode;

    #[test]
    fn password_shorter_than_8_is_rejected() {
        let err = validate_registration("user@example.com", "1234567").unwrap_err();
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(err.code(), "invalid_input");
        assert!(validate_registration("user@example.com", "12345678").is_ok());
    }
}
