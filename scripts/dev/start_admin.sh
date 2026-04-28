#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

"$(gateway_bin)" serve-admin --host 127.0.0.1 --port 19080
