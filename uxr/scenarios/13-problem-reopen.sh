#!/usr/bin/env bash
# Scenario 13: Problem Reopen
#
# Tests the `problem reopen` command:
#
#   problem reopen          (reopen a solved problem)
#   problem reopen          (reopen a dissolved problem)
#   problem reopen (error)  (already-open problem is rejected)
#   timeline                (problem_reopened event appears)
#
# Tests: reopen from solved, reopen from dissolved, error on open problem,
#        event logging

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Problem Reopen"

# ============================================================================
section "Setup"
# ============================================================================

setup_repo "problem-reopen"
run_jjj init
assert_success "init"

# ============================================================================
section "Step 1: Create and solve a problem, then verify it disappears from open list"
# ============================================================================

run_jjj problem new "Login times out after idle" --priority high --force
assert_success "create problem"
assert_contains "Login times out" "problem title in output"

# Solve the problem: need a reviewed+accepted solution first
run_jjj solution new "Add session keepalive" --problem "Login times out" --force
assert_success "create solution"

run_jjj solution submit "Add session keepalive"
assert_success "review solution"

run_jjj solution approve "Add session keepalive" --no-rationale
assert_success "accept solution"

run_jjj problem solve "Login times out"
assert_success "solve problem"
assert_contains "solved" "problem marked as solved"

run_jjj problem list
assert_success "list problems"
assert_not_contains "Login times out" "solved problem not in default open list"

run_jjj problem list --status solved
assert_success "list solved problems"
assert_contains "Login times out" "solved problem appears with --status solved"

observe "Solved problems are hidden from the default list — reopen brings them back"

# ============================================================================
section "Step 2: Reopen the solved problem"
# ============================================================================

run_jjj problem reopen "Login times out"
assert_success "reopen solved problem"
assert_contains "reopened" "output confirms reopen"

run_jjj problem list
assert_success "list after reopen"
assert_contains "Login times out" "reopened problem back in open list"

run_jjj problem show "Login times out"
assert_success "show reopened problem"
assert_contains "open" "status is now open"
assert_not_contains "solved" "status is no longer solved"

observe "problem reopen transitions solved → open with no extra steps"

# ============================================================================
section "Step 3: Dissolve a problem and then reopen it"
# ============================================================================

run_jjj problem new "Confusing error messages" --priority medium --force
assert_success "create second problem"

run_jjj problem dissolve "Confusing error" --reason "Error messages were already clarified in v1.2"
assert_success "dissolve problem"
assert_contains "dissolved" "problem marked as dissolved"

run_jjj problem list --status dissolved
assert_success "list dissolved problems"
assert_contains "Confusing error" "dissolved problem in list"

run_jjj problem reopen "Confusing error"
assert_success "reopen dissolved problem"
assert_contains "reopened" "output confirms reopen"

run_jjj problem show "Confusing error"
assert_success "show after reopen from dissolved"
assert_contains "open" "status is now open"

observe "problem reopen works from both solved and dissolved states"

# ============================================================================
section "Step 4: Reopening an already-open problem should fail"
# ============================================================================

run_jjj problem new "Another open issue" --priority low --force
assert_success "create open problem"

run_jjj problem reopen "Another open"
assert_failure "reopen of already-open problem fails"
assert_contains "open" "error mentions current status"

observe "Reopening an already-open problem is correctly rejected with a clear error"

# ============================================================================
section "Step 5: Timeline shows problem_reopened event"
# ============================================================================

run_jjj timeline "Login times out"
assert_success "timeline command"
assert_contains "problem reopened" "reopened event in timeline"
assert_contains "problem solved" "original solved event also in timeline"

run_jjj timeline "Confusing error"
assert_success "timeline for dissolved+reopened problem"
assert_contains "problem reopened" "reopened event in timeline"
assert_contains "problem dissolved" "original dissolved event in timeline"

observe "Timeline shows the full history including reopen events"

# ============================================================================
end_scenario
uxr_exit
