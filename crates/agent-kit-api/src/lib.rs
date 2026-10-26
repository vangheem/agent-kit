mod actions;
mod auth;
mod clients;
mod cookie;
mod directory;
mod error;
mod extract;
mod services;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use axum::{Json, http};
use serde::Serialize;
use tower_http::cors::CorsLayer;

pub use directory::service_directory;

pub(crate) mod ext {
    pub use agent_kit_core::action::{
        ActionError, ServiceConnection, ServiceDirectory, ServiceKind,
    };
    pub use agent_kit_core::catalog::ActionCatalog;
    pub use agent_kit_core::config::AppConfig;
    pub use agent_kit_core::crypto::{generate_token, hash_token, open, seal, token_prefix};
    pub use agent_kit_db::{Db, DbError, McpClient, User};
}

#[derive(Clone)]
pub struct ApiState {
    pub db: ext::Db,
    pub config: ext::AppConfig,
    pub catalog: Arc<dyn ext::ActionCatalog>,
}

pub fn router(state: ApiState) -> Router {
    let cors = cors_layer(&state.config.cors_origin);
    Router::new()
        .route("/health", get(health))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route(
            "/api/mcp-clients",
            get(clients::list_clients).post(clients::create_client),
        )
        .route(
            "/api/mcp-clients/{id}",
            axum::routing::patch(clients::update_client).delete(clients::revoke_client),
        )
        .route("/api/services", get(services::list_services))
        .route(
            "/api/services/{kind}",
            axum::routing::put(services::put_service).delete(services::delete_service),
        )
        .route("/api/actions", get(actions::list_actions))
        .layer(cors)
        .with_state(state)
}

#[derive(Serialize)]
struct HealthBody {
    status: &'static str,
}

async fn health() -> Json<HealthBody> {
    Json(HealthBody { status: "ok" })
}

fn cors_layer(origin: &str) -> CorsLayer {
    let origin = origin
        .parse::<http::HeaderValue>()
        .unwrap_or_else(|_| http::HeaderValue::from_static("http://localhost:4210"));
    CorsLayer::new()
        .allow_origin(origin)
        .allow_credentials(true)
        .allow_methods([
            http::Method::GET,
            http::Method::POST,
            http::Method::PUT,
            http::Method::PATCH,
            http::Method::DELETE,
            http::Method::OPTIONS,
        ])
        .allow_headers([
            http::header::CONTENT_TYPE,
            http::header::ACCEPT,
            http::header::AUTHORIZATION,
        ])
}
