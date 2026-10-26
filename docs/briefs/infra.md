# infra

Read `docs/PLAN.md` and `docs/CONTRACTS.md`. Implement only the files you own.

## Own

- `Cargo.toml` workspace listing every member in the contract
- `rust-toolchain.toml` channel stable
- `.gitignore` for Rust targets, `node_modules`, `dist`, `.env`, `.env.compose.local`, local blob dirs
- `.env.example` and `.env.compose` from the env table. Host DB URL uses `localhost:5433`. In-compose URL uses `postgres://agentkit:agentkit@db:5432/agentkit`
- `Makefile` with `help`, `install`, `migrate`, `run`, `stop`, `reset`, `ps`, `logs`, `test`, `lint`, `check`
- `docker-compose.yaml` and `docker/app-dev.sh`
- `crates/agent-kit/` binary only

## Do not own

Frontend Dockerfile, frontend package files, any other crate.

## Binary

Implement `migrate` and `serve` exactly as the contract describes. Depend on the other crates' public functions even if those crates are still being written. Do not copy their logic into the binary.

`make run` is `docker compose up --build`. `make reset` must ask for confirmation before deleting volumes. `make test` runs `cargo test --workspace` and `$(MAKE) -C frontend test`. `make lint` runs `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings`, plus frontend lint.

App dev command migrates then serves. Do not bind a different host port than 7010.

You may `cargo check -p agent-kit` once other crates exist. If they do not yet, still write the binary against the contract. Do not start compose.

Do not commit.
