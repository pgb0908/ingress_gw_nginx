#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

stage_sample_revision
load_revision "$(sample_runtime_revision_dir)"
