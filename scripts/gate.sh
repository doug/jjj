#!/usr/bin/env bash
# The full pre-commit gate, with every stage's verdict in the exit code.
#
# Written after a gate run reported EXIT=0 over two clippy errors. The previous
# incantation piped clippy through `rg '^error'` to keep the log short, so the
# pipeline's status was ripgrep's and `EXIT` only ever carried `cargo test`'s.
# The errors were printed and the gate still said it passed — a check that
# reports a fault and then reports success cannot gate anything, which is the
# same defect `jjj doctor` was fixed for.
set -uo pipefail

fail=0
run() {
    local name="$1"
    shift
    if "$@"; then
        echo "GATE_OK=$name"
    else
        echo "GATE_FAIL=$name($?)"
        fail=1
    fi
}

cargo fmt --all
run fmt cargo fmt --all --check
run invariants ./scripts/check-invariants.sh
run clippy cargo clippy --all-targets --all-features -- -D warnings
run tests cargo test --all-targets --no-fail-fast

echo "GATE_EXIT=$fail"
exit "$fail"
