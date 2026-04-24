#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

mkdir -p "$RUN_DIR"

if [[ ! -x "$PROJECT_NGINX_BIN" ]]; then
  echo "project nginx binary not found: $PROJECT_NGINX_BIN" >&2
  echo "run: bin/gateway-dev bootstrap" >&2
  exit 1
fi

if [[ ! -x "$(gateway_bin)" ]] || [[ ! -x "$(upstream_bin)" ]]; then
  "$ROOT_DIR/src/scripts/build_rust.sh"
fi

if [[ ! -f "$(admin_pid_file)" ]] || ! kill -0 "$(cat "$(admin_pid_file)")" 2>/dev/null; then
  nohup "$(gateway_bin)" serve-admin --host 127.0.0.1 --port 19080 >"$RUN_DIR/admin.log" 2>&1 &
  admin_pid="$!"
  printf '%s\n' "$admin_pid" >"$(admin_pid_file)"
fi

if [[ ! -f "$(upstream_pid_file)" ]] || ! kill -0 "$(cat "$(upstream_pid_file)")" 2>/dev/null; then
  nohup "$(upstream_bin)" >"$RUN_DIR/upstream.log" 2>&1 &
  upstream_pid="$!"
  printf '%s\n' "$upstream_pid" >"$(upstream_pid_file)"
fi

"$(gateway_bin)" activate-revision --revision-path "$ROOT_DIR/src/runtime-config/revisions/local-dev-001"
echo "gateway-dev up complete"
