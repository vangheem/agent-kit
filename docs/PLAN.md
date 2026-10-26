# agent-kit plan

MCP plus a small web app so agents stop driving browsers for search, shopping, flights, and market data. This pass builds the platform and mock providers. Premium API adapters plug in later behind the provider traits. Do not call real payment, booking, or brokerage endpoints.

## Product

Two MCP tools only:

- `search` finds actions and returns their schemas
- `execute` runs one action

Users sign in on the web UI and manage MCP client tokens and service connections. Agents authenticate to MCP with a bearer token.

Progressive disclosure is the point. Do not add more MCP tools.

## Layout

```
agent-kit/
  Cargo.toml                 workspace, owned by infra
  Makefile                   owned by infra
  docker-compose.yaml        owned by infra
  .env.example               owned by infra
  rust-toolchain.toml        owned by infra
  docker/app-dev.sh          owned by infra
  crates/agent-kit/          binary, owned by infra
  crates/agent-kit-core/     types and traits, owned by core
  crates/agent-kit-db/       sqlx repos and migrations, owned by db
  crates/agent-kit-blob/     blob providers, owned by blob
  crates/agent-kit-api/      management HTTP API, owned by api
  crates/agent-kit-mcp/      MCP search/execute, owned by mcp
  crates/agent-kit-actions/  action implementations, owned by actions
  frontend/                  Vite React app, owned by ui
  docs/                      frozen plan, nobody edits during implementation
```

`docs/CONTRACTS.md` is the interface contract. If a signature there cannot be implemented, write `docs/integration-notes/<your-agent>.md` and stop. Do not edit another agent's files. Do not edit `docs/PLAN.md` or `docs/CONTRACTS.md`.

## Stack

- Rust edition 2024, stable toolchain, tokio, axum 0.8, sqlx 0.8 runtime queries (no `query!` macros)
- MCP via `rmcp` 3.4.1 with `server`, `macros`, and `transport-streamable-http-server`
- PostgreSQL 18 for relational data
- Blob payload behind `BlobStore`: `pg` (bytea) and `fs` (directory). Metadata always in Postgres
- Frontend matches acuitysync/frontend: Vite, React 19, TypeScript, MUI 6, TanStack Query 5, react-router-dom 7, `@mdi/react`, Vitest, ESLint, Prettier
- Frontend installs from the public npm registry via `frontend/.npmrc` and Docker `NPM_REGISTRY`.

## Ports

This machine already uses 5432, 7000, and 4200.

- API and MCP: `7010`
- Frontend: `4210`
- Postgres host port: `5433` (container port stays 5432)

## Runtime

One process serves `/api/*`, `/health`, and `/mcp`. Compose runs Postgres, the Rust app, and the frontend. Makefile targets: `help`, `install`, `migrate`, `run`, `stop`, `reset`, `ps`, `logs`, `test`, `lint`, `check`.

## Parallel rules

- Touch only paths listed in your brief.
- Do not `git commit`, `git push`, or open a PR.
- Do not start Docker Compose or bind host ports. Other projects are using them.
- Do not rewrite the workspace `Cargo.lock` except the infra agent, and only if you must. Prefer writing `Cargo.toml` files and compiling your own package with `cargo check -p <name> --offline` skipped. Online `cargo check -p <your-package>` is allowed. If the workspace lock is missing, create your crate manifest and leave lockfile updates to infra.
- Use `sqlx::query`, not compile-time macros.
- No comments unless a safety invariant would otherwise be invisible.
- One happy-path test per crate. Skip static-string assertions.
- Secrets come from env or encrypted DB columns. Never log tokens, passwords, or credentials.

## Safety

- `web.fetch` is http(s) only and must reject loopback, private, link-local, and cloud metadata addresses.
- Mutating actions require `confirm: true`.
- `markets.order` never places a live order. After confirm is present, return `live_trading_disabled`.
- `shop.checkout` and `flights.book` may return a mock hold only. Do not collect card numbers or call a payment API.
- v1 provider id that actually runs is `mock`, and only when `AGENT_KIT_ALLOW_MOCK=true` or the stored provider is `mock`. Any other provider returns `provider_not_configured`.
- MCP tokens and session tokens are stored as SHA-256 hashes. The raw MCP token is returned once at creation.

## Done

Each agent finishes when its owned paths implement the contract, `cargo check -p <crate>` passes (UI: `npm run build` is optional if the registry blocks it; still write the app), and a short note of anything left is in `docs/integration-notes/<agent>.md` only if blocked. Do not write that note if you finished cleanly.
