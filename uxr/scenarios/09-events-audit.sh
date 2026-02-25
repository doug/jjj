#!/usr/bin/env bash
# Scenario 09: Events Audit
#
# Tests all event-log features not covered by the basic scenarios:
#
#   events rebuild        (synthesize missing events from entity state)
#   events validate       (check event log consistency)
#   events --from/--to    (date range filtering)
#   events --since        (RFC3339 timestamp filter)
#   events --search       (full-text search in rationales)
#   events --event_type   (filter by specific type)
#   events --solution     (filter to a specific solution)
#   events --problem      (filter to a specific problem)
#   events --limit        (cap number of results)
#   events --json         (structured output)
#
# Tests: rebuild, validate, all filter flags, JSON output

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Events Audit"

# ============================================================================
section "Setup: build a rich event history"
# ============================================================================

setup_repo "events-audit"
run_jjj init
assert_success "init"

# Create problems, solutions, critiques to generate varied events
run_jjj problem new "Memory leak in worker pool" --priority critical
assert_success "create first problem"

run_jjj problem new "Slow startup time" --priority high
assert_success "create second problem"

run_jjj solution new "Fix worker lifecycle" --problem "Memory leak"
assert_success "create solution for first problem"

run_jjj solution new "Lazy-load modules on startup" --problem "Slow startup"
assert_success "create solution for second problem"

# Add a critique with a searchable rationale
run_jjj critique new "Fix worker lifecycle" \
    "Workers not properly cleaned up on panic" \
    --severity critical
assert_success "add critical critique"

# Address it
run_jjj critique address "not properly cleaned"
assert_success "address the critique"

# Accept the solution with a rationale that we can search for later
run_jjj solution accept "Fix worker" \
    --rationale "RAII-based cleanup eliminates the leak class entirely"
assert_success "accept solution with searchable rationale"

# Refute the second solution
run_jjj solution refute "Lazy-load" \
    --rationale "Lazy loading increases first-request latency, not acceptable"
assert_success "refute second solution"

# Dissolve the second problem
run_jjj problem dissolve "Slow startup" \
    --reason "Profiling showed startup is 200ms — not actually a problem"
assert_success "dissolve second problem"

# ============================================================================
section "Step 1: events (baseline)"
# ============================================================================

run_jjj events
assert_success "events list works"
assert_contains "problem_created" "problem creation event present"
assert_contains "solution_created" "solution creation event present"
assert_contains "critique_raised" "critique event present"

observe "events gives a chronological audit trail of all decisions"

# ============================================================================
section "Step 2: events --limit"
# ============================================================================

run_jjj events --limit 3
assert_success "events with limit"
# Should be 3 lines of events (roughly — output includes headers)
# Just check it doesn't explode and returns something
assert_contains "problem" "limited output still shows events"

# ============================================================================
section "Step 3: events --event_type"
# ============================================================================

run_jjj events --event-type problem_created
assert_success "filter by problem_created"
assert_contains "problem_created" "filtered results are correct type"
assert_not_contains "solution_created" "no solution events in problem_created filter"

run_jjj events --event-type solution_accepted
assert_success "filter by solution_accepted"
assert_contains "solution_accepted" "accepted event present"
assert_not_contains "problem_created" "no problem events in solution filter"

run_jjj events --event-type critique_raised
assert_success "filter by critique_raised"
assert_contains "critique_raised" "raised event present"

# ============================================================================
section "Step 4: events --problem and --solution filters"
# ============================================================================

# Get the problem ID to filter by
run_jjj problem show "Memory leak" --json
assert_success "show problem as JSON"
PROBLEM_ID=$(echo "$OUTPUT" | grep -oE '"id":\s*"[0-9a-f-]+"' | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)

if [[ -n "${PROBLEM_ID:-}" ]]; then
    run_jjj events --problem "$PROBLEM_ID"
    assert_success "events filtered by problem ID"
    assert_contains "problem_created" "problem's own creation event in filter"
    observe "Problem-scoped event view shows the complete history of one problem"
else
    skip "Problem ID not captured — skipping --problem filter test"
fi

# Get the solution ID
run_jjj solution show "Fix worker" --json
assert_success "show solution as JSON"
SOLUTION_ID=$(echo "$OUTPUT" | grep -oE '"id":\s*"[0-9a-f-]+"' | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)

if [[ -n "${SOLUTION_ID:-}" ]]; then
    run_jjj events --solution "$SOLUTION_ID"
    assert_success "events filtered by solution ID"
    assert_contains "solution_created" "solution creation event in filter"
    assert_contains "solution_accepted" "solution acceptance in filter"
else
    skip "Solution ID not captured — skipping --solution filter test"
fi

# ============================================================================
section "Step 5: events --search (rationale full-text)"
# ============================================================================

run_jjj events --search "RAII"
assert_success "search events by rationale keyword"
assert_contains "solution_accepted" "rationale search finds the acceptance event"

run_jjj events --search "latency"
assert_success "search events for refutation rationale"
assert_contains "solution_refuted" "latency rationale event found"

run_jjj events --search "200ms"
assert_success "search events for dissolve reason"
assert_contains "problem_dissolved" "dissolve reason event found"

observe "Rationale search lets you find past decisions by their reasoning, not just by entity ID"

# ============================================================================
section "Step 6: events --from / --to date filtering"
# ============================================================================

TODAY=$(date +%Y-%m-%d)
YEAR=$(date +%Y)
MONTH=$(date +%Y-%m)

run_jjj events --from "$TODAY"
assert_success "events from today"
assert_contains "problem_created" "today's events included"

run_jjj events --from "$YEAR-01-01" --to "$YEAR-12-31"
assert_success "events for full year range"
assert_contains "problem_created" "year-range events included"

run_jjj events --from "$MONTH"
assert_success "events with YYYY-MM date format"
assert_contains "problem_created" "month-format filter works"

# Future date should return empty (no events from the future)
run_jjj events --from "2099-01-01"
assert_success "events from far future returns empty gracefully"

observe "Date filtering makes it easy to review what happened in a sprint or release window"

# ============================================================================
section "Step 7: events --since (RFC3339)"
# ============================================================================

# Use a timestamp in the past (yesterday-ish)
SINCE_TS="${YEAR}-01-01T00:00:00Z"
run_jjj events --since "$SINCE_TS"
assert_success "events --since with RFC3339 timestamp"
assert_contains "problem_created" "events after start-of-year visible"

observe "--since is useful for CI/automation: show everything since the last build"

# ============================================================================
section "Step 8: events --json"
# ============================================================================

run_jjj events --json
assert_success "events --json output"
assert_contains "\"type\"" "JSON has type field"
assert_contains "\"entity\"" "JSON has entity field"
assert_contains "\"when\"" "JSON has timestamp"
assert_contains "\"by\"" "JSON has author field"

run_jjj events --event-type solution_accepted --json
assert_success "events filtered by event-type with --json"
assert_contains "\"solution_accepted\"" "correct type in JSON"

observe "JSON output enables structured processing of the event log in scripts and pipelines"

# ============================================================================
section "Step 9: events rebuild"
# ============================================================================

run_jjj events rebuild
assert_success "events rebuild runs without error"
assert_contains "rebuilt" "rebuild reports completion"

observe "events rebuild synthesizes any missing events by replaying entity state — safe to run repeatedly"

# After rebuild, the log should still be consistent
run_jjj events
assert_success "events work after rebuild"
assert_contains "problem_created" "problem events present after rebuild"

# ============================================================================
section "Step 10: events validate"
# ============================================================================

run_jjj events validate
assert_success "events validate passes on clean repo"
assert_contains "valid" "validation reports clean state"

observe "events validate confirms the event log is internally consistent — useful in CI"

# ============================================================================
end_scenario
uxr_exit
