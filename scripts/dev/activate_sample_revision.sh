#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

stage_sample_revision
"$(gateway_bin)" activate-revision --revision-path "$(sample_runtime_revision_dir)"
