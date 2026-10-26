use std::sync::Arc;

use agent_kit_api::ApiState;
use agent_kit_core::{ActionCatalog, MapCatalog};
use agent_kit_mcp::McpState;

#[tokio::main]
async fn main() {
    init_tracing();
    match std::env::args().nth(1).as_deref() {
        Some("migrate") => migrate().await,
        Some("serve") => serve().await,
        _ => usage(),
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn usage() -> ! {
    eprintln!("usage: agent-kit <migrate|serve>");
    std::process::exit(2);
}

fn die(err: impl std::fmt::Display) -> ! {
    eprintln!("agent-kit: {err}");
    std::process::exit(1);
}

async fn migrate() {
    let config = agent_kit_core::load_config().unwrap_or_else(|err| die(err));
    let db = agent_kit_db::Db::connect(&config.database_url)
        .await
        .unwrap_or_else(|err| die(err));
    db.migrate().await.unwrap_or_else(|err| die(err));
    tracing::info!("migrations complete");
}

async fn serve() {
    let config = agent_kit_core::load_config().unwrap_or_else(|err| die(err));
    let db = agent_kit_db::Db::connect(&config.database_url)
        .await
        .unwrap_or_else(|err| die(err));
    let blobs = agent_kit_blob::connect(&config, db.pool())
        .await
        .unwrap_or_else(|err| die(err));
    let catalog: Arc<dyn ActionCatalog> = Arc::new(MapCatalog::new(agent_kit_actions::register()));
    let services = agent_kit_api::service_directory(db.clone(), config.app_secret.clone());
    let api = agent_kit_api::router(ApiState {
        db: db.clone(),
        config: config.clone(),
        catalog: catalog.clone(),
    });
    let mcp = agent_kit_mcp::router(McpState {
        db,
        config: config.clone(),
        catalog,
        services,
        blobs,
    });
    let app = api.merge(mcp);
    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .unwrap_or_else(|err| die(err));
    let addr = listener.local_addr().unwrap_or(config.bind);
    tracing::info!(%addr, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await
        .unwrap_or_else(|err| die(err));
}

async fn shutdown() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .unwrap_or_else(|err| die(err));
        tokio::select! {
            result = ctrl_c => {
                if let Err(err) = result {
                    tracing::error!(%err, "failed to wait for interrupt");
                }
            }
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        if let Err(err) = ctrl_c.await {
            tracing::error!(%err, "failed to wait for interrupt");
        }
    }
}
