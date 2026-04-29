#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

# Returns comma-separated listening ports for a given PID, or empty string
pid_ports() {
  local pid="$1"
  ss -tlnp 2>/dev/null \
    | awk -v p="pid=${pid}," '$NF ~ p { sub(/.*:/, "", $4); print $4 }' \
    | sort -un \
    | paste -sd,
}

print_process_status() {
  local name="$1" pid_val="$2"
  local ports
  if [[ "$pid_val" == "stopped" ]]; then
    printf '%s=stopped\n' "$name"
    return
  fi
  ports="$(pid_ports "$pid_val")"
  if [[ -n "$ports" ]]; then
    printf '%s=%s  port=%s\n' "$name" "$pid_val" "$ports"
  else
    printf '%s=%s\n' "$name" "$pid_val"
  fi
}

resolve_pid() {
  local pid_file="$1"
  if [[ ! -f "$pid_file" ]]; then echo "stopped"; return; fi
  local pid
  pid="$(cat "$pid_file")"
  if kill -0 "$pid" 2>/dev/null; then echo "$pid"; else echo "stopped"; fi
}

echo "[processes]"
print_process_status "admin_pid"    "$(resolve_pid "$(admin_pid_file)")"
print_process_status "upstream_pid" "$(resolve_pid "$(upstream_pid_file)")"
print_process_status "nginx_pid"    "$(resolve_pid "$(nginx_pid_file)")"

echo
echo "[gateway]"
"$(gateway_bin)" status || true
