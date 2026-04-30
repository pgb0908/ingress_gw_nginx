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
NGINX_BIN="${GATEWAY_RELEASE_NGINX_BIN:-$LOCAL_DIR/wasmx/nginx}"
BUILD_SHA="$(git -C "$ROOT_DIR" rev-parse --short=12 HEAD 2>/dev/null || printf 'unknown')"
BUILD_TIME="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
VERSION="$(cargo metadata --no-deps --format-version 1 | sed -n 's/.*"name":"gatewayd","version":"\([^"]*\)".*/\1/p' | head -n 1)"

if [[ ! -x "$NGINX_BIN" ]]; then
  echo "error: wasmx nginx not found at $NGINX_BIN" >&2
  echo "set GATEWAY_RELEASE_NGINX_BIN or run: bin/gateway-dev bootstrap" >&2
  exit 1
fi

echo "building gatewayd (release)..."
export GATEWAY_BUILD_SHA="$BUILD_SHA"
export GATEWAY_BUILD_TIME="$BUILD_TIME"
cargo build --release --package gatewayd

echo "staging $STAGING ..."
rm -rf "$STAGING"
mkdir -p "$STAGING/bin" "$STAGING/revisions" "$STAGING/config"

cp "$CARGO_TARGET_DIR/release/gatewayd"                        "$STAGING/bin/gatewayd"
cp "$NGINX_BIN"                                                 "$STAGING/bin/nginx"
cp "$ROOT_DIR/scripts/run.sh"                                   "$STAGING/run.sh"
cp "$ROOT_DIR/scripts/stop.sh"                                  "$STAGING/stop.sh"
cat > "$STAGING/VERSION" <<EOF
${VERSION:-unknown}
EOF
cat > "$STAGING/BUILD_INFO" <<EOF
version=${VERSION:-unknown}
build_sha=$BUILD_SHA
build_time=$BUILD_TIME
platform=${WASMX_PLATFORM:-unknown}
runtime=${WASMX_RUNTIME:-unknown}
EOF
chmod +x "$STAGING/run.sh" "$STAGING/stop.sh"

mkdir -p "$DIST_DIR"
echo "creating $ARCHIVE ..."
tar -czf "$ARCHIVE" -C "$DIST_DIR" "gateway-dev-dist"

echo "done: $ARCHIVE"
