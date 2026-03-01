#!/usr/bin/env bash
# Scenario 06: End-to-End Showcase
#
# Simulates a realistic onboarding experience: a developer joins a project
# with existing code history, initializes jjj, and works through the full
# Problem → Solution → Critique → Accept lifecycle.
#
# This scenario was converted from demo/ and serves as both a regression
# test and a living example of the core jjj workflow.
#
# Tests: init, problem new, solution new (with change attachment), critique
#        new, critique address, solution approve, status at each stage,
#        push/fetch metadata transport, ui launch check

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "End-to-End Showcase"

# ============================================================================
section "Setup: realistic project repo with existing commits"
# ============================================================================

setup_repo "showcase"

# Create a realistic source tree across multiple commits
mkdir -p src

cat > src/auth.rs <<'EOF'
pub fn authenticate(user: &str, password: &str) -> bool {
    !user.is_empty() && !password.is_empty()
}
EOF

cat > src/api.rs <<'EOF'
pub fn handle_request(path: &str) -> String {
    match path {
        "/" => "Welcome!".to_string(),
        "/api/status" => "OK".to_string(),
        _ => "Not found".to_string(),
    }
}
EOF

cat > src/db.rs <<'EOF'
pub struct Database { connection: String }
impl Database {
    pub fn new(url: &str) -> Self { Self { connection: url.to_string() } }
    pub fn query(&self, sql: &str) -> Vec<String> {
        println!("Executing: {}", sql); vec![]
    }
}
EOF

jj describe -m "feat: initial project structure (auth, api, db)"
jj new -m "feat: add password hashing"
echo 'pub fn hash_password(p: &str) -> String { format!("hash_{}", p) }' >> src/auth.rs

jj new -m "feat: add user lookup endpoint"
echo 'pub fn get_user(id: u64) -> Option<String> { if id == 1 { Some("alice".into()) } else { None } }' >> src/api.rs

observe "Repo has 3 commits across auth, api, db — represents a typical project in flight"

# ============================================================================
section "Step 1: Initialize jjj"
# ============================================================================

run_jjj init
assert_success "jjj init in a repo with existing commits"
assert_contains "initialized" "init confirms success"

observe "jjj init works on a repo already in use — no conflicts with existing history"

# Double-init should be rejected cleanly
run_jjj init
assert_failure "double init is rejected"
assert_contains "already" "error says already initialized"

# ============================================================================
section "Step 2: Identify a problem"
# ============================================================================

run_jjj problem new "Auth has no rate limiting" --priority high
assert_success "create a high-priority security problem"
assert_contains "Auth has no rate limiting" "title echoed back"
RATE_LIMIT_PROBLEM_ID=$(echo "$OUTPUT" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)

run_jjj problem new "DB queries not sanitized" --priority critical
assert_success "create a critical problem"

run_jjj problem list
assert_success "problem list works"
assert_contains "Auth has no rate limiting" "first problem in list"
assert_contains "DB queries not sanitized" "second problem in list"

observe "Two problems created — the list gives a clear overview of open work"

# ============================================================================
section "Step 3: Propose a solution (auto-attached to current change)"
# ============================================================================

run_jjj solution new "Add token bucket rate limiter" --problem "rate limiting"
assert_success "solution new resolves problem by partial title"
assert_contains "Add token bucket rate limiter" "solution title echoed"

# Problem should auto-transition when a solution is proposed
run_jjj problem show "rate limiting"
assert_success "show problem by partial title"
assert_contains "in_progress" "problem moves to in_progress when solution proposed"

observe "Problem auto-transitions to in_progress — no manual state change needed"

run_jjj solution list
assert_success "solution list works"
assert_contains "token bucket" "solution visible in list"

# ============================================================================
section "Step 4: Check status (mid-workflow)"
# ============================================================================

run_jjj status
assert_success "status command works"
assert_contains "token bucket" "active solution shown in status"

observe "jjj status gives a clear 'what am I working on right now' view"

# ============================================================================
section "Step 5: A teammate adds a critique"
# ============================================================================

run_jjj critique new "token bucket" "Rate limit state is not shared across replicas" --severity high
assert_success "critique created against solution"
assert_contains "Rate limit state" "critique title echoed"

run_jjj critique list
assert_success "critique list works"
assert_contains "not shared across replicas" "critique in list"

# Status should now show blocked
run_jjj status
assert_success "status with open critique"
assert_contains "BLOCKED" "open critique blocks solution"

observe "BLOCKED state is immediately visible — no way to accidentally accept a critiqued solution"

# Submit for review (makes critique-blocking visible on approve attempts)
run_jjj solution submit "token bucket"
assert_success "submit solution for review"

# Trying to accept now should warn about open critiques
run_jjj solution approve "token bucket"
assert_failure "accept blocked by open critique"
assert_contains "critique" "error mentions the blocking critique"

observe "Acceptance gate enforced — critique must be resolved first"

# ============================================================================
section "Step 6: Address the critique"
# ============================================================================

run_jjj critique address "not shared"
assert_success "address critique by partial title"

# Status should no longer be blocked
run_jjj status
assert_success "status after addressing critique"
assert_not_contains "BLOCKED" "no longer blocked after critique addressed"

observe "Once addressed, the path to acceptance is clear"

# ============================================================================
section "Step 7: Accept the solution"
# ============================================================================

run_jjj solution approve "token bucket" --force
assert_success "solution approved with all critiques resolved"
assert_contains "approved" "solution is now approved"

# Only solution for that problem → problem auto-closes
run_jjj problem show "rate limiting"
assert_success "show problem after accept"
assert_contains "solved" "problem auto-solved when only solution is accepted"

observe "Problem lifecycle closes automatically — no manual bookkeeping"

# ============================================================================
section "Step 8: Metadata transport (push/fetch)"
# ============================================================================

observe "jjj push/fetch move the jjj bookmark just like code — no extra infrastructure"

# Verify push help is accessible (can't test actual remote in isolation)
run_jjj push --help
assert_success "push --help accessible"
assert_contains "remote" "push explains remote option"

run_jjj fetch --help
assert_success "fetch --help accessible"

run_jjj sync --help
assert_success "sync --help accessible"
assert_contains "fetch" "sync describes fetch+push behaviour"

# ============================================================================
section "Step 9: Search and timeline"
# ============================================================================

run_jjj search "rate limit"
assert_success "search finds entities by keyword"

run_jjj timeline "rate limit"
assert_success "timeline shows full history for a problem"
assert_contains "proposed" "solution event appears in timeline"
assert_contains "Rate limit state" "critique appears in timeline"
observe "Timeline gives a complete audit trail of every decision made on a problem"

# ============================================================================
section "Step 10: Help discoverability"
# ============================================================================

run_jjj --help
assert_success "top-level --help works"
assert_contains "problem" "help lists problem"
assert_contains "solution" "help lists solution"
assert_contains "critique" "help lists critique"
assert_contains "sync" "help lists sync"
assert_contains "github" "help lists github"

observe "All top-level commands visible in --help — good discoverability"

# ============================================================================
end_scenario
uxr_exit
