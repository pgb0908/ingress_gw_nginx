#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

PASS=0
FAIL=0

run_pkg() {
    local pkg="$1"
    echo ""
    echo "==> cargo test --package ${pkg}"
    if cargo test --package "$pkg" -- --test-output immediate 2>&1; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
    fi
}

run_pkg gatewayd
run_pkg auth-filter
run_pkg rate-limit-filter
run_pkg header-filter

echo ""
echo "-------------------------------"
echo "Results: ${PASS} passed, ${FAIL} failed"
echo "-------------------------------"

if [[ "$FAIL" -gt 0 ]]; then
    exit 1
fi
