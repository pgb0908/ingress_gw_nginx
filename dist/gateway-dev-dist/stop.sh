#!/usr/bin/env bash
set -euo pipefail

GATEWAY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

ADMIN_PID_FILE="$GATEWAY_ROOT/runtime/process/admin.pid"
NGINX_BIN="$GATEWAY_ROOT/bin/nginx"
NGINX_RUNTIME="$GATEWAY_ROOT/runtime/dataplane/nginx"
NGINX_CONF="$NGINX_RUNTIME/conf/nginx.conf"

if [[ -f "$ADMIN_PID_FILE" ]]; then
  pid="$(cat "$ADMIN_PID_FILE")"
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid"
    echo "stopped gatewayd (pid=$pid)"
  fi
  rm -f "$ADMIN_PID_FILE"
fi

if [[ -f "$NGINX_RUNTIME/logs/nginx.pid" ]]; then
  "$NGINX_BIN" -p "$NGINX_RUNTIME" -c "$NGINX_CONF" -s quit 2>/dev/null && echo "stopped nginx"
fi
