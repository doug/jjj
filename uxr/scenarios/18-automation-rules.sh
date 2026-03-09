#!/usr/bin/env bash
# Scenario 18: Automation Rules
#
# Verifies config-driven automation rules fire on jjj events:
# - Shell actions execute on problem_created
# - Template variables expand correctly
# - Disabled rules are skipped
# - Multiple rules for the same event all fire
# - Solved/dissolved events fire automation

source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "Automation Rules"

# ============================================================================
section "Setup: init repo and configure automation rules"
# ============================================================================

setup_repo "auto-rules"

run_jjj init
assert_success "jjj init"

# Write config with automation rules
# The marker file captures automation output so we can assert against it
MARKER="/tmp/jjj-uxr-auto-$$"
CONFIG_PATH="$(pwd)/.jj/jjj-meta/config.toml"

cat > "$CONFIG_PATH" << TOMLEOF
name = "automation-test"

[[automation]]
on = "problem_created"
action = "shell"
command = "echo 'CREATED: {{title}}' >> $MARKER"

[[automation]]
on = "problem_solved"
action = "shell"
command = "echo 'SOLVED: {{title}}' >> $MARKER"

[[automation]]
on = "problem_dissolved"
action = "shell"
command = "echo 'DISSOLVED: {{title}}' >> $MARKER"

[[automation]]
on = "solution_submitted"
action = "shell"
command = "echo 'SUBMITTED: {{title}} for {{problem.title}}' >> $MARKER"

[[automation]]
on = "solution_approved"
action = "shell"
command = "echo 'APPROVED: {{title}}' >> $MARKER"

[[automation]]
on = "critique_raised"
action = "shell"
command = "echo 'CRITIQUE: {{title}}' >> $MARKER"

[[automation]]
on = "critique_raised"
action = "shell"
command = "echo 'CRITIQUE2: {{title}}' >> $MARKER"
enabled = false
TOMLEOF

# ============================================================================
section "Problem creation fires automation"
# ============================================================================

rm -f "$MARKER"

run_jjj problem new "Fix login timeout" --priority p1 --force
assert_success "create problem"
assert_contains "auto: shell" "automation reports shell action"

# Check the marker file
OUTPUT=$(cat "$MARKER" 2>/dev/null || echo "")
assert_contains "CREATED: Fix login timeout" "shell wrote correct title"

# ============================================================================
section "Disabled rules are skipped"
# ============================================================================

run_jjj solution new "Add retry logic" --problem "Fix login timeout" --force
assert_success "create solution"

run_jjj critique new "Add retry logic" "Missing tests"
assert_success "create critique"

OUTPUT=$(cat "$MARKER" 2>/dev/null || echo "")
assert_contains "CRITIQUE: Missing tests" "enabled critique rule fired"
assert_not_contains "CRITIQUE2:" "disabled critique rule did not fire"

# ============================================================================
section "Multiple rules for same event fire in order"
# ============================================================================

# The first CRITIQUE rule fired, the second (disabled) did not.
# Verify only one CRITIQUE line exists.
CRITIQUE_LINES=$(grep -c "^CRITIQUE:" "$MARKER" 2>/dev/null || echo "0")
OUTPUT="$CRITIQUE_LINES lines"
assert_contains "1 lines" "exactly one critique rule fired"

# ============================================================================
section "Solution submit fires automation with template vars"
# ============================================================================

run_jjj critique address "Missing tests"
assert_success "address critique"

run_jjj solution submit "Add retry logic"
assert_success "submit solution"

OUTPUT=$(cat "$MARKER" 2>/dev/null || echo "")
assert_contains "SUBMITTED: Add retry logic for Fix login timeout" "template expanded problem.title"

# ============================================================================
section "Solution approve fires automation"
# ============================================================================

run_jjj solution approve "Add retry logic" --force --no-rationale
assert_success "approve solution"

OUTPUT=$(cat "$MARKER" 2>/dev/null || echo "")
assert_contains "APPROVED: Add retry logic" "approve rule fired"
assert_contains "SOLVED: Fix login timeout" "auto-solve triggered problem_solved rule"

# ============================================================================
section "Problem dissolve fires automation"
# ============================================================================

run_jjj problem new "False alarm" --priority p3 --force
assert_success "create second problem"

run_jjj problem dissolve "False alarm" --reason "was user error, not a bug"
assert_success "dissolve problem"

OUTPUT=$(cat "$MARKER" 2>/dev/null || echo "")
assert_contains "DISSOLVED: False alarm" "dissolve rule fired"

# ============================================================================
section "Cleanup"
# ============================================================================

rm -f "$MARKER"

end_scenario
uxr_exit
