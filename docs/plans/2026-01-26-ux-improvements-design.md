# UX Improvements Design

## Overview

A comprehensive improvement to jjj's user experience across three areas: documentation alignment, UX automation, and model enrichment. The current documentation describes a Feature/Task/Bug model that no longer exists; the CLI requires too many manual state transitions; and the data model lacks fields needed for effective prioritization and knowledge capture.

## Goals

1. **Documentation matches reality** — every command snippet in docs is testable and runs against the real CLI
2. **Fewer manual transitions** — `jjj submit` handles acceptance and problem-solving automatically when conditions are met
3. **Richer model** — priority, dissolved reasons, and solution lineage make the Popperian workflow more expressive

## Area 1: Documentation Rewrite

### Problem

The getting-started docs describe Feature/Task/Bug commands that don't exist in the codebase. A new user following the Quick Start hits command-not-found errors on every step. Meanwhile, the actual Problem/Solution/Critique workflow and `jjj next` command are undocumented.

### Changes

**Replace getting-started docs:**

- `quick-start.md` — Rewrite from scratch. Walk through: `jjj init` → `problem new` → `solution new` → `start` → `critique new` → `critique address` → `solution accept` → `problem solve`. Every command is copy-pasteable and actually works.

- `index.md` — Rewrite overview to explain the Popperian model upfront: "Problems are things that need solving. Solutions are conjectures. Critiques eliminate errors." Remove all Feature/Task/Bug references.

**Rewrite reference section:**

Replace `cli-feature.md`, `cli-task.md`, `cli-bug.md`, `cli-review.md` with:
- `cli-problem.md` — All problem subcommands with examples
- `cli-solution.md` — All solution subcommands with examples
- `cli-critique.md` — All critique subcommands with examples
- `cli-milestone.md` — All milestone subcommands
- `cli-workflow.md` — `start`, `submit`, `next`, `review`, `lgtm`, `board`, `dashboard`

**Rewrite guides:**

- `task-management.md` → `problem-solving.md` — When to create problems, decomposition, when to dissolve vs solve
- `code-review.md` — Update to show critique-based review flow
- `board-dashboard.md` — Update to show Problem/Solution columns
- Add `critique-guidelines.md` — Severity levels, when to address vs dismiss, how to write effective critiques

**Fix SUMMARY.md** to match the new file structure.

### Testable Documentation

Every command snippet in the docs is extracted and run as an integration test. If the CLI changes and the docs don't update, CI fails.

**Markdown annotation:**

Fenced code blocks that should be tested use a `,test` marker:

````markdown
```bash,test
jjj init
jjj problem new "Search is slow"
jjj solution new "Add search index" --problem P-1
```
````

Blocks without `,test` are display-only (conceptual examples, output samples).

**Optional output assertions:**

```bash,test
jjj problem list --json
# expect: "P-1"
# expect: "Search is slow"
```

The harness greps stdout for each `# expect:` line.

**Test harness: Rust integration test** (`tests/doc_test.rs`)

1. Scans all `.md` files in `docs/` for ``` ```bash,test ``` blocks
2. Groups blocks by file (each doc page runs in sequence within a fresh temp repo)
3. For each file: creates a temp jj repo, runs `jjj init`, then executes each block in order
4. Asserts exit code 0 (command succeeded)
5. Checks `# expect:` assertions against stdout

**What this catches:**
- Renamed/removed commands (exit code != 0)
- Changed flag names (exit code != 0)
- Changed output format (expect line not found)
- Wrong argument order in examples

## Area 2: UX Automation

### Problem

The current workflow requires too many manual commands for state transitions the system could infer. After critiques are resolved and reviews pass, users must separately call `solution accept` and then `problem solve`.

### Changes

**Smarter `jjj submit`:**

Currently submit just squashes. It should also:

1. Detect the solution attached to the current change
2. Check if all critiques are resolved (addressed or dismissed)
3. Check if all requested reviewers have LGTM'd
4. If both pass → auto-accept the solution
5. If the solution is the only active solution for its problem, and the problem has no other open sub-problems → prompt to auto-solve the problem
6. If blocked, print exactly what's blocking with remediation commands

Blocked output:

```
$ jjj submit
Submitting S-1: Add search index

  ✗ 1 open critique:
    CQ-3: SQL injection risk [high]
    → jjj critique address CQ-3

  ✗ Review pending from @alice
    → waiting for LGTM

Cannot auto-accept. Use --force to submit without acceptance.
```

Happy path:

```
$ jjj submit
Submitting S-1: Add search index

  ✓ All critiques resolved
  ✓ LGTM from @alice, @bob
  → Solution S-1 accepted
  → Problem P-1 solved (only solution, no open sub-problems)

Squashed changes into trunk.
```

**Premature solve warning:**

`jjj problem solve` warns if there are active solutions still in testing.

**No other auto-transitions:**

`proposed → testing` already happens via `start`. `refuted` is always a deliberate human judgment. `dissolved` is always deliberate. Automation only where the intent is unambiguous.

## Area 3: Model & Philosophy Gaps

### 3a. Priority on Problems

Add an optional `priority` field to Problem: `P0`/`critical`, `P1`/`high`, `P2`/`medium`, `P3`/`low`. Defaults to `P2`/`medium`.

The CLI accepts both formats interchangeably:

```bash
jjj problem new "Auth broken" --priority P0
jjj problem new "Auth broken" --priority critical
# Both do the same thing
```

Display shows both:

```
P-1: Auth broken [open] P0/critical
```

Mapping: `P0 = critical`, `P1 = high`, `P2 = medium`, `P3 = low`.

`jjj next` uses priority as a tiebreaker within each category (BLOCKED items with critical critiques surface first, TODO items with higher priority surface first).

### 3b. Dissolved Reason

When dissolving a problem, require a reason:

```bash
jjj problem dissolve P-3 --reason "The data was correct; our test was wrong"
```

Dissolving means "the problem was based on false premises." Recording why creates institutional knowledge. Store as `dissolved_reason: Option<String>` on Problem.

### 3c. Solution Lineage

When a solution is refuted, users often propose a new one for the same problem. Add an optional `supersedes: Option<String>` field to Solution:

```bash
jjj solution new "Use parameterized queries" --problem P-1 --supersedes S-1
```

Creates a traceable chain: S-1 (refuted) → S-2 (supersedes). The project tree and virtual docs show this lineage. Not required — most solutions won't use it.

### 3d. Review/Critique Unification — Deferred

Code review (LGTM/requested reviewers) and critiques remain separate systems for now. Reviews gate on "does the code look right?", critiques gate on "is the approach sound?" Both must pass for acceptance. Full unification is a separate design effort.

## Implementation Order

1. **Documentation rewrite** — unblocks new users immediately
2. **Model changes** — priority, dissolved reason, solution lineage (data model before UX)
3. **UX automation** — smarter submit depends on model being right
4. **Testable docs** — needs the docs and CLI to be stable first

## Files Affected

### Documentation (create/rewrite)
- `docs/index.md` — rewrite
- `docs/SUMMARY.md` — rewrite
- `docs/getting-started/quick-start.md` — rewrite
- `docs/getting-started/installation.md` — review/update
- `docs/guides/problem-solving.md` — new (replaces task-management.md)
- `docs/guides/critique-guidelines.md` — new
- `docs/guides/code-review.md` — rewrite
- `docs/guides/board-dashboard.md` — rewrite
- `docs/reference/cli-problem.md` — new
- `docs/reference/cli-solution.md` — new
- `docs/reference/cli-critique.md` — new
- `docs/reference/cli-milestone.md` — new
- `docs/reference/cli-workflow.md` — new
- `docs/reference/configuration.md` — new
- Old `cli-feature.md`, `cli-task.md`, `cli-bug.md`, `cli-review.md` — delete

### Rust (model changes)
- `src/models/problem.rs` — add `priority: Priority`, `dissolved_reason: Option<String>`
- `src/models/solution.rs` — add `supersedes: Option<String>`
- `src/cli.rs` — add `--priority`, `--reason`, `--supersedes` flags
- `src/commands/problem.rs` — priority display, dissolve reason requirement
- `src/commands/solution.rs` — lineage display
- `src/commands/workflow.rs` — smarter submit logic
- `src/commands/next.rs` — priority-aware sorting
- `src/tui.rs` — priority display in board/dashboard

### Tests
- `tests/doc_test.rs` — new (testable documentation harness)
- `tests/workflow_test.rs` — update for smarter submit
- `tests/integration_test.rs` — update for new model fields

### VS Code Extension
- `vscode/src/cli.ts` — add priority, dissolved_reason, supersedes to interfaces
- `vscode/src/documents/entityDocumentProvider.ts` — render new fields
- `vscode/src/views/projectTreeProvider.ts` — show priority indicators
- `vscode/src/views/nextActionsProvider.ts` — priority-aware display
