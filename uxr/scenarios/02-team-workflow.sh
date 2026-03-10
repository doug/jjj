#!/usr/bin/env bash
# Scenario 02: Team Workflow
#
# Simulates a team lead (Bob) managing a sprint with three members.
# Bob creates milestones/problems, Alice proposes solutions, Charlie
# reviews and raises critiques. Tests the full team lifecycle.
#
# Tests: milestones, assign, reviewers, competing solutions, withdraw,
#        critique blocking, roadmap, events

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Team Workflow"

# ============================================================================
section "Setup: Initialize Project"
# ============================================================================

setup_repo "team-project"
run_jjj init
assert_success "init"

# ============================================================================
section "Bob: Create Milestone and Problems"
# ============================================================================

run_jjj milestone new "v1.0 Sprint" --date 2025-06-01
assert_success "create milestone"
assert_contains "v1.0 Sprint" "milestone title in output"

run_jjj problem new "User login broken" --priority critical
assert_success "create critical problem"

run_jjj problem new "Search is slow" --priority high
assert_success "create high-priority problem"

run_jjj problem new "Dashboard crashes" --priority medium
assert_success "create medium-priority problem"

# Add to milestone
run_jjj milestone add-problem "v1.0" "login"
assert_success "add problem to milestone by partial title"

run_jjj milestone add-problem "v1.0" "Search"
assert_success "add second problem to milestone"

run_jjj milestone add-problem "v1.0" "Dashboard"
assert_success "add third problem to milestone"

# Verify milestone shows all problems
run_jjj milestone show "v1.0"
assert_success "milestone show"
assert_contains "login" "milestone lists login problem"

# ============================================================================
section "Bob: Assign Problems"
# ============================================================================

run_jjj problem assign "login" --to alice
assert_success "assign login to alice"

run_jjj problem assign "Search" --to bob
assert_success "assign search to bob"

run_jjj problem assign "Dashboard" --to charlie
assert_success "assign dashboard to charlie"

# ============================================================================
section "Alice: Propose Solutions"
# ============================================================================

run_jjj solution new "Fix OAuth token refresh" --problem "login" --reviewer @bob
assert_success "alice creates solution with reviewer"

run_jjj solution new "Add elasticsearch" --problem "Search"
assert_success "alice creates search solution"

# ============================================================================
section "Bob: Propose Competing Solution"
# ============================================================================

run_jjj solution new "Use simple SQL LIKE search" --problem "Search"
assert_success "bob creates competing solution"

# Both solutions should appear
run_jjj solution list --problem "Search"
assert_success "list solutions for search"
assert_contains "elasticsearch" "first solution listed"
assert_contains "SQL LIKE" "second solution listed"

# ============================================================================
section "Charlie: Raise Critiques"
# ============================================================================

run_jjj critique new "OAuth token" "Token refresh doesn't handle clock skew" --severity high
assert_success "charlie critiques OAuth solution"

run_jjj critique new "OAuth token" "Missing rate limiting on refresh endpoint" --severity medium
assert_success "charlie adds second critique"

run_jjj critique new "elasticsearch" "Elasticsearch requires Java runtime" --severity high
assert_success "charlie critiques elasticsearch solution"

# ============================================================================
section "Check: Approval Should Be Blocked"
# ============================================================================

# Submit so approval attempts hit critique check (not state check)
run_jjj solution submit "OAuth token"
assert_success "submit OAuth solution for review"

run_jjj solution approve "OAuth token" --no-rationale
assert_failure "cannot approve with open critiques"
assert_contains "critique" "error mentions critiques"

# ============================================================================
section "Alice: Address Critiques"
# ============================================================================

run_jjj critique address "clock skew"
assert_success "address clock skew critique"

run_jjj critique address "rate limiting"
assert_success "address rate limiting critique"

# Now the reviewer critique from bob may still be open
run_jjj critique list --solution "OAuth"
assert_success "list critiques for OAuth solution"
observe "Remaining critiques: $OUTPUT"

# ============================================================================
section "Bob: Withdraw Elasticsearch, Approve OAuth"
# ============================================================================

# Withdraw the heavier solution
run_jjj solution withdraw "elasticsearch" --rationale "Too heavyweight for our needs" --no-rationale
assert_success "withdraw elasticsearch solution"

# Check it's withdrawn
run_jjj solution show "elasticsearch"
assert_success "show withdrawn solution"
assert_contains "withdrawn" "solution is withdrawn"

# Address the review critique (bob's sign-off)
run_jjj critique list --solution "OAuth" --status open
observe "Open critiques before approve: $OUTPUT"

# Try to approve OAuth (may need to address review critique first)
run_jjj solution approve "OAuth token" --no-rationale
if [[ $EXIT_CODE -ne 0 ]]; then
    observe "Approve failed, likely review critique still open"
    # Force approve as the reviewer
    run_jjj solution approve "OAuth token" --force --no-rationale
    assert_success "force approve with review critique"
else
    assert_success "approve OAuth solution"
fi

# ============================================================================
section "Verify Auto-Solve and Check Milestone"
# ============================================================================

# Problem should have been auto-solved when solution was approved
run_jjj problem show "login"
assert_success "show login problem"
assert_contains "solved" "login problem auto-solved after approve"

run_jjj milestone roadmap
assert_success "milestone roadmap"
assert_contains "v1.0" "roadmap shows milestone"

# ============================================================================
section "Events Audit Trail"
# ============================================================================

run_jjj events
assert_success "events command"
assert_line_count_ge 5 "at least 5 events recorded"

# Filter by event type
run_jjj events --event-type problem_created
assert_success "events filtered by type"

# ============================================================================
section "Status Overview"
# ============================================================================

run_jjj status
assert_success "status overview"
observe "Final status: $OUTPUT"

# ============================================================================
section "Problem Hierarchy"
# ============================================================================

# Create a sub-problem
run_jjj problem new "Fix OAuth for mobile" --parent "login"
assert_success "create sub-problem"

run_jjj problem tree "login"
assert_success "show problem tree"
assert_contains "mobile" "tree shows sub-problem"

# ============================================================================
section "Dissolve a Problem"
# ============================================================================

run_jjj problem dissolve "Dashboard" --reason "Turned out to be a browser caching issue"
assert_success "dissolve problem with reason"

run_jjj problem show "Dashboard"
assert_success "show dissolved problem"
assert_contains "dissolved" "problem shows dissolved status"

# ============================================================================
end_scenario
uxr_exit
