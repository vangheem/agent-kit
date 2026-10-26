#!/bin/sh
set -eu

export NPM_CONFIG_REGISTRY="${NPM_REGISTRY:-${NPM_CONFIG_REGISTRY:-https://registry.npmjs.org/}}"

if [ -f package-lock.json ]; then
  lock_hash_file="node_modules/.package-lock.sha256"
  current_lock_hash="$(sha256sum package-lock.json | awk '{print $1}')"
  installed_lock_hash="$(cat "$lock_hash_file" 2>/dev/null || true)"
  if [ ! -x node_modules/.bin/vite ] || [ "$current_lock_hash" != "$installed_lock_hash" ]; then
    npm ci
    mkdir -p node_modules
    printf '%s\n' "$current_lock_hash" > "$lock_hash_file"
  fi
elif [ ! -x node_modules/.bin/vite ]; then
  npm install
fi

exec npm run dev -- --port 4210 --host 0.0.0.0
