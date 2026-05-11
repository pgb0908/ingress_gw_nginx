#!/usr/bin/env bash
# Deploy all JSON resources in a directory to gatewayd.
# Usage: deploy.sh <config-dir> [admin-url]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_DIR="${1:-}"
ADMIN="${2:-http://127.0.0.1:19080}"

if [[ -z "$CONFIG_DIR" ]]; then
  echo "usage: $(basename "$0") <config-dir> [admin-url]" >&2
  exit 1
fi
if [[ ! -d "$CONFIG_DIR" ]]; then
  echo "error: directory not found: $CONFIG_DIR" >&2
  exit 1
fi

# admin API 확인
if ! curl -sf "$ADMIN/status" >/dev/null; then
  echo "error: gatewayd not reachable at $ADMIN" >&2
  exit 1
fi

# plugins/ 복사: 스크립트 옆에 plugins/ 디렉토리가 있으면 live/plugins/ 에 복사
PLUGINS_SRC="$SCRIPT_DIR/plugins"
if [[ -d "$PLUGINS_SRC" ]]; then
  # GATEWAY_ROOT: gatewayd 바이너리가 스크립트와 같은 디렉토리에 있다고 가정
  GATEWAY_ROOT="${GATEWAY_ROOT:-$SCRIPT_DIR}"
  LIVE_PLUGINS="$GATEWAY_ROOT/runtime/dataplane/live/plugins"
  echo "copying wasm plugins to $LIVE_PLUGINS ..."
  mkdir -p "$LIVE_PLUGINS"
  cp "$PLUGINS_SRC"/*.wasm "$LIVE_PLUGINS/"
fi

# JSON 파일 배포 (Gateway → Listener → Service → Router → Policy 순)
deploy_file() {
  local file="$1"
  local result status
  result="$(curl -sf -X POST "$ADMIN/deploy" \
    -H "Content-Type: application/json" \
    -d @"$file")"
  status="$(printf '%s' "$result" | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4)"
  printf '  %-10s  %s\n' "${status:-unknown}" "$(basename "$file")"
  if [[ "${status:-}" == "failed" ]]; then
    printf '%s\n' "$result" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('message',''))" 2>/dev/null || true
    exit 1
  fi
}

echo "deploying from $CONFIG_DIR to $ADMIN ..."

for kind in Gateway Listener Service Router Policy; do
  while IFS= read -r -d '' file; do
    kind_in_file="$(grep -o '"kind"[[:space:]]*:[[:space:]]*"[^"]*"' "$file" | head -1 | grep -o '"[^"]*"$' | tr -d '"')"
    if [[ "$kind_in_file" == "$kind" ]]; then
      deploy_file "$file"
    fi
  done < <(find "$CONFIG_DIR" -maxdepth 1 -name "*.json" -print0 | sort -z)
done

echo "done."
