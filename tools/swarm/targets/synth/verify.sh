#!/usr/bin/env bash
# Cheap pre-push check: does it still compute the right answer?
set -uo pipefail
cd "$(dirname "$0")"
(cd fixture && python3 measure.py >/dev/null 2>&1) || {
    echo "verify: the pipeline no longer produces the right answer" >&2; exit 1; }
