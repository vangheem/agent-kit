.PHONY: help install run build test lint format

PORT ?= 4210

help:
	@printf "Available frontend targets:\n"
	@printf "  make install   Install npm dependencies\n"
	@printf "  make run       Start Vite on port $(PORT)\n"
	@printf "  make build     Build the production frontend\n"
	@printf "  make test      Run Vitest once\n"
	@printf "  make lint      Run Prettier and ESLint checks\n"
	@printf "  make format    Format frontend source\n"

install:
	npm install

run:
	npm run dev -- --port $(PORT) --host 0.0.0.0

build:
	npm run build

test:
	npm test -- --run

lint:
	npm run lint

format:
	npm run format
