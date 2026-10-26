# mcp

Read `docs/PLAN.md` and `docs/CONTRACTS.md`. Implement `crates/agent-kit-mcp` only.

Use `rmcp` 3.4.1 features `server`, `macros`, `transport-streamable-http-server`. Mount streamable HTTP at `/mcp`. Tools are only `search` and `execute`. Read the rmcp axum streamable HTTP example if the mount type is unclear. Do not add a second MCP SDK.

Enforce bearer tokens via `mcp_client_by_token_hash`. Filter by `enabled_actions` (empty means all). Mutating actions need `confirm` true on the tool argument or `input.confirm`. Record `action_runs` for success and failure.

Tool failures the model should see use a tool error result whose text starts with the stable code. Do not add tools.

Package name `agent-kit-mcp`. Path deps on core and db. Do not edit the workspace `Cargo.toml`.

One test can cover enabled-action filtering or confirm handling if you extract that into a pure function. Do not start compose. Do not commit.
