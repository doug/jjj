#!/usr/bin/env bash
# Scenario 05: Error Recovery and Edge Cases
#
# Simulates a user making mistakes, encountering errors, and
# trying to recover. Tests error message quality, edge cases,
# and graceful degradation.
#
# Tests: invalid inputs, state violations, empty repos, boundary
#        conditions, error message helpfulness

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Error Recovery and Edge Cases"

# ============================================================================
section "Running jjj Outside a Repository"
# ============================================================================

cd /tmp
run_jjj status
assert_failure "status outside repo fails"
observe "Error message outside repo: $OUTPUT"

run_jjj problem list
assert_failure "problem list outside repo fails"

# ============================================================================
section "Setup: Fresh Repo"
# ============================================================================

setup_repo "error-recovery"
run_jjj init
assert_success "init"

# ============================================================================
section "Empty State Commands"
# ============================================================================

# These should all work gracefully on an empty project
run_jjj status
assert_success "status on empty project"

run_jjj problem list
assert_success "problem list on empty project"
assert_contains "No problems" "helpful empty message"

run_jjj solution list
assert_success "solution list on empty project"

run_jjj critique list
assert_success "critique list on empty project"

run_jjj milestone list
assert_success "milestone list on empty project"

run_jjj events
assert_success "events on empty project"

run_jjj milestone roadmap
assert_success "roadmap on empty project"

run_jjj problem tree
assert_success "problem tree on empty project"

# ============================================================================
section "Invalid Inputs"
# ============================================================================

# Empty title should be rejected
run_jjj problem new ""
assert_failure "empty title rejected"

# Very long title
LONG_TITLE=$(python3 -c "print('x' * 500)")
run_jjj problem new "$LONG_TITLE"
assert_success "very long title accepted (or gracefully handled)"

# Special characters in title
run_jjj problem new 'Fix "quoted" & <special> chars'
assert_success "special characters in title"

# Unicode in title
run_jjj problem new "Fix emoji handling 🎉"
assert_success "unicode in title"

# ============================================================================
section "Invalid Status Transitions"
# ============================================================================

run_jjj problem new "Transition test" --priority p2
assert_success "create test problem"

# Can't go from open to solved without approved solution
run_jjj problem edit "Transition" --status solved
assert_failure "solved requires approved solution"

# After the above edit, try to dissolve (may fail if solved is terminal)
run_jjj problem edit "Transition" --status dissolved
observe "Transition -> dissolved result: exit=$EXIT_CODE output=$OUTPUT"

# ============================================================================
section "Critique Without Solution"
# ============================================================================

# Try to critique a problem (should fail - critiques are for solutions)
run_jjj critique new "Transition test" "This is wrong" --severity low
assert_failure "can't critique a problem directly"
observe "Wrong entity type error: $OUTPUT"

# ============================================================================
section "Duplicate Detection"
# ============================================================================

run_jjj problem new "Exact duplicate test"
assert_success "create first"

# Create exact same title (should reject without --force)
run_jjj problem new "Exact duplicate test"
assert_failure "duplicate title rejected without --force"

# Force create should work
run_jjj problem new "Exact duplicate test" --force
assert_success "duplicate allowed with --force"

# ============================================================================
section "Solution for Non-existent Problem"
# ============================================================================

run_jjj solution new "Orphan solution" --problem "zzz-nonexistent"
assert_failure "solution for nonexistent problem fails"
observe "Orphan solution error: $OUTPUT"

# ============================================================================
section "Critique for Non-existent Solution"
# ============================================================================

run_jjj critique new "zzz-nonexistent" "Ghost critique" --severity low
assert_failure "critique for nonexistent solution fails"
observe "Ghost critique error: $OUTPUT"

# ============================================================================
section "Approve Solution With Open Critiques"
# ============================================================================

$JJJ problem new "Blocked approve test" --priority p3 2>/dev/null
$JJJ solution new "Blocked solution" --problem "Blocked approve" 2>/dev/null
$JJJ solution submit "Blocked solution" 2>/dev/null
$JJJ critique new "Blocked solution" "Blocking critique" --severity high 2>/dev/null

run_jjj solution approve "Blocked solution" --no-rationale
assert_failure "approve blocked by open critique"
assert_contains "critique" "error mentions the blocking critique"

# Force approve should work
run_jjj solution approve "Blocked solution" --force --no-rationale
assert_success "force approve bypasses critique check"

# ============================================================================
section "Double Operations (Idempotency)"
# ============================================================================

$JJJ problem new "Idempotency test" --priority p3 2>/dev/null
$JJJ solution new "Idemp solution" --problem "Idempotency" 2>/dev/null

# Address nonexistent critique
run_jjj critique address "zzz-no-such-critique"
assert_failure "address nonexistent critique fails"

# ============================================================================
section "JSON Output Mode"
# ============================================================================

run_jjj problem list --json
assert_success "problem list JSON"
assert_contains "[" "output is JSON array"

run_jjj solution list --json
assert_success "solution list JSON"

run_jjj milestone list --json
assert_success "milestone list JSON"

run_jjj critique list --json
assert_success "critique list JSON"

# ============================================================================
section "Sort Flags"
# ============================================================================

# Create a few problems for sorting
$JJJ problem new "AAA first alphabetically" --priority p3 2>/dev/null
$JJJ problem new "ZZZ last alphabetically" --priority p0 2>/dev/null

run_jjj problem list --sort title
assert_success "sort by title"

run_jjj problem list --sort priority
assert_success "sort by priority"

run_jjj problem list --sort status
assert_success "sort by status"

run_jjj problem list --sort created
assert_success "sort by created"

# Invalid sort field (should handle gracefully)
run_jjj problem list --sort invalid_field
assert_success "invalid sort field treated as default (no crash)"

# ============================================================================
section "Milestone Edge Cases"
# ============================================================================

# Milestone with no date
run_jjj milestone new "No date milestone"
assert_success "milestone without date"

# Milestone with past date
run_jjj milestone new "Past milestone" --date 2020-01-01
assert_success "milestone with past date (should work)"

# Add same problem to milestone twice
$JJJ problem new "Double add test" --priority p3 2>/dev/null
$JJJ milestone add-problem "No date" "Double add" 2>/dev/null
run_jjj milestone add-problem "No date" "Double add"
assert_success "double add-problem warns but succeeds"

# ============================================================================
section "Search (FTS)"
# ============================================================================

# Search should work (auto-syncs from markdown)
run_jjj search "test"
assert_success "search works without explicit rebuild"

# Rebuild DB
run_jjj db rebuild
assert_success "db rebuild succeeds"

# Search after rebuild
run_jjj search "duplicate"
assert_success "search finds results after rebuild"

# ============================================================================
end_scenario
uxr_exit
