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

# Submit then approve with a rationale that we can search for later
run_jjj solution submit "Fix worker"
assert_success "submit solution for review"
run_jjj solution approve "Fix worker" \
    --rationale "RAII-based cleanup eliminates the leak class entirely"
assert_success "approve solution with searchable rationale"

# Withdraw the second solution
run_jjj solution withdraw "Lazy-load" \
    --rationale "Lazy loading increases first-request latency, not acceptable"
assert_success "withdraw second solution"

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

run_jjj events --event-type solution_approved
assert_success "filter by solution_approved"
assert_contains "solution_approved" "approved event present"
assert_not_contains "problem_created" "no problem events in solution filter"

run_jjj events --event-type critique_raised
assert_success "filter by critique_raised"
assert_contains "critique_raised" "raised event present"

# ============================================================================
section "Step 4: events --problem and --solution filters"
# ============================================================================

run_jjj events --problem "Memory leak"
assert_success "events filtered by problem title"
assert_contains "problem_created" "problem's own creation event in filter"
observe "Problem-scoped event view shows the complete history of one problem"

run_jjj events --solution "Fix worker"
assert_success "events filtered by solution title"
assert_contains "solution_created" "solution creation event in filter"
assert_contains "solution_approved" "solution approval in filter"

# ============================================================================
section "Step 5: events --search (rationale full-text)"
# ============================================================================

run_jjj events --search "RAII"
assert_success "search events by rationale keyword"
assert_contains "solution_approved" "rationale search finds the approval event"

run_jjj events --search "latency"
assert_success "search events for refutation rationale"
assert_contains "solution_withdrawn" "latency rationale event found"

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

run_jjj events --event-type solution_approved --json
assert_success "events filtered by event-type with --json"
assert_contains "\"solution_approved\"" "correct type in JSON"

observe "JSON output enables structured processing of the event log in scripts and pipelines"

# ============================================================================
section "Step 9: events rebuild"
# ============================================================================

run_jjj events rebuild
assert_success "events rebuild runs without error"
assert_contains "rebuilt" "rebuild reports completion"

observe "events rebuild replays commit history — lossless, author/timestamp/rationale preserved exactly"

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
section "Step 11: no events.jsonl — events live in commit history"
# ============================================================================

# events.jsonl must not exist. Events are embedded as `jjj: <json>` lines in
# commit descriptions, so the history IS the event log. This means bookmark
# merges never produce conflict markers in an events file.
EVENTS_FILE=".jj/jjj-meta/events.jsonl"
if [[ -f "$EVENTS_FILE" ]]; then
    echo "    FAIL events.jsonl must not exist (found at $EVENTS_FILE)"
    _FAIL=$((_FAIL + 1))
else
    echo -e "    ${GREEN}PASS${RESET} no events.jsonl file — events embedded in commit descriptions"
    _PASS=$((_PASS + 1))
fi

# Events are still fully readable despite having no file
run_jjj events
assert_success "events readable with no events.jsonl"
assert_contains "problem_created" "problem events present from commit history"

observe "No events.jsonl means no merge conflicts. Two contributors can push independently; after fetch, ::@ traversal includes both sides of the merge and all events appear automatically."

# ============================================================================
section "Step 12: approve emits two events in one commit"
# ============================================================================

# Approving a solution that fully resolves its parent problem emits both
# solution_approved and problem_solved in the SAME commit (two jjj: lines).
run_jjj problem new "Two-Event Problem" --priority high
assert_success "create problem for two-event test"

run_jjj solution new "Two-Event Solution" --problem "Two-Event Problem"
assert_success "create solution"

run_jjj solution submit "Two-Event Solution"
assert_success "submit solution"

run_jjj solution approve "Two-Event Solution" --force
assert_success "approve solution (--force skips review requirement)"

run_jjj events --event-type solution_approved --json
assert_success "solution_approved event recorded"
assert_contains "solution_approved" "solution_approved in event log"

run_jjj events --event-type problem_solved --json
assert_success "problem_solved event recorded"
assert_contains "problem_solved" "problem_solved auto-emitted in same commit"

observe "Approving a solution records both solution_approved and problem_solved in one atomic commit — the timeline shows the full causal chain with no gaps."

# ============================================================================
end_scenario
uxr_exit
