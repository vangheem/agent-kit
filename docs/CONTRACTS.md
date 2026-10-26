# Contracts

Frozen for this implementation pass. Crate path names use hyphens. Rust library names use underscores.

## Workspace members

```
crates/agent-kit
crates/agent-kit-core
crates/agent-kit-db
crates/agent-kit-blob
crates/agent-kit-api
crates/agent-kit-mcp
crates/agent-kit-actions
```

Dependency direction:

- `agent-kit` depends on api, mcp, db, blob, actions, core
- `agent-kit-api` depends on core, db
- `agent-kit-mcp` depends on core, db
- `agent-kit-blob` depends on core and uses `sqlx` against the pool from db
- `agent-kit-db` depends on core
- `agent-kit-actions` depends on core
- core depends on no internal crate

`Db` is `Clone` (it wraps `PgPool`).

## Environment

| Name | Default | Notes |
| --- | --- | --- |
| `DATABASE_URL` | required | postgres URL |
| `APP_SECRET` | required | at least 32 bytes, used for AES-GCM |
| `BLOB_STORAGE_TYPE` | `pg` | `pg` or `fs` |
| `BLOB_FS_ROOT` | `/var/lib/agent-kit/blobs` | used when type is `fs` |
| `AGENT_KIT_ALLOW_MOCK` | `false` | compose example sets `true` |
| `SESSION_TTL_HOURS` | `168` | |
| `SESSION_COOKIE_SECURE` | `false` | |
| `BIND_ADDR` | `0.0.0.0:7010` | |
| `CORS_ORIGIN` | `http://localhost:4210` | one origin, credentials allowed |
| `RUST_LOG` | `info` | |

Cookie name: `agent_kit_session`. HttpOnly, SameSite=Lax, path `/`.

## Core crate

Public modules: `config`, `crypto`, `error`, `blob`, `action`, `catalog`.

```rust
pub struct AppConfig {
    pub database_url: String,
    pub app_secret: String,
    pub blob_storage_type: BlobBackendKind,
    pub blob_fs_root: PathBuf,
    pub allow_mock: bool,
    pub session_ttl: Duration,
    pub cookie_secure: bool,
    pub bind: SocketAddr,
    pub cors_origin: String,
}

pub enum BlobBackendKind { Pg, Fs }

pub fn load_config() -> Result<AppConfig, ConfigError>;

pub fn generate_token() -> String;          // 32 random bytes, URL-safe no padding
pub fn hash_token(token: &str) -> String;   // lowercase hex SHA-256
pub fn token_prefix(token: &str) -> String; // first 8 chars of the raw token
pub fn seal(secret: &str, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError>;
pub fn open(secret: &str, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError>;
```

`seal`/`open` use AES-256-GCM. Key is SHA-256(`APP_SECRET`). Nonce is random and prepended to the ciphertext.

```rust
#[async_trait::async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, request: PutBlob) -> Result<BlobMeta, BlobError>;
    async fn get(&self, id: Uuid) -> Result<BlobObject, BlobError>;
    async fn delete(&self, id: Uuid) -> Result<(), BlobError>;
}

pub struct PutBlob {
    pub owner_user_id: Uuid,
    pub content_type: String,
    pub bytes: Vec<u8>,
}
```

Reject puts over 10 MiB with `BlobError::TooLarge`.

```rust
#[async_trait::async_trait]
pub trait Action: Send + Sync {
    fn descriptor(&self) -> ActionDescriptor;
    async fn execute(&self, ctx: &ActionContext, input: serde_json::Value) -> Result<ActionOutput, ActionError>;
}

pub struct ActionDescriptor {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub requires: Vec<ServiceKind>,
    pub effect: Effect,
}

pub enum Effect { Read, Mutating }
pub enum ServiceKind { WebSearch, Shop, Flights, Markets }

impl ServiceKind {
    pub fn as_str(&self) -> &'static str; // web_search, shop, flights, markets
    pub fn parse(value: &str) -> Option<Self>;
}

pub struct ActionContext {
    pub user_id: Uuid,
    pub mcp_client_id: Uuid,
    pub allow_mock: bool,
    pub services: Arc<dyn ServiceDirectory>,
    pub blobs: Arc<dyn BlobStore>,
}

#[async_trait::async_trait]
pub trait ServiceDirectory: Send + Sync {
    async fn connection(&self, user_id: Uuid, kind: ServiceKind) -> Result<Option<ServiceConnection>, ActionError>;
}

pub struct ServiceConnection {
    pub provider: String,
    pub credentials: serde_json::Value,
}

pub struct ActionOutput {
    pub body: serde_json::Value,
    pub input_blob_id: Option<Uuid>,
    pub output_blob_id: Option<Uuid>,
}

pub enum ActionError {
    NotFound,
    Forbidden,
    ConfirmationRequired,
    ProviderNotConfigured { provider: String },
    InvalidInput { message: String },
    Provider { message: String },
    LiveTradingDisabled,
    Internal { message: String },
}
```

`ActionError` display text starts with a stable code: `not_found`, `forbidden`, `confirmation_required`, `provider_not_configured`, `invalid_input`, `provider_error`, `live_trading_disabled`, `internal`.

```rust
#[async_trait::async_trait]
pub trait ActionCatalog: Send + Sync {
    fn list(&self) -> Vec<ActionDescriptor>;
    fn get(&self, id: &str) -> Option<Arc<dyn Action>>;
    fn search(&self, query: &str, limit: usize) -> Vec<ActionDescriptor>;
}

pub struct MapCatalog { /* private */ }

impl MapCatalog {
    pub fn new(actions: Vec<Arc<dyn Action>>) -> Self;
}

impl ActionCatalog for MapCatalog {}
```

Search rules:

- Empty query returns the first `limit` descriptors sorted by id.
- If `query` equals an action id, return that one descriptor.
- Otherwise split `query` on whitespace, lowercase, and score id/title/summary by how many tokens occur. Sort by score desc, then id asc. `limit` is clamped to 1..=50. Default used by MCP is 10.

## Database

Migrations live in `crates/agent-kit-db/migrations`. One initial migration. Enable `citext` and `pgcrypto`.

```sql
CREATE TABLE users (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email citext NOT NULL UNIQUE,
  password_hash text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash text NOT NULL UNIQUE,
  expires_at timestamptz NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE mcp_clients (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name text NOT NULL,
  token_hash text NOT NULL UNIQUE,
  token_prefix text NOT NULL,
  enabled_actions text[] NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz
);

CREATE TABLE service_connections (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  kind text NOT NULL,
  provider text NOT NULL,
  credentials_ciphertext bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (user_id, kind)
);

CREATE TABLE blobs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  owner_user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  content_type text NOT NULL,
  byte_size bigint NOT NULL,
  storage_backend text NOT NULL,
  storage_key text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE blob_payloads (
  blob_id uuid PRIMARY KEY REFERENCES blobs(id) ON DELETE CASCADE,
  data bytea NOT NULL
);

CREATE TABLE action_runs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  mcp_client_id uuid NOT NULL REFERENCES mcp_clients(id) ON DELETE CASCADE,
  action_id text NOT NULL,
  status text NOT NULL,
  input_blob_id uuid REFERENCES blobs(id),
  output_blob_id uuid REFERENCES blobs(id),
  error_code text,
  duration_ms integer,
  created_at timestamptz NOT NULL DEFAULT now()
);
```

`storage_key` is the blob uuid string for both backends. `storage_backend` is `pg` or `fs`.

```rust
pub struct Db { /* pool */ }
impl Db {
    pub async fn connect(database_url: &str) -> Result<Self, DbError>;
    pub async fn migrate(&self) -> Result<(), DbError>;
    pub fn pool(&self) -> &sqlx::PgPool;
    pub async fn create_user(&self, email: &str, password_hash: &str) -> Result<User, DbError>;
    pub async fn user_by_email(&self, email: &str) -> Result<Option<User>, DbError>;
    pub async fn user_by_id(&self, id: Uuid) -> Result<Option<User>, DbError>;
    pub async fn create_session(&self, user_id: Uuid, token_hash: &str, expires_at: DateTime<Utc>) -> Result<(), DbError>;
    pub async fn session_user(&self, token_hash: &str, now: DateTime<Utc>) -> Result<Option<User>, DbError>;
    pub async fn delete_session(&self, token_hash: &str) -> Result<(), DbError>;
    pub async fn create_mcp_client(&self, user_id: Uuid, name: &str, token_hash: &str, token_prefix: &str, enabled_actions: &[String]) -> Result<McpClient, DbError>;
    pub async fn list_mcp_clients(&self, user_id: Uuid) -> Result<Vec<McpClient>, DbError>;
    pub async fn update_mcp_client(&self, user_id: Uuid, id: Uuid, name: &str, enabled_actions: &[String]) -> Result<McpClient, DbError>;
    pub async fn revoke_mcp_client(&self, user_id: Uuid, id: Uuid) -> Result<(), DbError>;
    pub async fn mcp_client_by_token_hash(&self, token_hash: &str) -> Result<Option<McpClient>, DbError>;
    pub async fn upsert_service(&self, user_id: Uuid, kind: &str, provider: &str, credentials_ciphertext: &[u8]) -> Result<ServiceConnectionRow, DbError>;
    pub async fn list_services(&self, user_id: Uuid) -> Result<Vec<ServiceConnectionRow>, DbError>;
    pub async fn service(&self, user_id: Uuid, kind: &str) -> Result<Option<ServiceConnectionRow>, DbError>;
    pub async fn delete_service(&self, user_id: Uuid, kind: &str) -> Result<(), DbError>;
    pub async fn insert_action_run(&self, run: NewActionRun) -> Result<(), DbError>;
    pub async fn insert_blob_meta(&self, meta: NewBlobMeta) -> Result<BlobMeta, DbError>;
    pub async fn blob_meta(&self, id: Uuid) -> Result<Option<BlobMeta>, DbError>;
    pub async fn delete_blob_meta(&self, id: Uuid) -> Result<(), DbError>;
}
```

`mcp_client_by_token_hash` returns `None` when `revoked_at` is set. `session_user` returns `None` when expired. Duplicate email returns `DbError::Conflict`.

`McpClient` fields: `id`, `user_id`, `name`, `token_prefix`, `enabled_actions`, `created_at`, `revoked_at`. Do not put `token_hash` on the struct returned to callers. Lookup can still use the hash internally.

## Blob crate

```rust
pub async fn connect(config: &AppConfig, pool: &sqlx::PgPool) -> Result<Arc<dyn BlobStore>, BlobError>;
```

- `pg`: metadata row plus `blob_payloads.data` in one transaction. `get` joins them. `delete` deletes the metadata row and relies on cascade.
- `fs`: write `{BLOB_FS_ROOT}/{id}` then insert metadata. If the insert fails, remove the file. `get` reads the file. `delete` removes metadata and the file.
- Unknown `BLOB_STORAGE_TYPE` is a config error in core, not here.

## API crate

JSON errors: `{ "error": { "code": "...", "message": "..." } }`.

Password hashing is argon2id in this crate. Minimum password length 8. Email must contain `@`.

```rust
#[derive(Clone)]
pub struct ApiState {
    pub db: Db,
    pub config: AppConfig,
    pub catalog: Arc<dyn ActionCatalog>,
}

pub fn router(state: ApiState) -> axum::Router;
pub fn service_directory(db: Db, app_secret: String) -> Arc<dyn ServiceDirectory>;
```

Routes:

- `GET /health` -> `{ "status": "ok" }` public
- `POST /api/auth/register` `{ "email", "password" }` -> `201` `{ "id", "email" }`
- `POST /api/auth/login` `{ "email", "password" }` -> `200` `{ "id", "email" }` and sets the session cookie
- `POST /api/auth/logout` -> `204` and clears the cookie
- `GET /api/auth/me` -> `{ "id", "email" }`
- `GET /api/mcp-clients` -> `{ "clients": [ { "id", "name", "token_prefix", "enabled_actions", "created_at" } ] }`
- `POST /api/mcp-clients` `{ "name", "enabled_actions": string[] }` -> `201` includes `token` once
- `PATCH /api/mcp-clients/:id` `{ "name", "enabled_actions" }`
- `DELETE /api/mcp-clients/:id` -> `204` revoke
- `GET /api/services` -> `{ "services": [ { "kind", "provider", "configured": true } ] }`
- `PUT /api/services/:kind` `{ "provider", "credentials": object }` -> `200` same public shape. Encrypt credentials before storage.
- `DELETE /api/services/:kind` -> `204`
- `GET /api/actions` -> `{ "actions": [ ActionDescriptor as JSON ] }`

Protected routes use the session cookie. `401` if missing or expired. CORS allows the configured origin with credentials. Do not return ciphertext or password hashes.

`service_directory` decrypts with `open`. Bad ciphertext is `ActionError::Internal`.

## MCP crate

Expose only tools `search` and `execute` through rmcp streamable HTTP mounted at `/mcp`. No resources, prompts, or extra tools.

```rust
#[derive(Clone)]
pub struct McpState {
    pub db: Db,
    pub config: AppConfig,
    pub catalog: Arc<dyn ActionCatalog>,
    pub services: Arc<dyn ServiceDirectory>,
    pub blobs: Arc<dyn BlobStore>,
}

pub fn router(state: McpState) -> axum::Router;
```

Bearer auth: `Authorization: Bearer <raw token>`. Missing or unknown token is HTTP 401 before MCP initialization if the transport allows it; otherwise fail the tool call with `forbidden`. Prefer rejecting the HTTP request with 401.

`search` arguments:

```json
{
  "type": "object",
  "properties": {
    "query": { "type": "string" },
    "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
  }
}
```

Filter the catalog to `enabled_actions` first. Empty `enabled_actions` means every action is enabled. Return JSON `{ "actions": [ descriptors ] }` as the tool text and as structured content.

`execute` arguments:

```json
{
  "type": "object",
  "required": ["action"],
  "properties": {
    "action": { "type": "string" },
    "input": { "type": "object" },
    "confirm": { "type": "boolean" }
  }
}
```

- Unknown action: tool error `not_found`
- Not enabled: tool error `forbidden`
- `Effect::Mutating` and `confirm` is not true: tool error `confirmation_required`. Also accept `input.confirm == true`.
- On success, return the action body as JSON text and structured content.
- Record an `action_runs` row for both success and tool-level failure. `status` is `ok` or `error`. `error_code` is the stable code.

Build `ActionContext` from the authenticated client, `config.allow_mock`, `services`, and `blobs`.

## Actions

```rust
pub fn register() -> Vec<Arc<dyn Action>>;
```

Action ids, effects, and required service kinds:

| id | effect | requires | input |
| --- | --- | --- | --- |
| `web.search` | read | web_search | `query` string, optional `limit` |
| `web.fetch` | read | none | `url` string |
| `shop.search` | read | shop | `query` string, optional `limit` |
| `shop.quote` | read | shop | `product_id` string |
| `shop.checkout` | mutating | shop | `product_id` string, `quantity` integer >= 1 |
| `flights.search` | read | flights | `origin`, `destination`, `depart_date`, optional `return_date`, optional `adults` |
| `flights.quote` | read | flights | `offer_id` string |
| `flights.book` | mutating | flights | `offer_id` string |
| `markets.quote` | read | markets | `symbol` string |
| `markets.candles` | read | markets | `symbol`, `interval` of `1d` or `1h`, optional `limit` |
| `markets.order` | mutating | markets | `symbol`, `side` of `buy` or `sell`, `quantity` number > 0 |

Output shapes:

- `web.search`: `{ "results": [{ "title", "url", "snippet" }] }`
- `web.fetch`: `{ "title", "url", "text", "content_type" }` and store the raw body via `BlobStore`. Put that id on `ActionOutput.output_blob_id`.
- `shop.search`: `{ "products": [{ "id", "title", "price": { "amount", "currency" }, "url", "merchant" }] }`
- `shop.quote`: `{ "product_id", "available", "price": { "amount", "currency" }, "shipping_estimate" }`
- `shop.checkout`: `{ "status": "mock_hold", "hold_id", "product_id", "quantity" }`
- `flights.search`: `{ "offers": [{ "id", "carrier", "depart_at", "arrive_at", "stops", "price": { "amount", "currency" } }] }`
- `flights.quote`: `{ "offer_id", "price": { "amount", "currency" }, "expires_at" }`
- `flights.book`: `{ "status": "mock_hold", "hold_id", "offer_id" }`
- `markets.quote`: `{ "symbol", "price", "currency", "as_of" }`
- `markets.candles`: `{ "candles": [{ "t", "o", "h", "l", "c", "v" }] }`
- `markets.order`: always `ActionError::LiveTradingDisabled` once confirm has been checked by MCP. The action itself also returns that error if called directly.

Provider resolution:

- If a service connection exists and `provider == "mock"`, run the mock.
- If no connection exists and `ctx.allow_mock` is true, run the mock.
- Otherwise return `provider_not_configured` with the provider name or `"none"`.
- `web.fetch` does not consult a service connection. It fetches the URL directly after the SSRF check, with a 10 second timeout and a 2 MiB body cap.

Mock data is deterministic from the query so tests can assert a field. Do not use network in mock paths.

`web.fetch` must refuse non-http(s) URLs and any resolved address that is loopback, private, link-local, multicast, or `169.254.169.254`. Check every resolved address before connecting.

## Frontend

Match acuitysync page structure, not its branding.

- Pages: `frontend/src/pages/<page>/index.tsx`
- Routes registered in `frontend/src/routes.ts`
- `App.tsx` only wires providers, theme, and the shell
- API functions in `frontend/src/api/`
- Shared types in `frontend/src/types.ts`
- `frontend/src/config.ts` reads `import.meta.env.VITE_API_URL`, default `http://localhost:7010`
- TanStack Query for server state. Components do not call `fetch` directly.
- Cookie auth: `credentials: "include"`
- MUI components. Icons from `@mdi/react` and `@mdi/js`

Pages:

- `/login`
- `/register`
- `/` dashboard with client count and connected services
- `/mcp-clients` create, list, revoke. Show the raw token once after create.
- `/services` connect or remove `web_search`, `shop`, `flights`, `markets`. Credential fields are a JSON object textarea plus provider name. Do not echo stored secrets; the API does not return them.
- `/actions` read-only catalog from `GET /api/actions`

Unauthenticated users hitting protected pages go to `/login`.

`frontend/.npmrc` must set:

```
registry=https://registry.npmjs.org/
```

Dockerfile development target listens on 4210. Build arg `NPM_REGISTRY` defaults to that same URL.

## Binary

`agent-kit` binary:

- `agent-kit migrate` connects and runs `Db::migrate`
- `agent-kit serve` loads config, connects db, connects blob store, builds `MapCatalog` from `agent_kit_actions::register()`, builds `service_directory`, merges `agent_kit_api::router` and `agent_kit_mcp::router`, and serves `BIND_ADDR`

`/health` stays on the API router. MCP is only `/mcp`.

## Compose

Services: `db`, `app`, `frontend`.

- `db`: `postgres:18-alpine`, user/db/password `agentkit`, host port `5433`, healthcheck `pg_isready`
- `app`: Rust dev image, source mounted, runs migrate then `agent-kit serve`, port `7010`
- `frontend`: node 24 alpine, source mounted, npm registry build arg as above, port `4210`
- Named volumes for postgres data, cargo registry, and frontend `node_modules`
- Env files: `.env.example` committed, `.env.compose` committed with local defaults, `.env.compose.local` gitignored

`APP_SECRET` in the example is a local-only value of at least 32 characters. Comments in the example say to replace it.
