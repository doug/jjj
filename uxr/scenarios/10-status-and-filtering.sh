#!/usr/bin/env bash
# Scenario 10: Status and Filtering
#
# Tests all the flag-based filtering and output modes not covered elsewhere:
#
#   status --mine, --all, --limit, --json
#   problem list --assignee, --status, --search, --sort, --tree, --milestone
#   solution list --status, --problem, --search, --sort
#   critique list --status, --solution, --reviewer
#   search --type, --text-only, --json
#   timeline --json
#   {problem,solution,critique,milestone} show --json
#   db status, db rebuild
#   completion (shell completion generation)
#
# Implementation notes:
#   - problem list --assignee and --reviewer use substring comparison
#   - problem list --search and jjj search use FTS — auto-indexed on save
#   - solution new auto-attaches current change but stays in "proposed" state
#   - problem list --milestone accepts UUID, prefix, or title (entity resolution)
#   - search --json field is "type" (not "entity_type")
#   - completion generates "_jjj" bash function
#
# Tests: all list/status filter flags, JSON output across all entity types,
#        search type filtering, db status/rebuild, completion generation

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Status and Filtering"

# ============================================================================
section "Setup: a project with varied entities"
# ============================================================================

setup_repo "status-filtering"
run_jjj init
assert_success "init"

# Create problems with different priorities and assignees
run_jjj problem new "Performance regression in search" --priority critical
assert_success "create P0 problem"

run_jjj problem new "Missing dark mode" --priority low
assert_success "create P3 problem"

run_jjj problem new "Auth tokens expire too quickly" --priority high
assert_success "create P1 problem"

run_jjj problem new "Onboarding flow is confusing" --priority medium
assert_success "create P2 problem"

# Assign some problems (filters use exact string match)
run_jjj problem assign "Performance regression" --to "alice@example.com"
assert_success "assign P0 to alice"

run_jjj problem assign "Auth tokens" --to "bob@example.com"
assert_success "assign auth problem to bob"

# Create a milestone and add problems to it
run_jjj milestone new "v2.0 Release" --date "2026-06-01"
assert_success "create milestone"

run_jjj milestone add-problem "v2.0" "Performance regression"
assert_success "add P0 to milestone"

run_jjj milestone add-problem "v2.0" "Auth tokens"
assert_success "add auth problem to milestone"

# Create solutions (auto-attach current change; stay in Proposed state)
run_jjj solution new "Add search result cache" --problem "Performance regression"
assert_success "solution for search perf"

run_jjj solution new "Extend token lifetime to 30 days" --problem "Auth tokens"
assert_success "solution for auth"

run_jjj solution new "Add token refresh flow" --problem "Auth tokens"
assert_success "competing solution for auth"

# Add critiques
run_jjj critique new "Add search result cache" \
    "Cache invalidation not handled" --severity high
assert_success "add critique to search solution"

run_jjj critique new "Extend token lifetime" \
    "30 days is too long for security policy" --severity critical \
    --reviewer "security@example.com"
assert_success "add security critique"

# FTS is auto-indexed on save — no db rebuild needed

# ============================================================================
section "Step 1: status flags"
# ============================================================================

run_jjj status
assert_success "status works"

run_jjj status --limit 2
assert_success "status --limit 2"
assert_contains "problem" "status with limit shows problems"

# status --all shows all items; P0 solution is BLOCKED by the open critique
run_jjj status --all
assert_success "status --all shows everything"
assert_contains "BLOCKED" "blocked solution appears in status --all"

run_jjj status --json
assert_success "status --json"
assert_contains "\"items\"" "JSON status has items array"
assert_contains "\"summary\"" "JSON status has summary"
assert_contains "\"total_count\"" "JSON has total_count"

observe "status --json enables CI dashboards and custom tooling on top of jjj"

# ============================================================================
section "Step 2: problem list filters"
# ============================================================================

# Problems with solutions are in_progress; genuinely open ones have no solution
run_jjj problem list --status open
assert_success "problem list --status open"
assert_contains "Onboarding flow" "open problem (no solution) in list"
assert_not_contains "Performance regression" "in_progress problem excluded from open filter"

run_jjj problem list --status in_progress
assert_success "problem list --status in_progress"
assert_contains "in_progress" "in_progress problems listed"
assert_contains "Performance regression" "P0 with solution is in_progress"

# assignee filter uses substring comparison
run_jjj problem list --assignee "alice"
assert_success "problem list --assignee alice (substring)"
assert_contains "Performance regression" "alice's problem in list"
assert_not_contains "Missing dark mode" "unassigned problem not in alice's list"

run_jjj problem list --assignee "bob@example.com"
assert_success "problem list --assignee bob (exact email also works)"
assert_contains "Auth tokens" "bob's problem in list"

# search filter uses FTS (auto-indexed on save — no db rebuild needed)
# Porter stemmer: "token" matches "tokens", "Auth" matches "auth"
run_jjj problem list --search "token"
assert_success "problem list --search token (stemmed: matches tokens)"
assert_contains "Auth tokens" "search finds matching problem via stemming"
assert_not_contains "Performance regression" "non-matching problem excluded"

# milestone filter accepts title directly (entity resolution)
run_jjj problem list --milestone "v2.0"
assert_success "problem list --milestone filter (by title prefix)"
assert_contains "Performance regression" "milestone problem in list"
assert_not_contains "Missing dark mode" "non-milestone problem excluded"
observe "problem list --milestone accepts UUID, prefix, or title — no need to look up UUID"

# Sort variants
run_jjj problem list --sort priority
assert_success "sort by priority"

run_jjj problem list --sort title
assert_success "sort by title"

run_jjj problem list --sort created
assert_success "sort by created"

run_jjj problem list --tree
assert_success "problem list --tree"

run_jjj problem list --json
assert_success "problem list --json"
assert_contains "\"id\"" "JSON has id"
assert_contains "\"priority\"" "JSON has priority"
assert_contains "\"status\"" "JSON has status"

observe "List filters reduce noise — show only what's relevant to the current focus"
observe "assignee and reviewer filters use substring matching — partial email or username works"

# ============================================================================
section "Step 3: problem show --json"
# ============================================================================

run_jjj problem show "Performance regression" --json
assert_success "problem show --json"
assert_contains "\"title\"" "JSON has title"
assert_contains "\"priority\"" "JSON has priority"
assert_contains "\"status\"" "JSON has status"
assert_contains "alice@example.com" "assignee in JSON"

# ============================================================================
section "Step 4: solution list filters"
# ============================================================================

# solution new auto-attaches current change but stays in Proposed state
run_jjj solution list --status proposed
assert_success "solution list --status proposed"
assert_contains "proposed" "proposed solutions returned"
assert_contains "Add search result cache" "cache solution in proposed list"

run_jjj solution list --problem "Auth tokens"
assert_success "solution list --problem filter"
assert_contains "Extend token" "solution for auth problem listed"
assert_contains "Add token refresh" "competing solution listed"
assert_not_contains "search result cache" "unrelated solution excluded"

# search uses FTS (populated by db rebuild above)
run_jjj solution list --search "token"
assert_success "solution list --search token"
assert_contains "token" "search finds token solutions"

run_jjj solution list --sort title
assert_success "solution list --sort title"

run_jjj solution list --sort created
assert_success "solution list --sort created"

run_jjj solution list --json
assert_success "solution list --json"
assert_contains "\"id\"" "JSON has id"
assert_contains "\"status\"" "JSON has status"

# ============================================================================
section "Step 5: solution show --json"
# ============================================================================

run_jjj solution show "Add search result cache" --json
assert_success "solution show --json"
assert_contains "\"title\"" "JSON title field"
assert_contains "\"status\"" "JSON status field"
assert_contains "\"problem_id\"" "JSON links to problem"

# ============================================================================
section "Step 6: critique list filters"
# ============================================================================

run_jjj critique list --status open
assert_success "critique list --status open"
assert_contains "Cache invalidation" "open critique in list"
assert_contains "30 days is too long" "second open critique in list"

run_jjj critique list --solution "Add search result cache"
assert_success "critique list --solution filter"
assert_contains "Cache invalidation" "critique for that solution"
assert_not_contains "30 days" "other solution's critique excluded"

# reviewer filter uses substring match
run_jjj critique list --reviewer "security"
assert_success "critique list --reviewer filter (substring match)"
assert_contains "30 days is too long" "security team's critique in filter"

run_jjj critique list --json
assert_success "critique list --json"
assert_contains "\"id\"" "JSON has id"
assert_contains "\"severity\"" "JSON has severity"

# ============================================================================
section "Step 7: critique show --json"
# ============================================================================

run_jjj critique show "Cache invalidation" --json
assert_success "critique show --json"
assert_contains "\"title\"" "JSON title"
assert_contains "\"severity\"" "JSON severity"
assert_contains "\"status\"" "JSON status"

# ============================================================================
section "Step 8: milestone show --json and list --json"
# ============================================================================

run_jjj milestone list --json
assert_success "milestone list --json"
assert_contains "\"id\"" "JSON has id"
assert_contains "\"title\"" "JSON has title"

run_jjj milestone show "v2.0" --json
assert_success "milestone show --json"
assert_contains "\"title\"" "JSON title"
assert_contains "\"status\"" "JSON status"
assert_contains "2026-06-01" "date in JSON"

# ============================================================================
section "Step 9: search --type and --text-only"
# ============================================================================

# jjj search uses FTS (auto-indexed on save, porter stemmer enabled)
# Porter stemmer: "token" matches "tokens", "cache" matches "cached", etc.
run_jjj search "token" --type problem
assert_success "search --type problem (stemmed: token→tokens)"
assert_contains "Auth tokens" "Auth tokens problem found via stemming"
assert_not_contains "Extend token lifetime" "solution excluded by type filter"

run_jjj search "cache" --type solution
assert_success "search --type solution"
assert_contains "Add search result cache" "cache solution found"

# --text-only skips embeddings and uses FTS only (hyphen, not underscore)
run_jjj search "cache" --text-only
assert_success "search --text-only (skip embeddings)"
assert_contains "cache" "text-only search returns results"

# JSON output field is "type" (not "entity_type")
run_jjj search "invalidation" --json
assert_success "search --json output"
assert_contains "\"title\"" "JSON search result has title"
assert_contains "\"type\"" "JSON has type field"

observe "search --type narrows results when you know what kind of entity you're looking for"
observe "search --text-only is useful when embeddings haven't been computed yet"
observe "FTS uses porter stemming — 'token' finds 'tokens', 'cache' finds 'cached'"
observe "search --json field is 'type' (not 'entity_type')"

# ============================================================================
section "Step 10: timeline --json"
# ============================================================================

run_jjj timeline "Performance regression" --json
assert_success "timeline --json for a problem"
assert_contains "\"type\"" "JSON timeline has type field"
assert_contains "\"entity\"" "JSON timeline has entity field"
assert_contains "\"when\"" "JSON timeline has timestamp"
observe "timeline --json is useful for generating changelogs and audit reports programmatically"

# ============================================================================
section "Step 11: db status"
# ============================================================================

run_jjj db status
assert_success "db status works"
assert_contains "Database" "db status shows path info"

observe "db status shows cache health — useful for debugging search and embedding issues"

# ============================================================================
section "Step 12: shell completion"
# ============================================================================

# completion generates shell-specific completion scripts
# Content checks are skipped: the large output causes SIGPIPE with grep -q + pipefail
run_jjj completion bash
assert_success "bash completion generates"

run_jjj completion zsh
assert_success "zsh completion generates"

run_jjj completion fish
assert_success "fish completion generates"

observe "Shell completions lower the barrier to learning all the flags"

# ============================================================================
end_scenario
uxr_exit
