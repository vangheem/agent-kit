# agent-kit

MCP server and small web app so agents can search, shop, look up flights, and read market data without driving a browser. This version runs the platform and mock providers. It does not call payment, booking, or brokerage APIs.

Agents get two tools:

- `search` finds actions and returns their schemas
- `execute` runs one action

People sign in on the web UI, create MCP client tokens, and connect services. Agents authenticate to MCP with a bearer token.

## Layout

```
agent-kit/
  crates/agent-kit/           binary: migrate and serve
  crates/agent-kit-core/      config, crypto, action and blob traits
  crates/agent-kit-db/        sqlx repositories and migrations
  crates/agent-kit-blob/      Postgres or filesystem blob payloads
  crates/agent-kit-api/       session-cookie management API
  crates/agent-kit-mcp/       streamable HTTP MCP at /mcp
  crates/agent-kit-actions/   web, shop, flights, and markets actions
  frontend/                   Vite, React, and MUI
  docker/                     app dev entrypoint
  docs/CONTRACTS.md           interface contract
  docs/PLAN.md                product and implementation plan
```

One process serves `/api/*`, `/health`, and `/mcp`.

## Requirements

- Rust stable, with `clippy` and `rustfmt` (`rust-toolchain.toml`)
- Docker with Compose
- Node.js 24 for host-side frontend work

`frontend/.npmrc` uses the public npm registry.

Ports are offset from the usual defaults because 5432, 7000, and 4200 are often already taken:

| Service | URL |
| --- | --- |
| Web UI | http://localhost:4210 |
| API and MCP | http://localhost:7010 |
| Postgres | `localhost:5433` |

## Quick start

```sh
make install
make run
```

`make install` creates `.env` from `.env.example` when it is missing, creates `.env.compose.local`, installs frontend dependencies, and fetches Rust crates.

`make run` starts Postgres, the Rust app, and the frontend. The app container builds the binary, migrates, then serves. The first start is slow.

Open http://localhost:4210, register, and create an MCP client. The raw token is shown once. Point an MCP client at `http://localhost:7010/mcp` with `Authorization: Bearer <token>`.

Compose defaults set `AGENT_KIT_ALLOW_MOCK=true`, so actions run the mock provider when no service connection is stored. Override compose settings in `.env.compose.local`.

Stop the attached stack with Ctrl-C, or from another terminal:

```sh
make stop
```

`make reset` asks you to type `RESET` before it deletes Compose volumes.

## Host-side development

Copy `.env.example` to `.env` if `make install` has not already. The host database URL uses `localhost:5433`. Compose uses `postgres://agentkit:agentkit@db:5432/agentkit` inside the app container.

Start only Postgres if you want to run the binary on the host:

```sh
docker compose up db
make migrate
make serve
```

In another terminal:

```sh
make -C frontend run
```

The binary is `agent-kit migrate` or `agent-kit serve`.

## Make targets

| Target | What it does |
| --- | --- |
| `make help` | List targets |
| `make install` | Create local env files and install dependencies |
| `make migrate` | Run migrations, loading `.env` when present |
| `make serve` | Run the API and MCP server on the host |
| `make run` | `docker compose up --build` |
| `make stop` | `docker compose down` |
| `make reset` | Stop the stack and delete volumes after confirmation |
| `make ps` | Compose service status |
| `make logs` | Follow `db`, `app`, and `frontend` logs |
| `make test` | `cargo test --workspace` and frontend Vitest |
| `make lint` | `cargo fmt --check`, Clippy `-D warnings`, frontend lint |
| `make fmt` | Format Rust and frontend sources |
| `make check` | `cargo check --workspace`, then lint |

Frontend-only targets live in `frontend/Makefile`.

## Configuration

`.env.example` is for host-side commands. `.env.compose` is the committed compose default. `.env` and `.env.compose.local` are gitignored.

| Name | Default | Notes |
| --- | --- | --- |
| `DATABASE_URL` | required | Postgres URL |
| `APP_SECRET` | required | At least 32 bytes. Used for AES-GCM. Replace the local placeholder before any shared use |
| `BLOB_STORAGE_TYPE` | `pg` | `pg` or `fs` |
| `BLOB_FS_ROOT` | `/var/lib/agent-kit/blobs` | Used when storage type is `fs` |
| `AGENT_KIT_ALLOW_MOCK` | `false` | Compose sets `true` |
| `SESSION_TTL_HOURS` | `168` | |
| `SESSION_COOKIE_SECURE` | `false` | |
| `BIND_ADDR` | `0.0.0.0:7010` | |
| `CORS_ORIGIN` | `http://localhost:4210` | One origin, credentials allowed |
| `RUST_LOG` | `info` | |

The session cookie is `agent_kit_session`. It is HttpOnly, SameSite=Lax, and path `/`.

Blob metadata always lives in Postgres. Payloads are either `bytea` (`pg`) or files under `BLOB_FS_ROOT` (`fs`). Puts over 10 MiB are rejected.

## Web UI

| Path | Page |
| --- | --- |
| `/login`, `/register` | Account |
| `/` | Dashboard |
| `/mcp-clients` | Create, list, and revoke MCP tokens |
| `/services` | Connect or remove `web_search`, `shop`, `flights`, `markets` |
| `/actions` | Read-only action catalog |

Unauthenticated visits to protected pages redirect to `/login`. The UI talks to `VITE_API_URL`, which defaults to `http://localhost:7010`, and sends cookies with `credentials: "include"`.

Service credentials are a provider name plus a JSON object. They are encrypted before storage and are not returned by the API.

## MCP

Only `search` and `execute` are exposed. There are no resources, prompts, or extra tools. Missing or unknown bearer tokens get HTTP 401.

`search` takes an optional `query` and `limit` (1–50, default 10). An empty `enabled_actions` list on the client means every action is enabled. Otherwise results are filtered to that list.

`execute` takes `action`, an optional `input` object, and an optional `confirm` boolean. Mutating actions fail with `confirmation_required` unless `confirm` is true or `input.confirm` is true.

Each execute attempt is recorded in `action_runs`.

## Actions

| id | effect | requires |
| --- | --- | --- |
| `web.search` | read | `web_search` |
| `web.fetch` | read | none |
| `shop.search` | read | `shop` |
| `shop.quote` | read | `shop` |
| `shop.checkout` | mutating | `shop` |
| `flights.search` | read | `flights` |
| `flights.quote` | read | `flights` |
| `flights.book` | mutating | `flights` |
| `markets.quote` | read | `markets` |
| `markets.candles` | read | `markets` |
| `markets.order` | mutating | `markets` |

Provider resolution:

- A stored connection whose provider is `mock` runs the mock.
- No connection and `AGENT_KIT_ALLOW_MOCK=true` runs the mock.
- Anything else returns `provider_not_configured`.

`web.fetch` does not use a service connection. It fetches the URL directly after an SSRF check, with a 10 second timeout and a 2 MiB body cap. It stores the raw body in the blob store.

## Safety

- `web.fetch` accepts only `http` and `https`, and refuses loopback, private, link-local, multicast, and cloud metadata addresses.
- `shop.checkout` and `flights.book` may return a mock hold only. They do not collect card numbers or call a payment API.
- `markets.order` never places a live order. After confirmation it returns `live_trading_disabled`.
- MCP tokens and session tokens are stored as SHA-256 hashes. The raw MCP token is returned once at creation.
- Do not log tokens, passwords, or credentials.

## Further reading

`docs/CONTRACTS.md` is the frozen interface contract: crate APIs, schema, routes, and action schemas. `docs/PLAN.md` is the product and layout plan.
