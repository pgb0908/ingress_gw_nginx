#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

printf 'root=%s\n' "$ROOT_DIR"
printf 'python=%s\n' "$(command -v python3 || true)"
printf 'rustup=%s\n' "$(command -v rustup || true)"
printf 'cargo=%s\n' "$(command -v cargo || true)"
printf 'project_nginx=%s\n' "$PROJECT_NGINX_BIN"
printf 'gateway_bin=%s\n' "$(gateway_bin)"
printf 'upstream_bin=%s\n' "$(upstream_bin)"
printf 'fixture_revision=%s\n' "$(sample_fixture_revision_dir)"
printf 'runtime_revision=%s\n' "$(sample_runtime_revision_dir)"

if [[ -x "$PROJECT_NGINX_BIN" ]]; then
  "$PROJECT_NGINX_BIN" -v || true
else
  echo "project nginx missing"
fi

if command -v cargo >/dev/null 2>&1; then
  cargo --version
else
  echo "cargo missing"
fi

if command -v rustup >/dev/null 2>&1; then
  rustup show active-toolchain || true
else
  echo "rustup missing"
fi
