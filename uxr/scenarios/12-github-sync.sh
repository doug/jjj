#!/usr/bin/env bash
# Scenario 12: GitHub sync — issue import, status, solve with auto-close
#
# Uses an embedded fake `gh` CLI stub so no real GitHub credentials are needed.
# The stub handles auth, repo detection, issue CRUD, and PR review fetching.

# Always use the debug binary so new commands are available without a release build.
SCENARIO_DIR="$(cd "$(dirname "$0")" && pwd)"
export JJJ_BIN="$SCENARIO_DIR/../../target/debug/jjj"

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "12 — GitHub sync (fake gh stub)"

# ── Fake gh stub ──────────────────────────────────────────────────────────────
FAKE_BIN="$UXR_TMPDIR/fake-bin"
mkdir -p "$FAKE_BIN"

cat > "$FAKE_BIN/gh" << 'GHEOF'
#!/usr/bin/env bash
# Minimal fake gh CLI for UXR testing.
# Match on first two positional args, then check the rest.

CMD="$1 $2"

case "$CMD" in
  "auth status")
    echo "Logged in to github.com account testuser (keyring)"
    exit 0
    ;;

  "api user")
    # gh api user --jq .login
    echo "testuser"
    exit 0
    ;;

  "repo view")
    # gh repo view --json owner,name --jq ...
    echo "testowner/testrepo"
    exit 0
    ;;

  "issue create")
    echo "42"
    exit 0
    ;;

  "issue view")
    # gh issue view N --json ...
    NUM="$3"
    ARGS="$*"
    if [[ "$ARGS" == *"state"* && "$ARGS" == *".state"* ]]; then
      echo "OPEN"
    elif [[ "$NUM" == "42" ]]; then
      cat <<'JSON'
{"number":42,"title":"Login is slow when session expires","body":"Users are logged out after 30 minutes of inactivity.","state":"OPEN","labels":[{"name":"high"}],"author":{"login":"octocat"}}
JSON
    else
      cat <<'JSON'
{"number":43,"title":"Memory leak in worker thread","body":"Worker process grows to 2 GB then crashes.","state":"OPEN","labels":[{"name":"critical"}],"author":{"login":"bob"}}
JSON
    fi
    exit 0
    ;;

  "issue list")
    cat <<'JSON'
[{"number":42,"title":"Login is slow when session expires","state":"OPEN","labels":[{"name":"high"}]},{"number":43,"title":"Memory leak in worker thread","state":"OPEN","labels":[{"name":"critical"}]}]
JSON
    exit 0
    ;;

  "issue close")
    echo "Closed issue #$3."
    exit 0
    ;;

  "issue reopen")
    echo "Reopened issue #$3."
    exit 0
    ;;

  "pr view")
    # Determine what field is requested
    ARGS="$*"
    if [[ "$ARGS" == *"reviewThreads"* ]]; then
      cat <<'JSON'
[{"isResolved":false,"isOutdated":false,"comments":[{"databaseId":99001,"author":{"login":"alice"},"body":"Missing null check on line 42","path":"src/session.rs","line":42,"originalLine":42}]}]
JSON
    elif [[ "$ARGS" == *"reviews"* ]]; then
      cat <<'JSON'
[{"id":9001,"author":{"login":"alice"},"state":"CHANGES_REQUESTED","body":"The session timeout logic needs error handling for network failures."}]
JSON
    elif [[ "$ARGS" == *"state"* ]]; then
      echo "OPEN"
    else
      echo '{}'
    fi
    exit 0
    ;;

  "pr create")
    echo "101"
    exit 0
    ;;

  "pr edit")
    echo "Updated PR #$3."
    exit 0
    ;;

  "pr merge")
    echo "Merged PR #$3."
    exit 0
    ;;

  *)
    echo "fake-gh: unhandled: $*" >&2
    exit 1
    ;;
esac
GHEOF
chmod +x "$FAKE_BIN/gh"

# Prepend fake bin so jjj uses it
export PATH="$FAKE_BIN:$PATH"

# ── Setup repo ────────────────────────────────────────────────────────────────
setup_repo "github-test"

# ── 1: Issue import ───────────────────────────────────────────────────────────
section "Issue import"

run_jjj github import 42
assert_success "import issue #42"
assert_contains "Login is slow" "problem title from issue body"
assert_contains "42" "issue number in output"

run_jjj problem list
assert_success "problem list shows imported problem"
assert_contains "Login is slow" "imported problem in list"

# Priority from 'high' label is set on the problem (not shown in import output)
run_jjj problem list --json
assert_success "problem list JSON"
assert_contains '"high"' "priority mapped from 'high' label"

# Reimporting the same issue should report it already linked
run_jjj github import 42
assert_success "reimport is idempotent"
assert_contains "already linked" "duplicate import detected"

# ── 2: Import all ─────────────────────────────────────────────────────────────
section "Import all issues"

run_jjj github import --all
assert_success "import --all"
assert_contains "Memory leak" "issue #43 imported"

# Priority for #43 (critical label) appears in the problem JSON
run_jjj problem list --json
assert_success "problem list JSON after import --all"
assert_contains '"critical"' "priority mapped from 'critical' label"

run_jjj problem list
assert_success "both problems visible"
assert_contains "Login is slow" "first problem"
assert_contains "Memory leak" "second problem"

# Running --all again should find nothing new
run_jjj github import --all
assert_success "second import --all is safe"
assert_contains "No unlinked" "nothing left to import"

# ── 3: GitHub status ─────────────────────────────────────────────────────────
section "GitHub status"

run_jjj github status
assert_success "github status"
assert_contains "testowner/testrepo" "repo detected via fake gh"
assert_contains "testuser" "authenticated user shown"
assert_contains "Linked problems" "shows linked problems section"
assert_contains "42" "issue #42 in linked problems"
assert_contains "43" "issue #43 in linked problems"
assert_contains "Sync critiques: true" "critique sync enabled by default"
assert_contains "Auto-close on solve: false" "auto-close off by default"

# ── 4: Full lifecycle → solve with --github-close ────────────────────────────
section "Solve with --github-close"

run_jjj solution new "Add session keepalive" --problem "Login is slow"
assert_success "create solution for login problem"

run_jjj solution review "Add session keepalive"
assert_success "move solution to review"

run_jjj solution accept "Add session keepalive" --force
assert_success "force-accept solution"

run_jjj problem solve "Login is slow" --github-close
assert_success "solve closes problem and GitHub issue"
assert_contains "marked as solved" "problem is solved"
assert_contains "auto-closed GitHub issue #42" "GitHub issue closed"

run_jjj problem list --status open
assert_success "problem list open only"
assert_not_contains "Login is slow" "solved problem excluded from open list"

# ── 5: Dissolve with --github-close ──────────────────────────────────────────
section "Dissolve with --github-close"

run_jjj problem dissolve "Memory leak" \
    --reason "Turned out to be a test harness issue, not production code" \
    --github-close
assert_success "dissolve closes problem and GitHub issue"
assert_contains "dissolved" "problem is dissolved"
assert_contains "auto-closed GitHub issue #43" "GitHub issue #43 closed"

# ── 6: GitHub push ───────────────────────────────────────────────────────────
section "GitHub push"

# Problems #42 and #43 are already solved/dissolved above but the mock always
# reports them as OPEN, so github push will attempt to close them again
# (idempotent — the mock handler echoes and exits 0).
run_jjj github push
assert_success "push exits cleanly"
assert_contains "Closed issue" "push reconciles solved/dissolved problems"

# ── 7: Solution comment shorthand ────────────────────────────────────────────
section "Solution comment (critique reply)"

# Set up a fresh problem/solution/critique to test comment
run_jjj problem new "API timeout on large payloads"
assert_success "new problem for comment test"

run_jjj solution new "Stream large payloads" --problem "API timeout"
assert_success "new solution"

run_jjj critique new "Stream large payloads" "Backpressure not handled" --severity high
assert_success "add critique"

# comment with inline body and explicit critique reference
run_jjj solution comment "Stream large payloads" \
    --critique "Backpressure" \
    "Good point — I'll add flow control in the next commit"
assert_success "solution comment posts reply"
assert_contains "Replied to critique" "reply confirmed"

run_jjj critique show "Backpressure not handled" --json
assert_success "critique show"
assert_contains "Good point" "reply body persisted"
assert_contains "flow control" "full reply text present"

end_scenario
uxr_exit
