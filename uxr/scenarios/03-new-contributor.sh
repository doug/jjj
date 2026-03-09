#!/usr/bin/env bash
# Scenario 03: New Contributor Discovery
#
# Simulates Charlie who just joined a project already using jjj.
# Tests the discovery experience: can Charlie figure out what to
# work on, understand the project state, and contribute effectively?
#
# Tests: status, list, show, search, help, error messages,
#        discoverability of commands and concepts

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "New Contributor Discovery"

# ============================================================================
section "Setup: Create an Existing Project"
# ============================================================================

setup_repo "existing-project"
run_jjj init
assert_success "init"

# Create realistic project state
$JJJ milestone new "v2.0 Beta" --date 2025-09-01 2>/dev/null
$JJJ problem new "Authentication fails on expired tokens" --priority p0 2>/dev/null
$JJJ problem new "Search pagination broken" --priority p1 2>/dev/null
$JJJ problem new "Dashboard loads slowly" --priority p2 2>/dev/null
$JJJ problem new "Add dark mode support" --priority p3 2>/dev/null
$JJJ problem new "API rate limiting missing" --priority p1 2>/dev/null

# Add problems to milestone
$JJJ milestone add-problem "v2.0" "auth" 2>/dev/null
$JJJ milestone add-problem "v2.0" "pagination" 2>/dev/null
$JJJ milestone add-problem "v2.0" "rate limiting" 2>/dev/null

# Create solutions for some
$JJJ solution new "Use JWT refresh tokens" --problem "auth" 2>/dev/null
$JJJ solution new "Add cursor-based pagination" --problem "pagination" 2>/dev/null

# Create critiques on the JWT solution
$JJJ critique new "JWT refresh" "XSS vulnerability in token storage" --severity critical 2>/dev/null
$JJJ critique new "JWT refresh" "No token rotation implemented" --severity high 2>/dev/null

# Assign some work
$JJJ problem assign "auth" --to alice 2>/dev/null
$JJJ problem assign "Dashboard" --to bob 2>/dev/null

echo "  (project state created: 5 problems, 2 solutions, 2 critiques)"

# ============================================================================
section "Charlie's First Command: What's Going On?"
# ============================================================================

run_jjj status
assert_success "status as first command"
assert_contains "BLOCKED" "shows something is blocked"
observe "Status output clarity: $OUTPUT"

# ============================================================================
section "Exploring Problems"
# ============================================================================

run_jjj problem list
assert_success "list all problems"
assert_line_count_ge 5 "at least 5 problems listed"
observe "Does list show priority? $(echo "$OUTPUT" | head -3)"

# Can Charlie filter by status?
run_jjj problem list --status open
assert_success "filter by open status"

# Can Charlie see the tree?
run_jjj problem tree
assert_success "problem tree view"

# Can Charlie drill into a specific problem?
run_jjj problem show "auth"
assert_success "show problem by keyword"
assert_contains "Authentication" "found the auth problem"
assert_contains "JWT" "shows linked solution"

# ============================================================================
section "Understanding Solutions"
# ============================================================================

run_jjj solution list
assert_success "list all solutions"
assert_contains "JWT" "JWT solution listed"
assert_contains "pagination" "pagination solution listed"

run_jjj solution show "JWT"
assert_success "show JWT solution details"
assert_contains "XSS" "shows linked critiques"
observe "Solution detail: $OUTPUT"

# ============================================================================
section "Checking Critiques"
# ============================================================================

run_jjj critique list
assert_success "list all critiques"
assert_contains "XSS" "XSS critique listed"

run_jjj critique show "XSS"
assert_success "show critique details"

# ============================================================================
section "Milestone and Roadmap"
# ============================================================================

run_jjj milestone list
assert_success "list milestones"
assert_contains "v2.0" "milestone listed"

run_jjj milestone roadmap
assert_success "roadmap view"

run_jjj milestone show "v2.0"
assert_success "milestone detail"
# Note: milestone show currently doesn't list problem names inline
observe "Milestone detail output: $(echo "$OUTPUT" | head -8)"

# ============================================================================
section "Help Discoverability"
# ============================================================================

# Top-level help
run_jjj --help
assert_success "top-level help"
assert_contains "problem" "mentions problems"
assert_contains "solution" "mentions solutions"
assert_contains "critique" "mentions critiques"
assert_contains "status" "mentions status"

# Subcommand help
run_jjj problem --help
assert_success "problem help"
assert_contains "new" "shows new command"
assert_contains "list" "shows list command"
assert_contains "show" "shows show command"

run_jjj solution --help
assert_success "solution help"

run_jjj critique --help
assert_success "critique help"

# ============================================================================
section "Common Mistakes and Error Quality"
# ============================================================================

# Typo in subcommand
run_jjj problm list
assert_failure "typo in subcommand"
observe "Typo error quality: $OUTPUT"

# Missing required argument
run_jjj problem new
assert_failure "missing title argument"
observe "Missing arg error: $OUTPUT"

# Wrong entity type keyword (github user says 'issue')
run_jjj issue list
assert_failure "wrong keyword 'issue'"
observe "Issue alias error: $OUTPUT"

# Try to show nonexistent entity
run_jjj problem show "zzz-does-not-exist"
assert_failure "nonexistent entity"
observe "Not found error: $OUTPUT"

# ============================================================================
section "Charlie Contributes: Propose a Solution"
# ============================================================================

run_jjj solution new "Add Redis-based rate limiter" --problem "rate limiting"
assert_success "charlie proposes a solution"

# Check it appears
run_jjj solution list --problem "rate"
assert_success "list solutions for rate limiting"
assert_contains "Redis" "charlie's solution listed"

# Charlie adds a self-critique
run_jjj critique new "Redis" "Redis adds operational complexity" --severity low
assert_success "charlie self-critiques"

# ============================================================================
section "Events and Timeline"
# ============================================================================

run_jjj events
assert_success "events log"
assert_line_count_ge 5 "several events logged"

# ============================================================================
section "Sorting and Filtering"
# ============================================================================

run_jjj problem list --sort title
assert_success "sort problems by title"

run_jjj problem list --sort priority
assert_success "sort problems by priority"

run_jjj problem list --sort created
assert_success "sort problems by creation time"

run_jjj solution list --sort status
assert_success "sort solutions by status"

# ============================================================================
end_scenario
uxr_exit
