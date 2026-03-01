#!/usr/bin/env bash
# Scenario 14: Milestone Status
#
# Tests the `milestone status` command:
#
#   milestone status        (text output with completion stats)
#   milestone status --json (structured JSON output)
#   milestone status        (empty milestone — 0 / 0 handled gracefully)
#
# Tests: 0% at start, percentage after solving, JSON fields, empty milestone

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Milestone Status"

# ============================================================================
section "Setup"
# ============================================================================

setup_repo "milestone-status"
run_jjj init
assert_success "init"

# ============================================================================
section "Step 1: Create milestone with 3 problems"
# ============================================================================

run_jjj milestone new "Beta Release" --date "2026-12-01"
assert_success "create milestone"
assert_contains "Beta Release" "milestone title in output"

run_jjj problem new "Fix login crash" --priority critical --force
assert_success "create problem 1"
run_jjj problem new "Add rate limiting" --priority high --force
assert_success "create problem 2"
run_jjj problem new "Improve error messages" --priority medium --force
assert_success "create problem 3"

run_jjj milestone add-problem "Beta Release" "Fix login crash"
assert_success "add problem 1 to milestone"
run_jjj milestone add-problem "Beta Release" "Add rate limiting"
assert_success "add problem 2 to milestone"
run_jjj milestone add-problem "Beta Release" "Improve error messages"
assert_success "add problem 3 to milestone"

observe "Milestone has 3 problems, all open"

# ============================================================================
section "Step 2: milestone status shows 0% at start"
# ============================================================================

run_jjj milestone status "Beta Release"
assert_success "milestone status at 0%"
assert_contains "3 total" "total problem count shown"
assert_contains "0% complete" "0% completion"
assert_contains "Beta Release" "milestone title in output"
assert_contains "2026-12-01" "target date shown"

observe "milestone status shows correct 0% when no problems are solved"

# ============================================================================
section "Step 3: Solve 1 problem → 33%"
# ============================================================================

run_jjj solution new "Add nil guard to auth handler" --problem "Fix login crash" --force
assert_success "create solution for crash fix"
run_jjj solution submit "Add nil guard"
assert_success "review solution"
run_jjj solution approve "Add nil guard" --no-rationale
assert_success "accept solution"
run_jjj problem solve "Fix login crash"
assert_success "solve first problem"

run_jjj milestone status "Beta Release"
assert_success "milestone status after 1 solved"
assert_contains "1 solved" "solved count updated"
assert_contains "33% complete" "33% completion after 1/3 solved"

observe "Completion percentage updates correctly as problems are solved"

# ============================================================================
section "Step 4: Dissolve a problem → counts toward completion"
# ============================================================================

run_jjj problem dissolve "Improve error messages" --reason "Out of scope for beta"
assert_success "dissolve third problem"

run_jjj milestone status "Beta Release"
assert_success "milestone status after dissolve"
assert_contains "66% complete" "66% after 1 solved + 1 dissolved of 3"

observe "Dissolved problems count toward milestone completion"

# ============================================================================
section "Step 5: milestone status --json shows structured data"
# ============================================================================

run_jjj milestone status "Beta Release" --json
assert_success "milestone status --json"
assert_contains "\"title\"" "JSON has title field"
assert_contains "\"total\"" "JSON has total field"
assert_contains "\"solved\"" "JSON has solved field"
assert_contains "\"dissolved\"" "JSON has dissolved field"
assert_contains "\"pct_complete\"" "JSON has pct_complete field"
assert_contains "\"target_date\"" "JSON has target_date field"
assert_contains "\"days_remaining\"" "JSON has days_remaining field"
assert_contains "\"in_progress\"" "JSON has in_progress field"
assert_contains "\"open\"" "JSON has open field"

# Verify actual values
assert_contains "\"total\": 3" "total is 3"
assert_contains "\"solved\": 1" "solved count is 1"
assert_contains "\"dissolved\": 1" "dissolved count is 1"
assert_contains "\"pct_complete\": 66" "pct_complete is 66"
assert_contains "2026-12-01" "target date in JSON"

observe "JSON output provides machine-readable milestone completion data"

# ============================================================================
section "Step 6: Empty milestone handles 0/0 gracefully"
# ============================================================================

run_jjj milestone new "Future Plans"
assert_success "create empty milestone"

run_jjj milestone status "Future Plans"
assert_success "milestone status on empty milestone"
assert_contains "0 total" "total is 0"
assert_contains "0% complete" "0% on empty milestone"

run_jjj milestone status "Future Plans" --json
assert_success "empty milestone status --json"
assert_contains "\"total\": 0" "total is 0 in JSON"
assert_contains "\"pct_complete\": 0" "pct_complete is 0 in JSON"

observe "milestone status handles milestones with no problems without error"

# ============================================================================
end_scenario
uxr_exit
