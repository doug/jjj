#!/usr/bin/env bash
# Scenario 15: Assignee Workflow
#
# Tests the assign commands for problems, solutions, and milestones:
#
#   problem assign <id> --to <name>    (assign to named person)
#   problem assign <id>                (assign to self — jj identity)
#   solution assign <id> --to <name>   (assign solution)
#   milestone assign <id> --to <name>  (assign milestone)
#   problem list --assignee <name>     (filter by assignee)
#   solution list --assignee <name>    (filter by assignee)
#   problem assign <id> --to <other>   (reassign)
#
# Self-assign uses the real jj user.name from the environment (not the git
# config set by setup_repo), so identity assertions use $SELF_SUBSTR which
# is discovered at runtime via `jj config get user.name`.

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Assignee Workflow"

# ============================================================================
section "Step 1: Setup"
# ============================================================================

setup_repo "assignee-workflow"
run_jjj init
assert_success "init"

# Discover the actual jj user identity so assertions are portable across machines.
# jj's user_identity() returns "name <email>"; capture just the name for matching.
SELF_NAME="$(jj config get user.name 2>/dev/null || echo "Unknown")"
# Use the name as the substring we assert on (avoids angle-bracket quoting issues)
SELF_SUBSTR="$SELF_NAME"
observe "Detected jj self identity name: $SELF_SUBSTR"

# Create three problems with different priorities
run_jjj problem new "Database connection pool exhausted" --priority p0 --force
assert_success "create problem 1 (critical)"
assert_contains "Database connection" "problem title in output"

run_jjj problem new "Search results are slow" --priority p1 --force
assert_success "create problem 2 (high)"
assert_contains "Search results" "problem title in output"

run_jjj problem new "Avatar upload fails silently" --priority p2 --force
assert_success "create problem 3 (medium)"
assert_contains "Avatar upload" "problem title in output"

# Create two solutions linked to problems
run_jjj solution new "Use connection pooling library" --problem "Database connection" --force
assert_success "create solution 1"

run_jjj solution new "Add search result caching" --problem "Search results" --force
assert_success "create solution 2"

# Create a milestone
run_jjj milestone new "v2.0 Release" --date "2026-06-01"
assert_success "create milestone"
assert_contains "v2.0" "milestone title in output"

observe "Setup complete: 3 problems, 2 solutions, 1 milestone — ready to test assignee workflow"

# ============================================================================
section "Step 2: Assign problems and solutions"
# ============================================================================

# Assign problem 1 to alice
run_jjj problem assign "Database connection" --to alice
assert_success "assign problem 1 to alice"
assert_contains "alice" "alice mentioned in assign output"

# Assign solution 1 to bob
run_jjj solution assign "connection pooling" --to bob
assert_success "assign solution 1 to bob"
assert_contains "bob" "bob mentioned in assign output"

# Assign problem 2 to self (no --to → uses jj identity)
run_jjj problem assign "Search results are slow"
assert_success "assign problem 2 to self (no --to)"
assert_contains "$SELF_SUBSTR" "self-identity used when --to omitted"

# Assign milestone to alice
run_jjj milestone assign "v2.0" --to alice
assert_success "assign milestone to alice"
assert_contains "alice" "alice mentioned in milestone assign output"

observe "problem assign with no --to uses the jj user.name (not git config user.name)"
observe "milestone assign shares the same pattern as problem/solution assign"

# ============================================================================
section "Step 3: Verify assignees in show output"
# ============================================================================

run_jjj problem show "Database connection" --json
assert_success "show problem 1 (assigned to alice)"
assert_contains '"alice"' "alice is the assignee in JSON output"

run_jjj solution show "connection pooling" --json
assert_success "show solution 1 (assigned to bob)"
assert_contains '"bob"' "bob is the assignee in JSON output"

run_jjj problem show "Search results" --json
assert_success "show problem 2 (assigned to self)"
assert_contains "\"$SELF_SUBSTR" "self is the assignee in JSON output"

run_jjj milestone show "v2.0" --json
assert_success "show milestone (assigned to alice)"
assert_contains '"alice"' "alice is the milestone assignee in JSON output"

observe "All entity types (problem, solution, milestone) persist assignee through save/load"

# ============================================================================
section "Step 4: Filter with --assignee"
# ============================================================================

# problem list --assignee alice should return problem 1 (Database connection)
run_jjj problem list --assignee alice
assert_success "problem list --assignee alice"
assert_contains "Database connection" "alice's problem appears in filtered list"
assert_not_contains "Search results" "self-assigned problem not in alice's list"
assert_not_contains "Avatar upload" "unassigned problem not in alice's list"

# problem list --assignee $SELF_NAME should return problem 2 (Search results)
run_jjj problem list --assignee "$SELF_NAME"
assert_success "problem list --assignee (self name)"
assert_contains "Search results" "self-assigned problem appears in filtered list"
assert_not_contains "Database connection" "alice's problem not in self list"

# solution list --assignee bob should return solution 1
run_jjj solution list --assignee bob
assert_success "solution list --assignee bob"
assert_contains "connection pooling" "bob's solution appears in filtered list"
assert_not_contains "search result caching" "unassigned solution not in bob's list"

observe "--assignee filter uses case-insensitive substring matching"
observe "Partial names like 'alice' match the stored assignee value"

# ============================================================================
section "Step 5: jjj next --mine (shows work for current user)"
# ============================================================================

# next --mine shows all open TODO problems (no active solutions) regardless of
# assignee.  It suppresses REVIEW critique items not assigned to the current
# user.  Problem 3 (Avatar upload, unassigned) will appear as a TODO.
run_jjj next --mine
assert_success "next --mine exits cleanly"
assert_contains "Avatar upload" "unassigned open problem appears as TODO in next --mine"

observe "next --mine does not filter problems by assignee — use 'problem list --assignee' for that"
observe "next --mine suppresses REVIEW critique items for reviewers other than self"

# ============================================================================
section "Step 6: Reassign to a different person"
# ============================================================================

# Reassign problem 1 from alice to charlie
run_jjj problem assign "Database connection" --to charlie
assert_success "reassign problem 1 from alice to charlie"
assert_contains "charlie" "charlie mentioned in reassign output"

run_jjj problem show "Database connection" --json
assert_success "show problem 1 after reassignment"
assert_contains '"charlie"' "charlie is now the assignee"
assert_not_contains '"alice"' "alice is no longer the assignee"

# Reassign solution from bob to self (no --to)
run_jjj solution assign "connection pooling"
assert_success "reassign solution to self (no --to)"
assert_contains "$SELF_SUBSTR" "self-identity applied in reassign"

run_jjj solution show "connection pooling" --json
assert_success "show solution after reassign to self"
assert_contains "\"$SELF_SUBSTR" "self is now the solution assignee"
assert_not_contains '"bob"' "bob is no longer the assignee"

observe "Reassignment overwrites the previous assignee with no confirmation required"

# ============================================================================
section "Step 7: Assignee appears in non-JSON show output"
# ============================================================================

run_jjj problem show "Database connection"
assert_success "plain-text show of assigned problem"
assert_contains "Assignee" "Assignee label present in plain-text output"
assert_contains "charlie" "assignee name present in plain-text output"

run_jjj solution show "connection pooling"
assert_success "plain-text show of assigned solution"
assert_contains "Assignee" "Assignee label present in solution plain-text output"
assert_contains "$SELF_SUBSTR" "self-assignee name present in plain-text output"

observe "Assignee field renders in both --json and plain-text output modes"

# ============================================================================
end_scenario
uxr_exit
