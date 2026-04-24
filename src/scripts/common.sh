#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export ROOT_DIR
source "$ROOT_DIR/config/dev-env.env"

export LOCAL_DIR="$ROOT_DIR/.local"
export RUN_DIR="$ROOT_DIR/.run"
export CARGO_HOME="$LOCAL_DIR/cargo"
export RUSTUP_HOME="$LOCAL_DIR/rustup"
export CARGO_TARGET_DIR="$ROOT_DIR/target"
export PATH="$LOCAL_DIR/bin:$CARGO_HOME/bin:$PATH"

export PROJECT_NGINX_DIR="$LOCAL_DIR/wasmx"
export PROJECT_NGINX_BIN="$PROJECT_NGINX_DIR/nginx"
export GATEWAY_NGINX_BIN="${GATEWAY_NGINX_BIN:-$PROJECT_NGINX_BIN}"

mkdir -p "$LOCAL_DIR/bin" "$RUN_DIR"

admin_pid_file() {
  printf '%s\n' "$RUN_DIR/admin.pid"
}

upstream_pid_file() {
  printf '%s\n' "$RUN_DIR/upstream.pid"
}

nginx_pid_file() {
  printf '%s\n' "$ROOT_DIR/src/gateway/runtime/nginx/logs/nginx.pid"
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing command: $1" >&2
    return 1
  fi
}

gateway_bin() {
  printf '%s\n' "$CARGO_TARGET_DIR/debug/gatewayd"
}

upstream_bin() {
  printf '%s\n' "$CARGO_TARGET_DIR/debug/mock_upstream"
}
