#!/usr/bin/env bash
#
# Is this tree safe to publish?
#
# Runs before every push and every merge of an approved solution, so it is the
# cheap check, not the full suite: does it build, and do the packages most
# likely to be broken by frame-cost work still draw the right pixels?
#
# Golden-image tests are the ones that matter here. An optimisation that skips
# a paint, reuses a buffer it should have cleared, or reorders a layer will
# still compile and still be fast — the pixels are what notice.

set -uo pipefail
cd "$(dirname "$0")"

CGO_ENABLED=0 go build ./... >/dev/null 2>&1 || { echo "verify: build failed" >&2; exit 1; }

# The paint and layout packages, not everything: this has to stay under a
# minute. The full suite runs in score.sh.
CGO_ENABLED=0 go test ./app/... ./internal/gfx/gg ./layout/... ./widget/... \
    >/tmp/verify.err 2>&1 || {
        echo "verify: $(grep -E '^--- FAIL' /tmp/verify.err | head -3 | tr '\n' ' ')" >&2
        exit 1
    }
