#!/usr/bin/env bash
# Scenario 01: Solo Developer Quick Start
#
# Simulates a solo developer named Alice who just discovered jjj
# and follows the Quick Start guide step by step.
#
# Tests: init, problem new, solution new, critique new, critique address,
#        submit, status, entity resolution by title

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Solo Developer Quick Start"

# ============================================================================
section "Step 1: Initialize"
# ============================================================================

setup_repo "alice-solo"

run_jjj init
assert_success "jjj init in fresh repo"
assert_contains "initialized" "init confirms success"

# Double-init should be safe
run_jjj init
assert_failure "double init is rejected"
assert_contains "already" "error mentions already initialized"

# ============================================================================
section "Step 2: Create a Problem"
# ============================================================================

run_jjj problem new "Search is slow" --priority high
assert_success "create problem with --priority high"
assert_contains "Search is slow" "output shows the title"

# Verify it appears in list
run_jjj problem list
assert_success "problem list works"
assert_contains "Search is slow" "problem appears in list"

# ============================================================================
section "Step 3: Propose a Solution"
# ============================================================================

run_jjj solution new "Add search index" --problem "Search is slow"
assert_success "create solution referencing problem by title"
assert_contains "Add search index" "output shows solution title"

# Verify solution appears
run_jjj solution list
assert_success "solution list works"
assert_contains "Add search index" "solution appears in list"

# Check problem auto-transitioned to in_progress
run_jjj problem show "Search is slow"
assert_success "problem show by fuzzy title"
assert_contains "in_progress" "problem auto-moved to in_progress"

# ============================================================================
section "Step 4: Resume Working"
# ============================================================================

run_jjj solution resume "search index"
assert_success "resume solution by partial title"

# ============================================================================
section "Step 5: Add a Critique"
# ============================================================================

run_jjj critique new "search index" "Missing error handling" --severity medium
assert_success "create critique by solution title"
assert_contains "Missing error handling" "output shows critique title"

# Verify critique appears
run_jjj critique list
assert_success "critique list works"
assert_contains "Missing error" "critique appears in list"

# ============================================================================
section "Step 6: Check Status (should show blocked)"
# ============================================================================

run_jjj status
assert_success "status command works"
assert_contains "BLOCKED" "status shows blocked state"

# ============================================================================
section "Step 7: Address the Critique"
# ============================================================================

run_jjj critique address "Missing error"
assert_success "address critique by partial title"

# Status should no longer show blocked
run_jjj status
assert_success "status after addressing"
assert_not_contains "BLOCKED" "no more blocked items"

# ============================================================================
section "Step 8: Accept the Solution"
# ============================================================================

# Accept should work now (no open critiques)
run_jjj solution accept "search index" --no-rationale
assert_success "accept solution with all critiques resolved"

# Problem should auto-transition to solved (only solution)
run_jjj problem show "Search is slow"
assert_success "show problem after accept"
assert_contains "solved" "problem auto-solved after accept"

# ============================================================================
section "Step 9: Entity Resolution Methods"
# ============================================================================

# Create another problem for testing resolution
run_jjj problem new "Authentication is broken" --priority critical
assert_success "create second problem"

# Full title match
run_jjj problem show "Authentication is broken"
assert_success "resolve by full title"

# Partial title match
run_jjj problem show "auth"
assert_success "resolve by partial title 'auth'"
assert_contains "Authentication" "found correct problem"

# Case-insensitive match
run_jjj problem show "AUTHENTICATION"
assert_success "resolve by uppercase title"

# ============================================================================
section "Step 10: Error Handling"
# ============================================================================

# Non-existent entity
run_jjj problem show "zzz-nonexistent-zzz"
assert_failure "show nonexistent problem fails"

# Invalid priority
run_jjj problem new "test" --priority invalid
assert_failure "invalid priority rejected"
assert_contains "Use" "error shows valid values"

# Invalid status transition: solved requires accepted solution
run_jjj problem edit "auth" --status solved
assert_failure "solved requires accepted solution"

# ============================================================================
section "Step 11: Help Discoverability"
# ============================================================================

run_jjj --help
assert_success "--help works"
assert_contains "problem" "help mentions problem command"
assert_contains "solution" "help mentions solution command"
assert_contains "critique" "help mentions critique command"

run_jjj problem --help
assert_success "problem --help works"
assert_contains "new" "help shows new subcommand"
assert_contains "list" "help shows list subcommand"

# ============================================================================
end_scenario
uxr_exit
