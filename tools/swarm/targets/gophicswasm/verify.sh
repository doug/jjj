#!/usr/bin/env bash
#
# Is this tree safe to publish?
#
# Runs before every push and every merge, so it is the cheap check: does it
# build for the host and for the web, and do the packages most likely to be
# broken by size work still pass? The full suite and the browser gate live in
# score.sh.

set -uo pipefail
cd "$(dirname "$0")"

CGO_ENABLED=0 go build ./... >/dev/null 2>&1 || { echo "verify: host build failed" >&2; exit 1; }
GOOS=js GOARCH=wasm go build -o /tmp/verify.wasm ./examples/counter >/dev/null 2>&1 \
    || { echo "verify: wasm build failed" >&2; exit 1; }

CGO_ENABLED=0 go test ./app/... ./widget/... ./layout/... ./shell/... >/tmp/verify.err 2>&1 \
    || { echo "verify: $(grep -E '^--- FAIL' /tmp/verify.err | head -3 | tr '\n' ' ')" >&2; exit 1; }
