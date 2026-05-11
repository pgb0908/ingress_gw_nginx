#!/usr/bin/env bash
set -euo pipefail

GATEWAY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export GATEWAY_ROOT

# gatewayd 바이너리: 스크립트와 같은 디렉토리에 있거나 bin/ 하위에 있을 수 있음
if [[ -x "$GATEWAY_ROOT/gatewayd" ]]; then
  GATEWAYD="$GATEWAY_ROOT/gatewayd"
elif [[ -x "$GATEWAY_ROOT/bin/gatewayd" ]]; then
  GATEWAYD="$GATEWAY_ROOT/bin/gatewayd"
else
  echo "error: gatewayd not found in $GATEWAY_ROOT or $GATEWAY_ROOT/bin" >&2; exit 1
fi

# nginx 바이너리: 번들 내 위치 우선, 없으면 환경변수 사용
if [[ -x "$GATEWAY_ROOT/nginx" ]]; then
  export GATEWAY_NGINX_BIN="$GATEWAY_ROOT/nginx"
elif [[ -x "$GATEWAY_ROOT/config/nginx" ]]; then
  export GATEWAY_NGINX_BIN="$GATEWAY_ROOT/config/nginx"
elif [[ -x "$GATEWAY_ROOT/bin/nginx" ]]; then
  export GATEWAY_NGINX_BIN="$GATEWAY_ROOT/bin/nginx"
elif [[ -n "${GATEWAY_NGINX_BIN:-}" ]] && [[ -x "$GATEWAY_NGINX_BIN" ]]; then
  : # 번들에 없을 때만 환경변수 사용
else
  echo "error: nginx not found. place nginx alongside run.sh or set GATEWAY_NGINX_BIN" >&2; exit 1
fi

RUN_DIR="$GATEWAY_ROOT/runtime/process"
REVISIONS_DIR="$GATEWAY_ROOT/runtime/revisions"
REVISION_PATH="${1:-}"

mkdir -p "$RUN_DIR" "$REVISIONS_DIR"

# 기존 gatewayd 정리
if [[ -f "$RUN_DIR/admin.pid" ]]; then
  old_pid="$(cat "$RUN_DIR/admin.pid")"
  if kill -0 "$old_pid" 2>/dev/null; then
    kill "$old_pid"
    sleep 1
  fi
  rm -f "$RUN_DIR/admin.pid"
fi

# 기존 nginx 정리 (이전 배포의 nginx가 남아있을 수 있음)
NGINX_PID_FILE="$GATEWAY_ROOT/runtime/dataplane/nginx/logs/nginx.pid"
if [[ -f "$NGINX_PID_FILE" ]]; then
  nginx_pid="$(cat "$NGINX_PID_FILE")"
  if kill -0 "$nginx_pid" 2>/dev/null; then
    "$GATEWAY_NGINX_BIN" -p "$GATEWAY_ROOT/runtime/dataplane/nginx" -s stop 2>/dev/null || kill "$nginx_pid" 2>/dev/null || true
    sleep 1
  fi
fi

echo "starting gatewayd (GATEWAY_ROOT=$GATEWAY_ROOT) ..."
echo "  gatewayd : $GATEWAYD"
echo "  nginx    : $GATEWAY_NGINX_BIN"
nohup "$GATEWAYD" serve-admin --host 0.0.0.0 --port 19080 \
  >"$RUN_DIR/admin.log" 2>&1 &
printf '%s\n' "$!" >"$RUN_DIR/admin.pid"
echo "admin pid=$(cat "$RUN_DIR/admin.pid"), log=$RUN_DIR/admin.log"

sleep 1

if [[ -n "$REVISION_PATH" ]]; then
  echo "activating revision $REVISION_PATH ..."
  "$GATEWAYD" activate-revision --revision-path "$REVISION_PATH"
else
  cat <<EOF

admin server started.
deploy resources via:
  curl -X POST http://127.0.0.1:19080/deploy -H 'Content-Type: application/json' -d @<resource.json>

or activate a pre-built revision:
  $GATEWAYD activate-revision --revision-path $REVISIONS_DIR/<revision-name>
EOF
fi

echo "done. stop: $GATEWAY_ROOT/stop.sh"
