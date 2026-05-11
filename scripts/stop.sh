#!/usr/bin/env bash
set -euo pipefail

GATEWAY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

ADMIN_PID_FILE="$GATEWAY_ROOT/runtime/process/admin.pid"
NGINX_RUNTIME="$GATEWAY_ROOT/runtime/dataplane/nginx"
NGINX_CONF="$NGINX_RUNTIME/conf/nginx.conf"

# gatewayd 종료
if [[ -f "$ADMIN_PID_FILE" ]]; then
  pid="$(cat "$ADMIN_PID_FILE")"
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid"
    echo "stopped gatewayd (pid=$pid)"
  fi
  rm -f "$ADMIN_PID_FILE"
fi

# nginx 종료: run.sh와 동일한 탐색 순서
if [[ -x "$GATEWAY_ROOT/nginx" ]]; then
  NGINX_BIN="$GATEWAY_ROOT/nginx"
elif [[ -x "$GATEWAY_ROOT/config/nginx" ]]; then
  NGINX_BIN="$GATEWAY_ROOT/config/nginx"
elif [[ -x "$GATEWAY_ROOT/bin/nginx" ]]; then
  NGINX_BIN="$GATEWAY_ROOT/bin/nginx"
else
  NGINX_BIN=""
fi

NGINX_PID_FILE="$NGINX_RUNTIME/logs/nginx.pid"
if [[ -f "$NGINX_PID_FILE" ]]; then
  nginx_pid="$(cat "$NGINX_PID_FILE")"
  if [[ -n "$NGINX_BIN" ]]; then
    "$NGINX_BIN" -p "$NGINX_RUNTIME" -c "$NGINX_CONF" -s quit 2>/dev/null && echo "stopped nginx (pid=$nginx_pid)"
  else
    kill "$nginx_pid" 2>/dev/null && echo "stopped nginx (pid=$nginx_pid)"
  fi
fi
