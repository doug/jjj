#!/usr/bin/env bash
# Shared helpers for UXR scenario scripts.
#
# Source this file at the top of each scenario:
#   source "$(dirname "$0")/../lib.sh"

set -euo pipefail

# --- Configuration ---

# Find the jjj binary (prefer release, fall back to debug, fall back to PATH)
JJJ="${JJJ_BIN:-}"
if [[ -z "$JJJ" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    if [[ -x "$REPO_ROOT/target/release/jjj" ]]; then
        JJJ="$REPO_ROOT/target/release/jjj"
    elif [[ -x "$REPO_ROOT/target/debug/jjj" ]]; then
        JJJ="$REPO_ROOT/target/debug/jjj"
    else
        JJJ="jjj"
    fi
fi
export JJJ

# Test workspace root
UXR_TMPDIR="${UXR_TMPDIR:-/tmp/jjj-uxr-$$}"

# Counters
_PASS=0
_FAIL=0
_SKIP=0
_STEP=0
_SCENARIO=""

# Colors (disable if not a terminal)
if [[ -t 1 ]]; then
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    YELLOW='\033[0;33m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    GREEN='' RED='' YELLOW='' CYAN='' BOLD='' RESET=''
fi

# --- Setup / Teardown ---

# Create a fresh jj+jjj repo in a temp directory.
# Usage: setup_repo "dirname"
# Sets CWD to the new repo.
setup_repo() {
    local name="${1:-repo}"
    local dir="$UXR_TMPDIR/$name"
    mkdir -p "$dir"
    cd "$dir"

    # Initialize git + jj
    git init -q .
    git commit -q --allow-empty -m "initial"
    jj git init --colocate 2>/dev/null || true

    # Set a fake user identity for reproducibility
    git config user.name "UXR Test"
    git config user.email "uxr@test.local"
}

# Start a named scenario
begin_scenario() {
    _SCENARIO="$1"
    _STEP=0
    echo ""
    echo -e "${BOLD}${CYAN}=== SCENARIO: $_SCENARIO ===${RESET}"
    echo ""
}

# Print final results for this scenario
end_scenario() {
    echo ""
    echo -e "${BOLD}--- Results: $_SCENARIO ---${RESET}"
    echo -e "  ${GREEN}PASS: $_PASS${RESET}  ${RED}FAIL: $_FAIL${RESET}  ${YELLOW}SKIP: $_SKIP${RESET}"
    echo ""
}

# Clean up temp files (call in trap or at end)
cleanup() {
    if [[ "${UXR_KEEP_TMPDIR:-}" != "1" ]]; then
        rm -rf "$UXR_TMPDIR"
    fi
}

# --- Assertions ---

# Run a jjj command and capture output. Always succeeds (we check separately).
# Usage: run_jjj problem list --status open
# Sets: $OUTPUT (stdout+stderr), $EXIT_CODE
run_jjj() {
    _STEP=$((_STEP + 1))
    local cmd="$JJJ $*"
    echo -e "  ${CYAN}[$_STEP]${RESET} \$ jjj $*"
    set +e
    OUTPUT=$($JJJ "$@" 2>&1)
    EXIT_CODE=$?
    set -e
}

# Assert the last command succeeded (exit 0)
assert_success() {
    local msg="${1:-command should succeed}"
    if [[ $EXIT_CODE -eq 0 ]]; then
        echo -e "    ${GREEN}PASS${RESET} $msg"
        _PASS=$((_PASS + 1))
    else
        echo -e "    ${RED}FAIL${RESET} $msg (exit code: $EXIT_CODE)"
        echo "    OUTPUT: $(echo "$OUTPUT" | head -5)"
        _FAIL=$((_FAIL + 1))
    fi
}

# Assert the last command failed (non-zero exit)
assert_failure() {
    local msg="${1:-command should fail}"
    if [[ $EXIT_CODE -ne 0 ]]; then
        echo -e "    ${GREEN}PASS${RESET} $msg"
        _PASS=$((_PASS + 1))
    else
        echo -e "    ${RED}FAIL${RESET} $msg (expected failure but got exit 0)"
        _FAIL=$((_FAIL + 1))
    fi
}

# Assert output contains a substring
assert_contains() {
    local needle="$1"
    local msg="${2:-output should contain '$needle'}"
    if [[ "$OUTPUT" == *"$needle"* ]]; then
        echo -e "    ${GREEN}PASS${RESET} $msg"
        _PASS=$((_PASS + 1))
    else
        echo -e "    ${RED}FAIL${RESET} $msg"
        echo "    EXPECTED to contain: $needle"
        echo "    ACTUAL (first 5 lines): $(echo "$OUTPUT" | head -5)"
        _FAIL=$((_FAIL + 1))
    fi
}

# Assert output does NOT contain a substring
assert_not_contains() {
    local needle="$1"
    local msg="${2:-output should not contain '$needle'}"
    if [[ "$OUTPUT" == *"$needle"* ]]; then
        echo -e "    ${RED}FAIL${RESET} $msg"
        echo "    FOUND unwanted: $needle"
        _FAIL=$((_FAIL + 1))
    else
        echo -e "    ${GREEN}PASS${RESET} $msg"
        _PASS=$((_PASS + 1))
    fi
}

# Assert output matches a regex
assert_matches() {
    local pattern="$1"
    local msg="${2:-output should match /$pattern/}"
    if echo "$OUTPUT" | grep -qE "$pattern"; then
        echo -e "    ${GREEN}PASS${RESET} $msg"
        _PASS=$((_PASS + 1))
    else
        echo -e "    ${RED}FAIL${RESET} $msg"
        echo "    EXPECTED to match: $pattern"
        echo "    ACTUAL (first 5 lines): $(echo "$OUTPUT" | head -5)"
        _FAIL=$((_FAIL + 1))
    fi
}

# Assert output line count
assert_line_count_ge() {
    local expected="$1"
    local msg="${2:-output should have >= $expected lines}"
    local actual
    actual=$(echo "$OUTPUT" | wc -l | tr -d ' ')
    if [[ $actual -ge $expected ]]; then
        echo -e "    ${GREEN}PASS${RESET} $msg ($actual lines)"
        _PASS=$((_PASS + 1))
    else
        echo -e "    ${RED}FAIL${RESET} $msg (got $actual lines, expected >= $expected)"
        _FAIL=$((_FAIL + 1))
    fi
}

# Skip a check with a note
skip() {
    local msg="$1"
    echo -e "    ${YELLOW}SKIP${RESET} $msg"
    _SKIP=$((_SKIP + 1))
}

# Add a UX observation (not pass/fail, just a note for analysis)
observe() {
    local msg="$1"
    echo -e "    ${YELLOW}NOTE${RESET} $msg"
}

# Print section header within a scenario
section() {
    echo ""
    echo -e "  ${BOLD}-- $1 --${RESET}"
}

# Return overall exit code (0 if no failures)
uxr_exit() {
    if [[ $_FAIL -gt 0 ]]; then
        exit 1
    fi
    exit 0
}
