#!/usr/bin/env bash
# Scenario 11: Milestone Advanced
#
# Tests all milestone features not covered by the basic team-workflow scenario:
#
#   milestone new          (with and without date)
#   milestone edit         (title, date, status)
#   milestone remove-problem
#   milestone assign
#   milestone roadmap      (progression as problems get solved)
#   problem new --milestone (assign on creation)
#   milestone completion   (all problems solved → milestone completion)
#   milestone show --json  (structured output)
#
# Tests: edit, remove-problem, assign, roadmap progression, --milestone on
#        problem new, lifecycle to completion, JSON output

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Milestone Advanced"

# ============================================================================
section "Setup"
# ============================================================================

setup_repo "milestone-advanced"
run_jjj init
assert_success "init"

# ============================================================================
section "Step 1: milestone new (with and without date)"
# ============================================================================

run_jjj milestone new "v1.0 Launch"
assert_success "create milestone without date"
assert_contains "v1.0" "milestone title in output"

run_jjj milestone new "v1.1 Patch" --date "2026-09-01"
assert_success "create milestone with date"
assert_contains "v1.1" "milestone title in output"

run_jjj milestone list
assert_success "milestone list shows both"
assert_contains "v1.0" "first milestone in list"
assert_contains "v1.1" "second milestone in list"

observe "Milestones can be created with or without a target date for flexible planning"

# ============================================================================
section "Step 2: milestone edit (title, date, status)"
# ============================================================================

run_jjj milestone edit "v1.0 Launch" --title "v1.0 GA Release"
assert_success "edit milestone title"

run_jjj milestone show "v1.0 GA"
assert_success "show milestone after title edit"
assert_contains "v1.0 GA" "new title in output"

run_jjj milestone edit "v1.0 GA" --date "2026-07-15"
assert_success "add date to previously dateless milestone"

run_jjj milestone show "v1.0 GA"
assert_success "show milestone after date edit"
assert_contains "2026-07-15" "date now visible"

run_jjj milestone edit "v1.1 Patch" --status active
assert_success "set milestone status to active"

run_jjj milestone show "v1.1"
assert_success "show milestone after status edit"
assert_contains "active" "active status visible"

observe "milestone edit lets you refine plans as dates and scope become clearer"

# ============================================================================
section "Step 3: problem new --milestone (assign on creation)"
# ============================================================================

run_jjj problem new "Login crashes on empty password" \
    --priority critical \
    --milestone "v1.0 GA"
assert_success "create problem assigned to milestone on creation"

run_jjj problem new "Settings page layout broken" \
    --priority high \
    --milestone "v1.0 GA"
assert_success "create second problem for v1.0 milestone"

run_jjj problem new "Add export to CSV" \
    --priority medium \
    --milestone "v1.1 Patch"
assert_success "create problem for v1.1 milestone"

run_jjj milestone show "v1.0 GA"
assert_success "show v1.0 milestone with problems"
assert_contains "Login crashes" "first problem in milestone"
assert_contains "Settings page" "second problem in milestone"

observe "problem new --milestone saves a step — no need to milestone add-problem separately"

# ============================================================================
section "Step 4: milestone add-problem (explicit)"
# ============================================================================

run_jjj problem new "Dark mode flickers" --priority low
assert_success "create problem without milestone"

run_jjj milestone add-problem "v1.1 Patch" "Dark mode flickers"
assert_success "add problem to milestone explicitly"

run_jjj milestone show "v1.1"
assert_success "show v1.1 milestone"
assert_contains "Dark mode" "added problem visible"

# ============================================================================
section "Step 5: milestone remove-problem"
# ============================================================================

# Remove the low-priority dark mode fix from v1.1 — it can slip
run_jjj milestone remove-problem "v1.1 Patch" "Dark mode flickers"
assert_success "remove problem from milestone"

run_jjj milestone show "v1.1"
assert_success "show v1.1 after remove"
assert_not_contains "Dark mode" "removed problem no longer in milestone"

observe "remove-problem lets you adjust scope without deleting the problem itself"

# ============================================================================
section "Step 6: milestone assign"
# ============================================================================

run_jjj milestone assign "v1.0 GA" --to "alice@example.com"
assert_success "assign milestone to alice"

run_jjj milestone show "v1.0 GA"
assert_success "show milestone after assign"
assert_contains "alice" "assignee visible in milestone"

# ============================================================================
section "Step 7: milestone roadmap"
# ============================================================================

run_jjj milestone roadmap
assert_success "roadmap shows all milestones"
assert_contains "v1.0" "v1.0 in roadmap"
assert_contains "v1.1" "v1.1 in roadmap"
assert_contains "problems solved" "roadmap shows problem progress counts"

observe "roadmap gives a cross-milestone view of what's planned and what's solved"

# ============================================================================
section "Step 8: milestone progression (solve problems → complete milestone)"
# ============================================================================

# Solve the v1.0 problems via solutions
run_jjj solution new "Add nil check before auth" --problem "Login crashes"
assert_success "create solution for crash bug"
run_jjj critique new "Add nil check" "Test coverage missing" --severity low
assert_success "add low-severity critique"
run_jjj critique address "Test coverage"
assert_success "address the critique"
run_jjj solution submit "Add nil check"
assert_success "submit nil check for review"
run_jjj solution approve "Add nil check" --no-rationale
assert_success "approve login crash solution"

run_jjj solution new "Fix flexbox order in settings" --problem "Settings page"
assert_success "create solution for layout bug"
run_jjj solution submit "Fix flexbox"
assert_success "submit flexbox for review"
run_jjj solution approve "Fix flexbox" --no-rationale
assert_success "approve settings layout solution"

# Both v1.0 problems are now solved — check roadmap reflects this
run_jjj milestone roadmap
assert_success "roadmap after solving problems"
assert_contains "solved" "solved problems appear in roadmap"

# Mark the milestone complete
run_jjj milestone edit "v1.0 GA" --status completed
assert_success "mark v1.0 as completed"

run_jjj milestone show "v1.0 GA"
assert_success "show completed milestone"
assert_contains "completed" "milestone shows completed status"

observe "Milestone progression is visible in the roadmap as problems get solved"

# ============================================================================
section "Step 9: milestone show --json"
# ============================================================================

run_jjj milestone show "v1.0 GA" --json
assert_success "milestone show --json"
assert_contains "\"title\"" "JSON has title"
assert_contains "\"status\"" "JSON has status"
assert_contains "completed" "completed status in JSON"

run_jjj milestone list --json
assert_success "milestone list --json"
assert_contains "\"id\"" "JSON list has id"
assert_contains "v1.0" "v1.0 in JSON list"
assert_contains "v1.1" "v1.1 in JSON list"

observe "JSON output makes milestone data available to project dashboards and reports"

# ============================================================================
section "Step 10: milestone roadmap --json"
# ============================================================================

run_jjj milestone roadmap --json
assert_success "roadmap --json"
# roadmap --json returns a top-level array of milestone objects
assert_contains "\"id\"" "JSON roadmap has milestone id"
assert_contains "\"problem_ids\"" "JSON roadmap has problem_ids"

# ============================================================================
end_scenario
uxr_exit
