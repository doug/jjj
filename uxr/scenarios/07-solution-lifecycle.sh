#!/usr/bin/env bash
# Scenario 07: Solution Lifecycle
#
# Tests the full solution state machine and all solution-specific commands
# that aren't covered by the basic P→S→CQ scenarios:
#
#   solution submit      (solution new stays Proposed; solution submit advances to Submitted)
#   solution attach      (link current jj change)
#   solution detach      (unlink a change; requires --force from Submitted state)
#   solution withdraw    (with --rationale)
#   solution approve     (with --rationale)
#   solution assign      (assign to named person)
#   solution --supersedes (track iteration)
#   solution list        (--status, --problem filters)
#   solution show        (--json output)
#
# Note: solution new auto-attaches the current jj change but stays in Proposed
# state. Call solution submit explicitly to advance to Submitted.
# Detaching from Submitted requires --force.
#
# Tests: solution submit, attach, detach, withdraw/approve with rationale,
#        supersedes chain, assign, list filters, show --json

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Solution Lifecycle"

# ============================================================================
section "Setup"
# ============================================================================

setup_repo "solution-lifecycle"
run_jjj init
assert_success "init"

run_jjj problem new "Login takes too long" --priority high
assert_success "create problem"

# ============================================================================
section "Step 1: solution new stays Proposed; solution submit advances to Submitted"
# ============================================================================

run_jjj solution new "Cache session tokens" --problem "Login takes too long"
assert_success "create solution"
assert_contains "Cache session tokens" "solution title in output"

# solution new auto-attaches the current jj change but stays in Proposed state
run_jjj solution list
assert_success "solution list"
assert_contains "proposed" "solution stays proposed after creation"
assert_contains "Cache session" "solution title in list"

# solution submit explicitly advances to Testing
run_jjj solution submit "Cache session"
assert_success "solution submit advances to submitted"
assert_contains "submitted" "solution is now submitted after explicit submit call"

observe "solution new auto-attaches current jj change but stays Proposed"
observe "call solution submit explicitly when ready to submit for review"

# ============================================================================
section "Step 2: solution list --status filter"
# ============================================================================

run_jjj solution list --status submitted
assert_success "list filtered to submitted"
assert_contains "Cache session" "submitted solution in filtered list"

run_jjj solution list --status proposed
assert_success "list filtered to proposed (empty — cache solution is now submitted)"

run_jjj solution list --problem "Login"
assert_success "list filtered by problem"
assert_contains "Cache session" "solution linked to problem appears"

# ============================================================================
section "Step 3: solution attach and detach"
# ============================================================================

# Create a new jj change to work on a second solution
jj new -m "feat: try a different approach"

run_jjj solution new "Use JWT with short expiry" --problem "Login takes too long"
assert_success "create second solution"
assert_contains "Use JWT" "JWT solution created"

# JWT solution is Proposed (auto-attached, but not yet testing).
# Advance to Testing, then verify attach is idempotent.
run_jjj solution submit "JWT"
assert_success "advance JWT to review"

run_jjj solution attach "JWT"
assert_success "attach current change to solution (idempotent)"
assert_contains "Attached" "attach confirms the link"

run_jjj solution show "JWT"
assert_success "show solution after attach"
assert_contains "JWT" "solution details visible"

# Detaching from a Testing solution requires --force
run_jjj solution detach "JWT" --force
assert_success "detach current change from solution (--force required from Submitted state)"
assert_contains "Detached" "detach confirms removal"

observe "Detach from Submitted requires --force — prevents accidental loss of work in progress"

# ============================================================================
section "Step 4: Validate critique then withdraw with rationale"
# ============================================================================

run_jjj critique new "JWT" "JWT expiry too short for mobile clients" --severity high
assert_success "add critique to JWT solution"

run_jjj critique validate "JWT expiry"
assert_success "validate the critique (confirms it is correct)"
assert_contains "validated" "critique is now valid"

observe "Validated critiques mean the solution has a confirmed flaw"

# Validated critiques hard-block approval (same as Open critiques).
# Must address or dismiss the blocking critique before approving.
run_jjj solution approve "JWT" --no-rationale
assert_failure "approve is blocked by validated critique"
observe "Validated critiques hard-block approval — resolve them before approving"

# Dismiss the validated critique to unblock approval
run_jjj critique dismiss "JWT expiry"
assert_success "dismiss the validated critique"

run_jjj solution approve "JWT" --no-rationale
assert_success "approve succeeds once critique is dismissed"
assert_contains "approved" "solution approved after resolving blocking critique"

# Reset: re-open by... actually we just approved it, so let's demonstrate withdraw
# on a freshly created solution instead
jj new -m "feat: jwt retry approach"
run_jjj solution new "JWT with sliding expiry" --problem "Login takes too long"
assert_success "create fresh JWT variant"

run_jjj critique new "JWT with sliding" "Sliding expiry still leaks session state" --severity high
assert_success "add critique to JWT variant"

run_jjj solution withdraw "JWT with sliding" \
    --rationale "JWT statelessness is fundamentally incompatible with immediate revocation requirements"
assert_success "withdraw solution with explicit rationale"
assert_contains "withdrawn" "solution is now withdrawn"

run_jjj solution show "JWT with sliding"
assert_success "show withdrawn solution"
assert_contains "withdrawn" "withdrawn state visible in details"

observe "Withdrawing with a rationale creates a clear audit trail of why the approach failed"

# ============================================================================
section "Step 5: Superseding solution (iteration)"
# ============================================================================

run_jjj solution show "JWT with sliding" --json
assert_success "show withdrawn solution as JSON"
assert_contains "\"withdrawn\"" "JSON shows withdrawn status"

run_jjj solution new "Sliding window sessions with refresh tokens" \
    --problem "Login takes too long" \
    --supersedes "JWT with sliding"
assert_success "create superseding solution"
assert_contains "Sliding window" "superseding solution created"
observe "supersedes links the new solution to the one it replaces — maintains decision history"

# ============================================================================
section "Step 6: Approve with rationale"
# ============================================================================

run_jjj solution approve "Cache session" \
    --rationale "Session token cache gives 10x speedup with acceptable security tradeoffs"
assert_success "approve solution with explicit rationale"
assert_contains "approved" "solution approved"

run_jjj solution show "Cache session" --json
assert_success "show approved solution as JSON"
assert_contains "\"approved\"" "JSON shows approved status"

observe "Rationale on approve records the 'why' alongside the decision"

# ============================================================================
section "Step 7: solution assign"
# ============================================================================

run_jjj problem new "DB connection pool exhaustion" --priority critical
assert_success "create second problem"

run_jjj solution new "Increase pool size" --problem "DB connection"
assert_success "create solution to assign"

run_jjj solution assign "Increase pool" --to "alice@example.com"
assert_success "assign solution to alice"
assert_contains "alice" "assignee shown in output"

run_jjj solution show "Increase pool"
assert_success "show assigned solution"
assert_contains "alice" "assignee visible in solution details"

# ============================================================================
section "Step 8: JSON output"
# ============================================================================

run_jjj solution list --json
assert_success "solution list --json"
assert_contains "\"id\"" "JSON output has id field"
assert_contains "\"title\"" "JSON output has title field"
assert_contains "\"status\"" "JSON output has status field"

# ============================================================================
end_scenario
uxr_exit
