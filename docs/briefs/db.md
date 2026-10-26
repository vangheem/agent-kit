# db

Read `docs/PLAN.md` and `docs/CONTRACTS.md`. Implement `crates/agent-kit-db` only.

Own the initial migration and the `Db` methods in the contract. Use `sqlx::query`, not `query!`. `Db` is `Clone`.

Duplicate email is `DbError::Conflict`. Revoked MCP clients and expired sessions are not returned by the lookup methods.

Add one test of token-prefix or conflict mapping that does not need Postgres if you can. A Postgres integration test must be `#[ignore]` and must not start compose.

Do not edit the workspace `Cargo.toml`. Package name `agent-kit-db`. Depend on `agent-kit-core` by path `../agent-kit-core`.

Do not commit.
