#!/usr/bin/env bash
# Scenario 17: Solution Diff
#
# Tests the `solution diff` command:
#
#   solution diff <unknown>    (error for unknown solution)
#   solution diff <id>         (no change IDs → informative message)
#   solution diff <id>         (with change ID → shows diff header)
#
# Tests: error handling, empty change_ids message, diff header output

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Solution Diff"

# ============================================================================
section "Setup"
# ============================================================================

setup_repo "solution-diff"
run_jjj init
assert_success "init"

run_jjj problem new "Rendering lag on large datasets" --priority p1 --force
assert_success "create problem"

# ============================================================================
section "Step 1: solution diff on unknown solution returns error"
# ============================================================================

run_jjj solution diff "totally-nonexistent-solution-xyz"
assert_failure "diff of unknown solution fails"

observe "solution diff correctly errors on non-existent solution"

# ============================================================================
section "Step 2: solution with no change IDs shows informative message"
# ============================================================================

run_jjj solution new "Virtualise row rendering" --problem "Rendering lag" --force
assert_success "create solution (auto-attaches current change)"

# Detach the auto-attached change so we can test the empty case
CHANGE_ID="$(jj log --no-graph -r @ -T 'change_id' 2>/dev/null | head -1)"
observe "Auto-attached change ID: $CHANGE_ID"

run_jjj solution detach "Virtualise row rendering" --force
assert_success "detach change to leave empty change_ids"

run_jjj solution diff "Virtualise row rendering"
assert_success "diff with no change IDs succeeds"
assert_contains "No change IDs" "informative message when no changes attached"

observe "solution diff handles empty change_ids gracefully"

# ============================================================================
section "Step 3: solution with a change ID shows diff header"
# ============================================================================

run_jjj solution attach "Virtualise row rendering"
assert_success "re-attach current change to solution"
assert_contains "Attached" "attach confirmed in output"

run_jjj solution diff "Virtualise row rendering"
assert_success "diff with attached change ID"
assert_contains "=== Change:" "diff header present for attached change"
assert_contains "$CHANGE_ID" "change ID appears in diff header"

observe "solution diff shows '=== Change: <id> ===' header for each attached change"

# ============================================================================
end_scenario
uxr_exit
