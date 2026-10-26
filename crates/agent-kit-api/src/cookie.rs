use crate::error::ApiError;
use crate::ext::AppConfig;
use axum::http::{HeaderMap, HeaderValue, header};

pub(crate) const SESSION_COOKIE: &str = "agent_kit_session";

pub(crate) fn read_session_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let Some((name, value)) = part.trim().split_once('=') else {
            continue;
        };
        if name.trim() == SESSION_COOKIE {
            let token = value.trim();
            if token.is_empty() {
                return None;
            }
            return Some(token.to_string());
        }
    }
    None
}

pub(crate) fn set_session_cookie(token: &str, config: &AppConfig) -> Result<HeaderValue, ApiError> {
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}{}",
        config.session_ttl.as_secs(),
        secure_attr(config),
    ))
    .map_err(|_| ApiError::internal())
}

pub(crate) fn clear_session_cookie(config: &AppConfig) -> Result<HeaderValue, ApiError> {
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}",
        secure_attr(config),
    ))
    .map_err(|_| ApiError::internal())
}

fn secure_attr(config: &AppConfig) -> &'static str {
    if config.cookie_secure { "; Secure" } else { "" }
}
