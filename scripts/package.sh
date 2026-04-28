#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/env/dev-env.env"

LOCAL_DIR="$ROOT_DIR/env/local"
export CARGO_HOME="$LOCAL_DIR/cargo"
export RUSTUP_HOME="$LOCAL_DIR/rustup"
export CARGO_TARGET_DIR="$ROOT_DIR/target"
export PATH="$LOCAL_DIR/bin:$CARGO_HOME/bin:$PATH"

DIST_DIR="$ROOT_DIR/dist"
STAGING="$DIST_DIR/gateway-dev-dist"
ARCHIVE="$DIST_DIR/gateway-dev-dist.tar.gz"
NGINX_BIN="$LOCAL_DIR/wasmx/nginx"

if [[ ! -x "$NGINX_BIN" ]]; then
  echo "error: wasmx nginx not found at $NGINX_BIN" >&2
  echo "run: bin/gateway-dev bootstrap" >&2
  exit 1
fi

echo "building gatewayd (release)..."
cargo build --release --package gatewayd

echo "staging $STAGING ..."
rm -rf "$STAGING"
mkdir -p "$STAGING/bin" "$STAGING/revisions"

cp "$CARGO_TARGET_DIR/release/gatewayd"                        "$STAGING/bin/gatewayd"
cp "$NGINX_BIN"                                                 "$STAGING/bin/nginx"
cp -R "$ROOT_DIR/fixtures/revisions/local-dev-001"             "$STAGING/revisions/"
cp "$ROOT_DIR/scripts/run.sh"                                   "$STAGING/run.sh"
cp "$ROOT_DIR/scripts/stop.sh"                                  "$STAGING/stop.sh"
chmod +x "$STAGING/run.sh" "$STAGING/stop.sh"

mkdir -p "$DIST_DIR"
echo "creating $ARCHIVE ..."
tar -czf "$ARCHIVE" -C "$DIST_DIR" "gateway-dev-dist"

echo "done: $ARCHIVE"
