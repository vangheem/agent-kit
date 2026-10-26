# api

Read `docs/PLAN.md` and `docs/CONTRACTS.md`. Implement `crates/agent-kit-api` only.

Build the axum router and `service_directory`. Argon2id for passwords. Session cookie name `agent_kit_session`. Store only `hash_token` of the session and MCP token. Return the raw MCP token only from `POST /api/mcp-clients`.

Never return password hashes or credential ciphertext. Encrypt service credentials with `agent_kit_core::seal` before `upsert_service`.

CORS: configured origin, credentials, the methods and headers the UI needs.

Add one test that a password shorter than 8 is rejected, or that the public service JSON shape has no ciphertext field. Do not start compose.

Package name `agent-kit-api`. Path deps on core and db. Do not edit the workspace `Cargo.toml`.

Do not commit.
