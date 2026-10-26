.PHONY: help install migrate serve run stop reset ps logs test lint fmt check
.DEFAULT_GOAL := help

help:
	@printf "Available targets:\n"
	@printf "  make install   Create local env files and install dependencies\n"
	@printf "  make migrate   Run database migrations (uses .env when present)\n"
	@printf "  make serve     Run the API and MCP server on the host\n"
	@printf "  make run       Start the Docker Compose stack\n"
	@printf "  make stop      Stop the Docker Compose stack\n"
	@printf "  make reset     Stop the stack and delete volumes after confirmation\n"
	@printf "  make ps        Show Docker Compose service status\n"
	@printf "  make logs      Follow db, app, and frontend logs\n"
	@printf "  make test      Run workspace tests and frontend tests\n"
	@printf "  make lint      Run Rust fmt/clippy and frontend lint\n"
	@printf "  make fmt       Format Rust and frontend sources\n"
	@printf "  make check     Run cargo check and lint\n"

install:
	touch .env.compose.local
	@if [ ! -f .env ]; then cp .env.example .env; printf "Created .env from .env.example\n"; fi
	$(MAKE) -C frontend install
	cargo fetch

migrate:
	@set -a; \
	if [ -f .env ]; then . ./.env; fi; \
	set +a; \
	cargo run -p agent-kit -- migrate

serve:
	@set -a; \
	if [ -f .env ]; then . ./.env; fi; \
	set +a; \
	cargo run -p agent-kit -- serve

run:
	touch .env.compose.local
	docker compose up --build

stop:
	docker compose down

reset:
	@printf "This will stop the stack and delete Compose volumes. Type RESET to continue: "; \
	read confirm; \
	if [ "$$confirm" != "RESET" ]; then \
		printf "Aborted.\n"; \
		exit 1; \
	fi; \
	docker compose down -v

ps:
	docker compose ps

logs:
	docker compose logs -f db app frontend

test:
	cargo test --workspace
	$(MAKE) -C frontend test

lint:
	cargo fmt --check
	cargo clippy --workspace --all-targets -- -D warnings
	$(MAKE) -C frontend lint

fmt:
	cargo fmt
	$(MAKE) -C frontend format

check:
	cargo check --workspace
	$(MAKE) lint
