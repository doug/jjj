# UXR (User Experience Research) Test Suite

Reproducible user journey simulations for jjj. Each scenario script simulates
a specific persona walking through jjj commands, asserting on outputs and
documenting friction points.

## Quick Start

```bash
# Run all scenarios
./uxr/run-all.sh

# Run a specific scenario
./uxr/run-all.sh 01          # solo quickstart
./uxr/run-all.sh conflict    # conflict resolution

# Save output for later analysis
./uxr/run-all.sh --save

# Keep temp directories for debugging
UXR_KEEP_TMPDIR=1 ./uxr/run-all.sh 04
```

## Scenarios

| # | Script | Persona | What It Tests |
|---|--------|---------|---------------|
| 01 | `solo-quickstart.sh` | Solo dev (Alice) | Quick Start guide walkthrough, entity resolution, error messages |
| 02 | `team-workflow.sh` | Team lead (Bob) | Milestones, assignments, reviewers, competing solutions, refutation |
| 03 | `new-contributor.sh` | New joiner (Charlie) | Discovery experience: status, list, show, help, discoverability |
| 04 | `conflict-resolution.sh` | Two concurrent users | Concurrent edits, frontmatter integrity, cascade effects, state machines |
| 05 | `error-recovery.sh` | Mistake-prone user | Invalid inputs, empty states, edge cases, JSON output, search |
| 06 | `end-to-end-showcase.sh` | New adopter | Full P→S→CQ lifecycle on a realistic multi-commit repo; push/fetch/sync help; timeline |
| 07 | `solution-lifecycle.sh` | Developer iterating | solution test, attach/detach, refute/accept with rationale, supersedes, assign, list filters |
| 08 | `critique-validate.sh` | Reviewer | critique validate, dismiss, reply, edit, file/line annotations, reviewer assignment |
| 09 | `events-audit.sh` | Tech lead auditing | events rebuild, validate, date/type/search/solution/problem filters, JSON output |
| 10 | `status-and-filtering.sh` | Power user | All status flags, all list filters, search --type/--text-only, db status/rebuild, completions |
| 11 | `milestone-advanced.sh` | Project manager | milestone edit, remove-problem, assign, roadmap progression, lifecycle to completion |

## How It Works

Each scenario:
1. Creates a fresh jj+jjj repo in `/tmp/jjj-uxr-$$`
2. Runs through a scripted user journey
3. Asserts on command success/failure and output content
4. Reports PASS/FAIL/SKIP with colored output
5. Cleans up temp files on exit

### Assertion Helpers

Scripts source `lib.sh` which provides:

- `run_jjj <args>` -- run a jjj command, capture output and exit code
- `assert_success "message"` -- last command should succeed
- `assert_failure "message"` -- last command should fail
- `assert_contains "needle" "message"` -- output should contain substring
- `assert_not_contains "needle" "message"` -- output should NOT contain substring
- `assert_matches "regex" "message"` -- output should match regex
- `assert_line_count_ge N "message"` -- output should have >= N lines
- `observe "message"` -- log a UX observation (not pass/fail)
- `skip "message"` -- mark a check as skipped

### Environment Variables

- `JJJ_BIN=/path/to/jjj` -- override the jjj binary path
- `UXR_KEEP_TMPDIR=1` -- don't clean up temp directories after run
- `UXR_TMPDIR=/custom/path` -- use a specific temp directory

## Analyzing Results

Run with `--save` to capture output:

```bash
./uxr/run-all.sh --save
```

This creates timestamped logs in `uxr/output/`:
- `01-solo-quickstart.log` -- full output per scenario
- `summary-YYYYMMDD-HHMMSS.txt` -- pass/fail summary

### With Claude

Feed the output to Claude for analysis:

```bash
./uxr/run-all.sh --save 2>&1 | pbcopy
# Paste into Claude: "Analyze these UXR test results..."
```

Or point Claude at the saved logs:

```bash
# In Claude Code:
# "Read uxr/output/01-solo-quickstart.log and analyze the UX friction points"
```

### Grep for Patterns

```bash
# Find all failures
grep "FAIL" uxr/output/*.log

# Find all UX observations
grep "NOTE" uxr/output/*.log

# Find error message quality issues
grep -A2 "Error" uxr/output/*.log
```

## Adding New Scenarios

1. Create `uxr/scenarios/NN-description.sh`
2. Source the helper library: `source "$(dirname "$0")/../lib.sh"`
3. Call `setup_repo "name"` to create a fresh environment
4. Use `run_jjj` + assertions to test the journey
5. Use `observe` for UX notes that aren't pass/fail
6. End with `end_scenario` and `uxr_exit`

Template:

```bash
#!/usr/bin/env bash
source "$(dirname "$0")/../lib.sh"
trap cleanup EXIT

begin_scenario "My New Scenario"
setup_repo "my-repo"
run_jjj init
assert_success "init"

# ... your journey here ...

end_scenario
uxr_exit
```

## Known Issues Tracked by These Tests

Scenarios are designed to catch regressions in:

- Entity resolution (title matching, UUID prefixes)
- State machine transitions (problem/solution/critique lifecycles)
- Error message quality (helpful vs. cryptic)
- CLI discoverability (--help, typo suggestions)
- Concurrent edit safety (frontmatter integrity)
- Empty state handling (no entities yet)
- Cascade behavior (delete/dissolve propagation)
