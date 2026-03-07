# Testing the jjj Skill

Verify the jjj skill works correctly in Claude Code or another AI agent.

## Setup

1. Install jjj: `cargo install --path .` (or ensure `jjj` is on your PATH)
2. Install the skill:
   ```bash
   mkdir -p ~/.claude/skills/jjj && \
     cp docs-site/public/SKILL.md ~/.claude/skills/jjj/SKILL.md
   ```
3. Create a test repository:
   ```bash
   mkdir /tmp/jjj-skill-test && cd /tmp/jjj-skill-test
   jj git init
   jjj init
   ```

## Test Prompts

Run each prompt in Claude Code inside the test repository. Verify the agent uses the correct command and the output makes sense.

### 1. Problem creation

**Prompt:** "Create a high-priority problem called 'Login page crashes on empty password'"

**Expected:** Agent runs `jjj problem new "Login page crashes on empty password" --priority high`

### 2. Solution creation with problem link

**Prompt:** "Propose a solution called 'Add nil guard to auth handler' for the login crash problem"

**Expected:** Agent runs `jjj solution new "Add nil guard to auth handler" --problem "Login page crashes"`

### 3. Critique with severity

**Prompt:** "Add a medium-severity critique to the nil guard solution: 'Missing test for empty string vs nil'"

**Expected:** Agent runs `jjj critique new "nil guard" "Missing test for empty string vs nil" --severity medium`

### 4. Address a critique

**Prompt:** "Mark the 'Missing test' critique as addressed"

**Expected:** Agent runs `jjj critique address "Missing test"`

### 5. Submit and approve

**Prompt:** "Submit the nil guard solution for review, then approve it"

**Expected:** Agent runs `jjj solution submit "nil guard"` then `jjj solution approve "nil guard"`

### 6. Status check

**Prompt:** "What should I work on next?"

**Expected:** Agent runs `jjj status` or `jjj next`

### 7. Entity resolution — fuzzy title

**Prompt:** "Show me the login problem"

**Expected:** Agent runs `jjj problem show "login"` (fuzzy match, not full title)

### 8. JSON output

**Prompt:** "List all problems as JSON"

**Expected:** Agent runs `jjj problem list --json`

### 9. Milestone workflow

**Prompt:** "Create a milestone called 'v1.0' due June 1 2026, and add the login crash problem to it"

**Expected:** Agent runs `jjj milestone new "v1.0" --date 2026-06-01` then `jjj milestone add-problem "v1.0" "Login page crashes"`

### 10. Withdraw a solution

**Prompt:** "Withdraw the nil guard solution with rationale 'Superseded by input validation approach'"

**Expected:** Agent runs `jjj solution withdraw "nil guard" --rationale "Superseded by input validation approach"`

## Pass Criteria

- Agent uses correct command vocabulary (`submit`/`approve`/`withdraw`, not `accept`/`refute`)
- Agent uses fuzzy title matching (not requiring exact titles or UUIDs)
- Agent applies correct flags (`--priority`, `--severity`, `--rationale`, `--json`)
- Agent chains commands in logical order (create before reference, submit before approve)
- No hallucinated subcommands or flags

## Cleanup

```bash
rm -rf /tmp/jjj-skill-test
```
