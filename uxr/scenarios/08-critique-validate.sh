#!/usr/bin/env bash
# Scenario 08: Critique Depth
#
# Tests all critique lifecycle paths not covered by the basic scenarios:
#
#   critique validate     (Valid state — informational, does not hard-block accept)
#   critique dismiss      (shown to be incorrect)
#   critique reply        (comment threading)
#   critique edit         (change title/severity/status)
#   critique new --file --line  (source-location annotations)
#   critique new --reviewer     (assign reviewer)
#   critique list filters (--status, --solution, --reviewer)
#   critique show --json  (structured output)
#
# Note: --file/--line/--reviewer appear in creation output and --json, but
# not in the default text of critique show.
# Note: Only Open critiques hard-block solution accept; Valid critiques are
# informational (team is expected to refute manually).
#
# Tests: validate, dismiss, reply, edit, file/line annotations,
#        reviewer assignment, list filters, show --json

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Critique Depth"

# ============================================================================
section "Setup"
# ============================================================================

setup_repo "critique-depth"
run_jjj init
assert_success "init"

run_jjj problem new "API lacks input validation" --priority critical
assert_success "create problem"

run_jjj solution new "Add JSON schema validation" --problem "API lacks"
assert_success "create solution"

# ============================================================================
section "Step 1: critique new with --file and --line annotations"
# ============================================================================

# Creation output shows "Location: file:line"
run_jjj critique new "Add JSON schema" \
    "Schema validator does not reject unknown fields" \
    --severity high \
    --file "src/api/validate.rs" \
    --line 42
assert_success "create critique with source location"
assert_contains "Schema validator" "critique title in output"
assert_contains "validate.rs" "file annotation in creation output"
assert_contains "42" "line number in creation output"

# critique show text output is brief (id + title + status); use --json for full detail
run_jjj critique show "unknown fields" --json
assert_success "show critique as JSON"
assert_contains "\"file_path\"" "file path in JSON"
assert_contains "src/api/validate.rs" "correct file path in JSON"
assert_contains "\"line_start\"" "line number in JSON"

observe "File and line annotations let reviewers jump directly to the problem in code"
observe "Use critique show --json to see all fields including file/line/reviewer"

# ============================================================================
section "Step 2: critique new with --reviewer"
# ============================================================================

run_jjj critique new "Add JSON schema" \
    "No rate limiting on validation endpoint" \
    --severity medium \
    --reviewer "bob@example.com"
assert_success "create critique with reviewer"

# Reviewer appears in --json, not in default text output
run_jjj critique show "rate limiting" --json
assert_success "show critique with reviewer as JSON"
assert_contains "bob@example.com" "reviewer in JSON output"

observe "Assigning a reviewer makes ownership explicit from the start"

# ============================================================================
section "Step 3: critique list filters"
# ============================================================================

run_jjj critique list --solution "JSON schema"
assert_success "list critiques filtered by solution"
assert_contains "unknown fields" "first critique in filtered list"
assert_contains "rate limiting" "second critique in filtered list"

run_jjj critique list --status open
assert_success "list open critiques"
assert_contains "unknown fields" "open critique in list"

# Reviewer filter uses exact string match
run_jjj critique list --reviewer "bob@example.com"
assert_success "list critiques filtered by reviewer (exact email)"
assert_contains "rate limiting" "bob's critique in filtered list"

run_jjj critique list --json
assert_success "list critiques as JSON"
assert_contains "\"id\"" "JSON has id field"
assert_contains "\"title\"" "JSON has title field"
assert_contains "\"status\"" "JSON has status field"

observe "critique list --reviewer requires the exact assigned string (full email)"

# ============================================================================
section "Step 4: critique edit"
# ============================================================================

run_jjj critique edit "rate limiting" --severity high
assert_success "edit critique severity"

run_jjj critique show "rate limiting"
assert_success "show edited critique"
assert_contains "high" "updated severity visible"

run_jjj critique edit "rate limiting" --title "Validation endpoint has no rate limiting — DoS risk"
assert_success "edit critique title"

run_jjj critique show "DoS risk"
assert_success "show after title edit"
assert_contains "DoS risk" "new title takes effect"

observe "critique edit lets you refine severity and title as understanding improves"

# ============================================================================
section "Step 5: critique reply (comment threading)"
# ============================================================================

run_jjj critique reply "unknown fields" \
    "The strictMode option handles this — see schema.json line 8"
assert_success "reply to critique"
assert_contains "reply" "reply confirmed in output"

run_jjj critique reply "unknown fields" \
    "Confirmed — strictMode only applies to top-level keys, not nested objects"
assert_success "second reply to same critique"

run_jjj critique show "unknown fields"
assert_success "show critique after replies"

observe "Replies keep the discussion in context alongside the critique"

# ============================================================================
section "Step 6: critique dismiss"
# ============================================================================

run_jjj critique dismiss "unknown fields"
assert_success "dismiss critique"
assert_contains "dismissed" "critique is now dismissed"

run_jjj critique list --status dismissed
assert_success "list dismissed critiques"
assert_contains "unknown fields" "dismissed critique in filtered list"

# Dismissed critiques should not block acceptance
run_jjj critique list --status open
assert_success "list open critiques after dismiss"
assert_not_contains "unknown fields" "dismissed critique not in open list"

observe "Dismissed critiques are archived, not deleted — the reasoning remains visible"

# ============================================================================
section "Step 7: critique validate (informational) and the refute path"
# ============================================================================

# The DoS risk critique is still open — validate it
run_jjj critique validate "DoS risk"
assert_success "validate the rate limiting critique"
assert_contains "validated" "critique is now valid"

run_jjj critique list --status valid
assert_success "list valid critiques"
assert_contains "DoS risk" "validated critique in list"

observe "Validate means: this critique is confirmed correct — the solution has a flaw"

# Note: Only Open critiques hard-block acceptance. Validated critiques are
# informational — acceptance succeeds, but the team should refute manually.
run_jjj solution accept "JSON schema" --no-rationale
assert_success "accept succeeds even with validated critique (not a hard block)"
observe "Validated critiques do not hard-block acceptance — team decides whether to refute"
observe "Convention: if a critique is validated, refute the solution and propose a new one"

# Demonstrate the proper flow: create a new solution, refute it if it has a valid critique
run_jjj solution new "Rewrite validation layer with type-safe parser" \
    --problem "API lacks input validation"
assert_success "create replacement solution"

run_jjj critique new "Rewrite validation" \
    "Parser library adds 2MB to binary" \
    --severity low
assert_success "add low-severity critique to new solution"

run_jjj critique validate "adds 2MB"
assert_success "validate the size critique"

run_jjj solution refute "Rewrite validation" \
    --rationale "2MB binary increase violates our 1MB size budget for this service"
assert_success "refute solution because validated critique confirms it violates constraints"
assert_contains "refuted" "solution is now refuted"

observe "Validated critique → explicit refute → clear audit trail of why the approach failed"

# Final solution that actually works
run_jjj solution new "Inline schema validation with zero dependencies" \
    --problem "API lacks input validation"
assert_success "create final solution"

run_jjj critique new "Inline schema validation" \
    "Test coverage for edge cases needed" \
    --severity medium
assert_success "add test coverage critique"

run_jjj critique address "edge cases"
assert_success "address the coverage critique"

run_jjj solution accept "Inline schema" \
    --rationale "Zero-dependency validation eliminates size concern; test coverage added"
assert_success "accept final solution with all critiques resolved"
assert_contains "accepted" "solution accepted"

observe "Full validate→refute→new solution→accept cycle completes cleanly"

# ============================================================================
section "Step 8: critique show --json"
# ============================================================================

run_jjj critique show "DoS risk" --json
assert_success "show validated critique as JSON"
assert_contains "\"valid\"" "JSON shows valid status"
assert_contains "\"severity\"" "JSON has severity field"
assert_contains "\"reviewer\"" "JSON has reviewer field"

# ============================================================================
end_scenario
uxr_exit
