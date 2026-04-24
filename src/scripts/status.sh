#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

echo "[processes]"
if [[ -f "$(admin_pid_file)" ]]; then
  admin_pid="$(cat "$(admin_pid_file)")"
  if kill -0 "$admin_pid" 2>/dev/null; then
    printf 'admin_pid=%s\n' "$admin_pid"
  else
    echo "admin_pid=stopped"
  fi
else
  echo "admin_pid=stopped"
fi

if [[ -f "$(upstream_pid_file)" ]]; then
  upstream_pid="$(cat "$(upstream_pid_file)")"
  if kill -0 "$upstream_pid" 2>/dev/null; then
    printf 'upstream_pid=%s\n' "$upstream_pid"
  else
    echo "upstream_pid=stopped"
  fi
else
  echo "upstream_pid=stopped"
fi

if [[ -f "$(nginx_pid_file)" ]]; then
  nginx_pid="$(cat "$(nginx_pid_file)")"
  if kill -0 "$nginx_pid" 2>/dev/null; then
    printf 'nginx_pid=%s\n' "$nginx_pid"
  else
    echo "nginx_pid=stopped"
  fi
else
  echo "nginx_pid=stopped"
fi

echo
echo "[gateway]"
"$(gateway_bin)" status || true
