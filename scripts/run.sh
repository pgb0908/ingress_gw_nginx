#!/usr/bin/env bash
set -euo pipefail

GATEWAY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export GATEWAY_ROOT

GATEWAYD="$GATEWAY_ROOT/bin/gatewayd"
export GATEWAY_NGINX_BIN="$GATEWAY_ROOT/bin/nginx"

RUN_DIR="$GATEWAY_ROOT/runtime/process"
REVISIONS_DIR="$GATEWAY_ROOT/runtime/revisions"
SAMPLE_REVISION_NAME="local-dev-001"
SAMPLE_SRC="$GATEWAY_ROOT/revisions/$SAMPLE_REVISION_NAME"
SAMPLE_DEST="$REVISIONS_DIR/$SAMPLE_REVISION_NAME"

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

if [[ ! -d "$SAMPLE_DEST" ]]; then
  echo "staging sample revision $SAMPLE_REVISION_NAME ..."
  cp -R "$SAMPLE_SRC" "$SAMPLE_DEST"
fi

echo "starting gatewayd (GATEWAY_ROOT=$GATEWAY_ROOT) ..."
nohup "$GATEWAYD" serve-admin --host 0.0.0.0 --port 19080 \
  >"$RUN_DIR/admin.log" 2>&1 &
printf '%s\n' "$!" >"$RUN_DIR/admin.pid"
echo "admin pid=$(cat "$RUN_DIR/admin.pid"), log=$RUN_DIR/admin.log"

sleep 1

echo "activating revision $SAMPLE_REVISION_NAME ..."
"$GATEWAYD" activate-revision --revision-path "$SAMPLE_DEST"

echo "done. stop: kill \$(cat $RUN_DIR/admin.pid)"
