#!/usr/bin/env bash
set -euo pipefail

GATEWAY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export GATEWAY_ROOT

GATEWAYD="$GATEWAY_ROOT/bin/gatewayd"
export GATEWAY_NGINX_BIN="$GATEWAY_ROOT/bin/nginx"

RUN_DIR="$GATEWAY_ROOT/runtime/process"
REVISIONS_DIR="$GATEWAY_ROOT/runtime/revisions"
REVISION_PATH="${1:-}"

if [[ ! -x "$GATEWAYD" ]]; then
  echo "error: $GATEWAYD not found or not executable" >&2; exit 1
fi
if [[ ! -x "$GATEWAY_NGINX_BIN" ]]; then
  echo "error: $GATEWAY_NGINX_BIN not found or not executable" >&2; exit 1
fi

mkdir -p "$RUN_DIR" "$REVISIONS_DIR"

if [[ -f "$RUN_DIR/admin.pid" ]]; then
  old_pid="$(cat "$RUN_DIR/admin.pid")"
  if kill -0 "$old_pid" 2>/dev/null; then
    kill "$old_pid"
    sleep 1
  fi
  rm -f "$RUN_DIR/admin.pid"
fi

echo "starting gatewayd (GATEWAY_ROOT=$GATEWAY_ROOT) ..."
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
place a revision bundle under:
  $REVISIONS_DIR/<revision-name>
then activate it with:
  $GATEWAYD activate-revision --revision-path $REVISIONS_DIR/<revision-name>
EOF
fi

echo "done. stop: $GATEWAY_ROOT/stop.sh"
