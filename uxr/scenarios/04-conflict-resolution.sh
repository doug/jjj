#!/usr/bin/env bash
# Scenario 04: Conflict Resolution
#
# Simulates two users (Alice and Bob) working in the same repo
# and creating conflicting changes to the same entities. Tests
# how jjj handles concurrent edits to frontmatter and files.
#
# This tests the shadow-graph merge behavior when two users
# modify the same problem/solution/critique files.
#
# Tests: concurrent edits, frontmatter conflicts, merge behavior,
#        data integrity after conflict

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Conflict Resolution"

# ============================================================================
section "Setup: Create Shared Project"
# ============================================================================

# Create the "origin" repo
setup_repo "conflict-origin"
run_jjj init
assert_success "init origin"

# Create initial data
$JJJ problem new "Shared problem" --priority p1 2>/dev/null
$JJJ solution new "Initial approach" --problem "Shared" 2>/dev/null
$JJJ critique new "Initial approach" "Needs tests" --severity medium 2>/dev/null

# Record the problem ID for later verification
PROBLEM_TITLE="Shared problem"
run_jjj problem show "Shared"
assert_success "initial problem exists"

echo "  (shared project created with 1 problem, 1 solution, 1 critique)"

# ============================================================================
section "Test 1: Sequential Edits (No Conflict)"
# ============================================================================

# Alice edits the problem title
run_jjj problem edit "Shared" --title "Shared problem (updated by Alice)"
assert_success "alice edits problem title"

# Bob edits the problem priority (different field)
run_jjj problem edit "updated by Alice" --priority p0
assert_success "bob edits problem priority"

# Verify both edits persisted
run_jjj problem show "updated by Alice"
assert_success "problem still accessible"
assert_contains "p0" "bob's priority edit persisted"
assert_contains "Alice" "alice's title edit persisted"

# ============================================================================
section "Test 2: Concurrent Critique Resolution"
# ============================================================================

# Two critiques on the same solution
$JJJ critique new "Initial approach" "Missing error handling" --severity high 2>/dev/null
$JJJ critique new "Initial approach" "No input validation" --severity medium 2>/dev/null

# Address them in rapid succession (simulating concurrent activity)
run_jjj critique address "error handling"
assert_success "address first critique"

run_jjj critique address "input validation"
assert_success "address second critique"

# Verify both are addressed
run_jjj critique list --solution "Initial" --status open
assert_success "list open critiques"
# Only the original "Needs tests" should still be open
observe "Remaining open critiques: $OUTPUT"

# ============================================================================
section "Test 3: Edit Same Entity Back-to-Back"
# ============================================================================

# Problem may already be in_progress (auto-transition from solution creation)
run_jjj problem show "Alice"
observe "Problem status before edits: $OUTPUT"

# Edit back to open, then to in_progress
run_jjj problem edit "Alice" --status open
assert_success "move problem back to open"

run_jjj problem edit "Alice" --status in_progress
assert_success "alice moves to in_progress"

# Verify the entity is still consistent
run_jjj problem show "Alice"
assert_success "problem still consistent"
assert_contains "in_progress" "status is in_progress"

# ============================================================================
section "Test 4: Competing Solutions for Same Problem"
# ============================================================================

$JJJ problem new "Performance issue" --priority p1 2>/dev/null

# Alice proposes one solution
run_jjj solution new "Add caching layer" --problem "Performance"
assert_success "alice proposes caching"

# Bob proposes a different solution to the same problem
run_jjj solution new "Optimize database queries" --problem "Performance"
assert_success "bob proposes query optimization"

# Both should coexist
run_jjj solution list --problem "Performance"
assert_success "list solutions for performance"
assert_contains "caching" "alice's solution exists"
assert_contains "database" "bob's solution exists"

# Submit both solutions for review
run_jjj solution submit "caching"
assert_success "submit caching for review"
run_jjj solution submit "database"
assert_success "submit database for review"

# Charlie critiques alice's solution
run_jjj critique new "caching" "Cache invalidation is hard" --severity high
assert_success "critique on caching solution"

# Alice tries to approve her solution (should be blocked)
run_jjj solution approve "caching" --no-rationale
assert_failure "cannot approve with open critique"

# Bob's solution has no critiques, can be approved
run_jjj solution approve "database" --no-rationale
assert_success "approve bob's uncontested solution"

# ============================================================================
section "Test 5: Concurrent Entity Creation"
# ============================================================================

# Simulate rapid creation of multiple entities
run_jjj problem new "Bug A" --priority p3
assert_success "create bug A"

run_jjj problem new "Bug B" --priority p3
assert_success "create bug B"

run_jjj problem new "Bug C" --priority p3
assert_success "create bug C"

# All three should exist with unique IDs
run_jjj problem list
assert_success "list all problems"
assert_contains "Bug A" "bug A exists"
assert_contains "Bug B" "bug B exists"
assert_contains "Bug C" "bug C exists"

# ============================================================================
section "Test 6: Delete and Re-create"
# ============================================================================

# Create a problem, delete it, create one with similar name
run_jjj problem new "Temporary problem"
assert_success "create temporary"

run_jjj problem show "Temporary"
assert_success "show temporary"

# Delete it (dissolve, since there's no delete command in the CLI)
run_jjj problem dissolve "Temporary" --reason "Was a duplicate"
assert_success "dissolve temporary problem"

# Create new one with similar name
run_jjj problem new "Temporary issue (new)"
assert_success "create new temporary"

# Both should exist (dissolved and new)
run_jjj problem list
assert_success "list shows both"

# ============================================================================
section "Test 7: Frontmatter Integrity After Many Edits"
# ============================================================================

# Create a problem and edit it many times
run_jjj problem new "Stress test entity" --priority p3
assert_success "create stress test entity"

run_jjj problem edit "Stress test" --priority p2
assert_success "edit 1: priority to medium"

run_jjj problem edit "Stress test" --priority p1
assert_success "edit 2: priority to high"

run_jjj problem edit "Stress test" --title "Stress test entity (v3)"
assert_success "edit 3: update title"

run_jjj problem edit "Stress test" --priority p0
assert_success "edit 4: priority to critical"

# Verify final state is consistent
run_jjj problem show "Stress test"
assert_success "show after many edits"
assert_contains "p0" "final priority is p0"
assert_contains "v3" "final title has v3"

# ============================================================================
section "Test 8: Solution State Machine Under Concurrent Pressure"
# ============================================================================

$JJJ problem new "State machine test" --priority p2 2>/dev/null
run_jjj solution new "SM solution" --problem "State machine"
assert_success "create solution for state test"

# Add and address a critique
run_jjj critique new "SM solution" "Issue found" --severity low
assert_success "add critique"

run_jjj critique address "Issue found"
assert_success "address critique"

# Submit then approve solution
run_jjj solution submit "SM solution"
assert_success "submit SM solution for review"
run_jjj solution approve "SM solution" --no-rationale
assert_success "approve solution"

# Double-approve should be rejected
run_jjj solution approve "SM solution" --no-rationale
assert_failure "double-approve is rejected"

# Try invalid transitions
run_jjj solution edit "SM solution" --status proposed
assert_failure "can't go from approved back to proposed"

# ============================================================================
section "Test 9: Cascade Effects"
# ============================================================================

# Create a problem with a solution that has critiques
$JJJ problem new "Cascade test" --priority p3 2>/dev/null
$JJJ solution new "Cascade solution" --problem "Cascade test" 2>/dev/null
$JJJ critique new "Cascade solution" "Cascade critique" --severity low 2>/dev/null

# Dissolve the problem - what happens to solution and critique?
run_jjj problem dissolve "Cascade test" --reason "Testing cascade"
assert_success "dissolve problem with children"

# Check that entities still exist (dissolved, not deleted)
run_jjj problem show "Cascade test"
assert_success "dissolved problem still accessible"
assert_contains "dissolved" "problem is dissolved"

# ============================================================================
end_scenario
uxr_exit
