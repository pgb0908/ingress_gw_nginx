#!/usr/bin/env bash
# Deploy samples/basic resources to a running gatewayd instance.
# Usage: scripts/deploy-sample.sh [admin-url]
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ADMIN="${1:-http://127.0.0.1:19080}"
SAMPLE_DIR="$ROOT_DIR/samples/basic"
LIVE_DIR="$ROOT_DIR/runtime/dataplane/live"
FIXTURES_PLUGINS="$ROOT_DIR/fixtures/revisions/local-dev-001/plugins"

# ── 1. admin API 확인 ─────────────────────────────────────────────────────────
echo "checking admin API at $ADMIN ..."
if ! curl -sf "$ADMIN/status" >/dev/null; then
  echo "error: gatewayd admin API not reachable at $ADMIN" >&2
  echo "start it with: scripts/dev/start_admin.sh" >&2
  exit 1
fi

# ── 2. wasm 바이너리 준비 ──────────────────────────────────────────────────────
if [[ -d "$FIXTURES_PLUGINS" ]]; then
  echo "copying wasm binaries from fixtures to live/plugins/ ..."
  mkdir -p "$LIVE_DIR/plugins"
  cp "$FIXTURES_PLUGINS"/*.wasm "$LIVE_DIR/plugins/"
else
  echo "warning: fixtures plugins not found at $FIXTURES_PLUGINS — skipping wasm copy" >&2
fi

# ── 3. 리소스 배포 ────────────────────────────────────────────────────────────
deploy() {
  local label="$1" file="$2"
  echo "deploying $label ..."
  result="$(curl -sf -X POST "$ADMIN/deploy" \
    -H "Content-Type: application/json" \
    -d @"$file")"
  status="$(printf '%s' "$result" | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4)"
  printf '  status: %s\n' "${status:-unknown}"
  if [[ "${status:-}" == "failed" ]]; then
    printf '%s\n' "$result" | grep -o '"message":"[^"]*"' | head -1 >&2
    exit 1
  fi
}

deploy "Gateway"  "$SAMPLE_DIR/gateway.json"
deploy "Listener" "$SAMPLE_DIR/listener.json"
deploy "Service"  "$SAMPLE_DIR/service.json"
deploy "Router"   "$SAMPLE_DIR/router.json"

# ── 4. 테스트 curl 예시 ───────────────────────────────────────────────────────
LISTENER_PORT=18000
echo ""
echo "deployed. test with:"
echo "  curl -v http://127.0.0.1:${LISTENER_PORT}/test"
echo ""
echo "note: samples/basic/service.json의 backend가 실제로 응답해야 합니다."
