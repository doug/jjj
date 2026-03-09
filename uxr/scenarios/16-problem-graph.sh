#!/usr/bin/env bash
# Scenario 16: Problem Graph
#
# Tests the `problem graph` command:
#
#   problem graph              (render all active problems as ASCII DAG)
#   problem graph --all        (include solved/dissolved problems)
#   problem graph --milestone  (filter to a milestone)
#
# Tests: 3-level hierarchy, two separate trees, --all flag, --milestone filter

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Problem Graph"

# ============================================================================
section "Setup"
# ============================================================================

setup_repo "problem-graph"
run_jjj init
assert_success "init"

# ============================================================================
section "Step 1: Create 3-level hierarchy"
# ============================================================================

run_jjj problem new "Authentication system" --priority p1 --force
assert_success "create root problem"
assert_contains "Authentication system" "root problem title in output"

run_jjj problem new "Login flow" --parent "Authentication system" --priority p2 --force
assert_success "create child problem"
assert_contains "Login flow" "child problem title in output"

run_jjj problem new "OAuth2 integration" --parent "Login flow" --priority p2 --force
assert_success "create grandchild problem"
assert_contains "OAuth2 integration" "grandchild problem title in output"

observe "3-level hierarchy created: root → child → grandchild"

# ============================================================================
section "Step 2: problem graph shows all three with tree characters"
# ============================================================================

run_jjj problem graph
assert_success "problem graph runs successfully"
assert_contains "Authentication system" "root problem in graph"
assert_contains "Login flow" "child problem in graph"
assert_contains "OAuth2 integration" "grandchild problem in graph"
assert_contains "○" "open problem icon present"

observe "Graph renders all three levels with tree characters"

# ============================================================================
section "Step 3: Add a second root; verify two separate trees shown"
# ============================================================================

run_jjj problem new "Performance monitoring" --priority p3 --force
assert_success "create second root problem"
assert_contains "Performance monitoring" "second root problem title in output"

run_jjj problem new "Request latency tracking" --parent "Performance monitoring" --priority p3 --force
assert_success "create first child of second root"

# Add a second child to Authentication system so ├─ appears (first child = ├─, last = └─)
run_jjj problem new "Session management" --parent "Authentication system" --priority p2 --force
assert_success "create second child of first root"

run_jjj problem graph
assert_success "problem graph with two roots"
assert_contains "Authentication system" "first root in graph"
assert_contains "Performance monitoring" "second root in graph"
assert_contains "Login flow" "child under first root"
assert_contains "Session management" "second child under first root"
assert_contains "Request latency tracking" "child under second root"
assert_contains "├─" "branch character present for non-last child"
assert_contains "└─" "end-of-subtree character present"

observe "Two independent trees shown; ├─ for non-last children, └─ for last"

# ============================================================================
section "Step 4: problem graph --all shows solved problems"
# ============================================================================

# Solve "OAuth2 integration" so it disappears from default graph
run_jjj solution new "Implement OAuth2 flow" --problem "OAuth2 integration" --force
assert_success "create solution for OAuth2"

run_jjj solution submit "Implement OAuth2 flow"
assert_success "submit solution"

run_jjj solution approve "Implement OAuth2 flow" --no-rationale
assert_success "approve solution"

run_jjj problem solve "OAuth2 integration"
assert_success "solve OAuth2 problem"

# Default graph should NOT show solved problem
run_jjj problem graph
assert_success "default graph after solving"
assert_not_contains "OAuth2 integration" "solved problem hidden in default graph"

# --all should show it
run_jjj problem graph --all
assert_success "graph --all shows solved problem"
assert_contains "OAuth2 integration" "solved problem visible with --all"
assert_contains "◉" "solved problem icon present"

observe "--all includes solved/dissolved problems with ◉ icon"

# ============================================================================
section "Step 5: problem graph --milestone filters to milestone problems"
# ============================================================================

run_jjj milestone new "Q1 Release" --date 2026-03-31
assert_success "create milestone"
assert_contains "Q1 Release" "milestone created"

run_jjj milestone add-problem "Q1 Release" "Authentication system"
assert_success "add root problem to milestone"

run_jjj milestone add-problem "Q1 Release" "Login flow"
assert_success "add child problem to milestone"

run_jjj problem graph --milestone "Q1 Release"
assert_success "graph with milestone filter"
assert_contains "Login flow" "milestone problem in graph"
assert_not_contains "Performance monitoring" "non-milestone problem excluded"
assert_not_contains "Request latency tracking" "non-milestone child excluded"

observe "--milestone filter restricts graph to problems in that milestone"

# ============================================================================
end_scenario
uxr_exit
