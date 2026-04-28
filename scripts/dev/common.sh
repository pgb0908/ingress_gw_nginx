#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export ROOT_DIR
source "$ROOT_DIR/env/dev-env.env"

export LOCAL_DIR="$ROOT_DIR/env/local"
export CACHE_DIR="$ROOT_DIR/env/cache"
export RUN_DIR="$ROOT_DIR/runtime/process"
export RUNTIME_DIR="$ROOT_DIR/runtime/dataplane"
export RUNTIME_REVISIONS_DIR="$ROOT_DIR/runtime/revisions"
export CURRENT_REVISION_LINK="$ROOT_DIR/runtime/current"
export FIXTURE_REVISIONS_DIR="$ROOT_DIR/fixtures/revisions"
export CARGO_HOME="$LOCAL_DIR/cargo"
export RUSTUP_HOME="$LOCAL_DIR/rustup"
export CARGO_TARGET_DIR="$ROOT_DIR/target"
export PATH="$LOCAL_DIR/bin:$CARGO_HOME/bin:$PATH"

export PROJECT_NGINX_DIR="$LOCAL_DIR/wasmx"
export PROJECT_NGINX_BIN="$PROJECT_NGINX_DIR/nginx"
export GATEWAY_NGINX_BIN="${GATEWAY_NGINX_BIN:-$PROJECT_NGINX_BIN}"

mkdir -p "$LOCAL_DIR/bin" "$CACHE_DIR" "$RUN_DIR" "$RUNTIME_DIR" "$RUNTIME_REVISIONS_DIR"

admin_pid_file() {
  printf '%s\n' "$RUN_DIR/admin.pid"
}

upstream_pid_file() {
  printf '%s\n' "$RUN_DIR/upstream.pid"
}

nginx_pid_file() {
  printf '%s\n' "$RUNTIME_DIR/nginx/logs/nginx.pid"
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

sample_revision_name() {
  printf '%s\n' "local-dev-001"
}

sample_fixture_revision_dir() {
  printf '%s\n' "$FIXTURE_REVISIONS_DIR/$(sample_revision_name)"
}

sample_runtime_revision_dir() {
  printf '%s\n' "$RUNTIME_REVISIONS_DIR/$(sample_revision_name)"
}

stage_sample_revision() {
  local source_dir target_dir
  source_dir="$(sample_fixture_revision_dir)"
  target_dir="$(sample_runtime_revision_dir)"
  mkdir -p "$target_dir"
  cp -R "$source_dir"/. "$target_dir"/
}
