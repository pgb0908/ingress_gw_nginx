#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

stop_pid_file() {
  local file="$1"
  if [[ -f "$file" ]]; then
    local pid
    pid="$(cat "$file")"
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" || true
    fi
    rm -f "$file"
  fi
}

stop_pid_file "$(admin_pid_file)"
stop_pid_file "$(upstream_pid_file)"

if [[ -f "$(nginx_pid_file)" ]]; then
  "$PROJECT_NGINX_BIN" -p "$RUNTIME_DIR/nginx" -c "$RUNTIME_DIR/nginx/conf/nginx.conf" -s quit || true
fi

echo "gateway-dev down complete"
