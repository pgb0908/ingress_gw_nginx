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
TARGET="x86_64-unknown-linux-musl"
WASM_TARGET="wasm32-unknown-unknown"
WASM_FILTERS="tenant-filter auth-filter header-filter rate-limit-filter observe-filter"
BUILD_SHA="$(git -C "$ROOT_DIR" rev-parse --short=12 HEAD 2>/dev/null || printf 'unknown')"
BUILD_TIME="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
ARCHIVE="$DIST_DIR/gateway.tar.gz"
STAGING="$DIST_DIR/.staging/gateway"

if ! command -v musl-gcc >/dev/null 2>&1; then
  echo "error: musl-gcc not found. run: sudo apt-get install -y musl-tools" >&2
  exit 1
fi

# ── 빌드 ─────────────────────────────────────────────────────────────────────
echo "building gatewayd (release, $TARGET) ..."
GATEWAY_BUILD_SHA="$BUILD_SHA" \
GATEWAY_BUILD_TIME="$BUILD_TIME" \
cargo build --release --package gatewayd --target "$TARGET"

echo "building wasm filters ..."
for filter in $WASM_FILTERS; do
  echo "  $filter"
  cargo build --target "$WASM_TARGET" --profile wasm-release --package "$filter"
done

NGINX_BIN="${GATEWAY_NGINX_BIN:-$LOCAL_DIR/wasmx/nginx}"
if [[ ! -x "$NGINX_BIN" ]]; then
  echo "error: nginx not found at $NGINX_BIN" >&2
  echo "run bin/gateway-dev bootstrap or set GATEWAY_NGINX_BIN" >&2
  exit 1
fi

# ── 스테이징 ──────────────────────────────────────────────────────────────────
echo "staging ..."
rm -rf "$DIST_DIR/.staging"
mkdir -p "$STAGING/plugins" "$STAGING/config"

cp "$CARGO_TARGET_DIR/$TARGET/release/gatewayd" "$STAGING/gatewayd"
cp "$NGINX_BIN"                                 "$STAGING/nginx"
cp "$ROOT_DIR/scripts/run.sh"                   "$STAGING/run.sh"
cp "$ROOT_DIR/scripts/stop.sh"                  "$STAGING/stop.sh"
cp "$ROOT_DIR/scripts/deploy.sh"                "$STAGING/deploy.sh"
chmod +x "$STAGING/gatewayd" "$STAGING/nginx" "$STAGING/run.sh" "$STAGING/stop.sh" "$STAGING/deploy.sh"

for filter in $WASM_FILTERS; do
  cp "$CARGO_TARGET_DIR/$WASM_TARGET/wasm-release/${filter//-/_}.wasm" "$STAGING/plugins/$filter.wasm"
done

cp -r "$ROOT_DIR/samples/." "$STAGING/config/"

cat > "$STAGING/BUILD_INFO" <<EOF
build_sha=$BUILD_SHA
build_time=$BUILD_TIME
EOF

# ── 아카이브 ──────────────────────────────────────────────────────────────────
mkdir -p "$DIST_DIR"
tar -czf "$ARCHIVE" -C "$DIST_DIR/.staging" gateway
rm -rf "$DIST_DIR/.staging"

echo ""
echo "done: $ARCHIVE (sha=$BUILD_SHA)"
echo ""
echo "사용법:"
echo "  tar -xzf gateway.tar.gz"
echo "  cd gateway"
echo "  ./run.sh"
echo "  ./deploy.sh config/basic"
