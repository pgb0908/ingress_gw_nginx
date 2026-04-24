#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

"$(gateway_bin)" activate-revision --revision-path "$ROOT_DIR/src/runtime-config/revisions/local-dev-001"
