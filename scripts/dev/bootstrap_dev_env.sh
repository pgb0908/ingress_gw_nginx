#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

install_rust() {
  if [[ -x "$CARGO_HOME/bin/rustup" ]]; then
    echo "rustup already installed at $CARGO_HOME/bin/rustup"
    return
  fi

  local installer="$CACHE_DIR/rustup-init.sh"
  curl -L https://sh.rustup.rs -o "$installer"
  chmod +x "$installer"
  CARGO_HOME="$CARGO_HOME" RUSTUP_HOME="$RUSTUP_HOME" sh "$installer" -y --default-toolchain "$RUST_TOOLCHAIN" --profile minimal
}

install_targets() {
  require_cmd rustup
  for target in $RUST_TARGETS; do
    rustup target add "$target"
  done
}

install_wasmx() {
  if [[ -x "$PROJECT_NGINX_BIN" ]]; then
    echo "wasmx nginx already installed at $PROJECT_NGINX_BIN"
    return
  fi

  local archive="$CACHE_DIR/$WASMX_ARCHIVE"
  local unpack_dir="$CACHE_DIR/wasmx-unpack"
  rm -rf "$unpack_dir"
  mkdir -p "$unpack_dir"
  curl -L "$WASMX_DOWNLOAD_URL" -o "$archive"
  tar -xzf "$archive" -C "$unpack_dir"

  local extracted
  extracted="$(find "$unpack_dir" -maxdepth 1 -mindepth 1 -type d | head -n 1)"
  mkdir -p "$PROJECT_NGINX_DIR"
  cp -R "$extracted"/. "$PROJECT_NGINX_DIR"/
}

echo "bootstrapping local development environment into $LOCAL_DIR"
install_rust
install_targets
install_wasmx
echo "bootstrap complete"
