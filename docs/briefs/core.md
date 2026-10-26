# core

Read `docs/PLAN.md` and `docs/CONTRACTS.md`. Implement `crates/agent-kit-core` only.

Export the config loader, crypto helpers, blob trait, action trait, catalog, and errors with the signatures in the contract. No database, HTTP, or provider logic.

`MapCatalog::search` must follow the ranking rules in the contract. Add one test that an exact action id returns that action and a token query ranks a matching descriptor first.

`load_config` fails if `DATABASE_URL` or `APP_SECRET` is missing, or if `APP_SECRET` is shorter than 32 bytes.

Do not edit the workspace `Cargo.toml`. Your package name is `agent-kit-core`. Infra adds the workspace member.

Do not commit. Do not start compose.
