use std::sync::Arc;
use std::time::Instant;

use agent_kit_core::action::{ActionContext, ActionError};
use agent_kit_core::blob::BlobStore;
use agent_kit_core::catalog::{ActionCatalog, MapCatalog};
use agent_kit_core::config::AppConfig;
use agent_kit_db::{Db, NewActionRun};
use axum::middleware;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{RoleServer, ServerHandler, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::auth::{self, AuthedClient};
use crate::policy;

#[derive(Clone)]
pub struct McpState {
    pub db: Db,
    pub config: AppConfig,
    pub catalog: Arc<dyn ActionCatalog>,
    pub services: Arc<dyn agent_kit_core::action::ServiceDirectory>,
    pub blobs: Arc<dyn BlobStore>,
}

pub fn router(state: McpState) -> axum::Router {
    let service_state = state.clone();
    let service = StreamableHttpService::new(
        move || Ok(AgentKitServer::new(service_state.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    axum::Router::new()
        .nest_service("/mcp", service)
        .route_layer(middleware::from_fn_with_state(state, auth::require_bearer))
}

#[derive(Clone)]
struct AgentKitServer {
    state: McpState,
}

#[derive(Debug, Default, Deserialize)]
struct SearchArgs {
    #[serde(default)]
    query: String,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ExecuteArgs {
    action: String,
    #[serde(default)]
    input: Option<Value>,
    #[serde(default)]
    confirm: Option<bool>,
}

struct RunRecord {
    action_id: String,
    status: &'static str,
    error_code: Option<String>,
    input_blob_id: Option<Uuid>,
    output_blob_id: Option<Uuid>,
    result: CallToolResult,
}

fn object_schema(value: Value) -> Arc<Map<String, Value>> {
    Arc::new(match value {
        Value::Object(map) => map,
        _ => Map::new(),
    })
}

fn search_input_schema() -> Arc<Map<String, Value>> {
    object_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" },
            "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
        }
    }))
}

fn execute_input_schema() -> Arc<Map<String, Value>> {
    object_schema(serde_json::json!({
        "type": "object",
        "required": ["action"],
        "properties": {
            "action": { "type": "string" },
            "input": { "type": "object" },
            "confirm": { "type": "boolean" }
        }
    }))
}

fn json_result(value: Value) -> CallToolResult {
    CallToolResult::structured(value)
}

fn tool_error(code: &str) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(code.to_string())])
}

fn tool_error_detail(code: &str, detail: impl std::fmt::Display) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(policy::error_text(code, detail))])
}

fn descriptors_payload(
    descriptors: &[agent_kit_core::action::ActionDescriptor],
) -> Result<Value, ()> {
    let mut actions = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        actions.push(serde_json::to_value(descriptor).map_err(|_| ())?);
    }
    Ok(serde_json::json!({ "actions": actions }))
}

fn enabled_catalog(catalog: &dyn ActionCatalog, enabled: &[String]) -> MapCatalog {
    let actions = catalog
        .list()
        .into_iter()
        .filter(|descriptor| policy::action_allowed(enabled, &descriptor.id))
        .filter_map(|descriptor| catalog.get(&descriptor.id))
        .collect();
    MapCatalog::new(actions)
}

fn normalize_input(value: Option<Value>) -> Result<Value, &'static str> {
    match value {
        None | Some(Value::Null) => Ok(serde_json::json!({})),
        Some(value) if value.is_object() => Ok(value),
        Some(_) => Err("invalid_input"),
    }
}

#[tool_router]
impl AgentKitServer {
    fn new(state: McpState) -> Self {
        Self { state }
    }

    #[tool(
        description = "Find actions and return their schemas",
        input_schema = search_input_schema(),
        annotations(read_only_hint = true, destructive_hint = false, open_world_hint = false)
    )]
    async fn search(
        &self,
        Parameters(args): Parameters<SearchArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let client = match auth::client_from_request(&self.state, &ctx.extensions).await {
            Ok(client) => client,
            Err(code) => return Ok(tool_error(code)),
        };
        let catalog = enabled_catalog(self.state.catalog.as_ref(), &client.enabled_actions);
        let descriptors = catalog.search(&args.query, policy::search_limit(args.limit));
        match descriptors_payload(&descriptors) {
            Ok(payload) => Ok(json_result(payload)),
            Err(()) => Ok(tool_error("internal")),
        }
    }

    #[tool(
        description = "Run one action",
        input_schema = execute_input_schema(),
        annotations(read_only_hint = false, destructive_hint = true, open_world_hint = true)
    )]
    async fn execute(
        &self,
        Parameters(args): Parameters<ExecuteArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let started = Instant::now();
        let client = match auth::client_from_request(&self.state, &ctx.extensions).await {
            Ok(client) => client,
            Err(code) => return Ok(tool_error(code)),
        };
        let record = self.execute_for(&client, args).await;
        self.record_run(&client, &record, started).await;
        Ok(record.result)
    }
}

impl AgentKitServer {
    async fn execute_for(&self, client: &AuthedClient, args: ExecuteArgs) -> RunRecord {
        let action_id = args.action;
        let Some(action) = self.state.catalog.get(&action_id) else {
            return failed(action_id, "not_found");
        };
        if !policy::action_allowed(&client.enabled_actions, &action_id) {
            return failed(action_id, "forbidden");
        }
        let input = match normalize_input(args.input) {
            Ok(input) => input,
            Err(code) => return failed(action_id, code),
        };
        let descriptor = action.descriptor();
        if policy::confirmation_required(&descriptor.effect, args.confirm, &input) {
            return failed(action_id, "confirmation_required");
        }
        let input = policy::strip_confirm(input);
        let action_ctx = ActionContext {
            user_id: client.user_id,
            mcp_client_id: client.id,
            allow_mock: self.state.config.allow_mock,
            services: Arc::clone(&self.state.services),
            blobs: Arc::clone(&self.state.blobs),
        };
        match action.execute(&action_ctx, input).await {
            Ok(output) => RunRecord {
                action_id,
                status: "ok",
                error_code: None,
                input_blob_id: output.input_blob_id,
                output_blob_id: output.output_blob_id,
                result: json_result(output.body),
            },
            Err(err) => failed_action(action_id, err),
        }
    }

    async fn record_run(&self, client: &AuthedClient, record: &RunRecord, started: Instant) {
        let duration_ms = i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX);
        let run = NewActionRun {
            mcp_client_id: client.id,
            action_id: record.action_id.clone(),
            status: record.status.to_string(),
            input_blob_id: record.input_blob_id,
            output_blob_id: record.output_blob_id,
            error_code: record.error_code.clone(),
            duration_ms: Some(duration_ms),
        };
        if self.state.db.insert_action_run(run).await.is_err() {
            tracing::error!("failed to record action run");
        }
    }
}

fn failed(action_id: String, code: &str) -> RunRecord {
    RunRecord {
        action_id,
        status: "error",
        error_code: Some(code.to_string()),
        input_blob_id: None,
        output_blob_id: None,
        result: tool_error(code),
    }
}

fn failed_action(action_id: String, err: ActionError) -> RunRecord {
    let code = err.code();
    RunRecord {
        action_id,
        status: "error",
        error_code: Some(code.to_string()),
        input_blob_id: None,
        output_blob_id: None,
        result: tool_error_detail(code, err),
    }
}

#[tool_handler]
impl ServerHandler for AgentKitServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("agent-kit", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Use search to discover actions and execute to run one. Mutating actions require confirm true.",
            )
    }
}
