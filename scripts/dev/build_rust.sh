#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

require_cmd cargo

echo "building gatewayd..."
cargo build --package gatewayd

echo "building wasm filters..."
WASM_FILTERS="tenant-filter auth-filter header-filter rate-limit-filter observe-filter"
for filter in $WASM_FILTERS; do
    echo "  building $filter..."
    cargo build \
        --target wasm32-unknown-unknown \
        --profile wasm-release \
        --package "$filter"
done

echo "copying wasm binaries to fixtures..."
WASM_OUT="target/wasm32-unknown-unknown/wasm-release"
PLUGINS_DIR="fixtures/revisions/local-dev-001/plugins"
for filter in $WASM_FILTERS; do
    binary="${filter//-/_}.wasm"
    cp "$WASM_OUT/$binary" "$PLUGINS_DIR/$filter.wasm"
    echo "  copied $filter.wasm"
done

echo "build complete."
