#!/bin/sh
set -eu

cd /app
cargo build -p agent-kit
./target/debug/agent-kit migrate
exec ./target/debug/agent-kit serve
