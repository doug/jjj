# CLI Ergonomics Restructure Design

## Goal

Simplify and clarify the jjj CLI by consolidating overlapping view commands, removing the ambiguous `start` command, removing unused tags, and making `solution new` the single entry point for starting work.

## Architecture

Three changes: (1) merge `dashboard` and `next` into a new `status` command, (2) replace top-level `start` with `solution resume` and enhanced `solution new`, (3) remove the `tag` system entirely. These are independent changes that together produce a cleaner, less confusing CLI.

## Design Decisions

### 1. View Consolidation

**Remove:** `dashboard`, `next` (both absorbed into `status`)

**Add:** `jjj status` — single personal command combining:
- Active solution (what you're currently working on, its review/critique state)
- Next actions (prioritized BLOCKED/READY/REVIEW/WAITING/TODO items with suggested commands)
- Summary stats (project-level counts)

**Keep unchanged:** `jjj board` — project-wide kanban of solutions by status

**Distinction:**
- `status` = "Where am I and what should I do?" (personal, action-oriented)
- `board` = "Where is the project?" (team-wide, status-oriented)

**Output format:**
```
$ jjj status

Active: S-3 "Add auth middleware" → P-2 "User authentication"
  Status: testing | Reviewers: @alice (pending), @bob (signed off)
  Critiques: 1 open (CQ-5: "Missing CSRF protection" [high])

Next actions:
  [BLOCKED] S-3 has 1 open critique → jjj critique show CQ-5
  [REVIEW]  S-7 "Fix pagination" needs your review → jjj solution show S-7
  [TODO]    P-4 "API rate limiting" has no solutions → jjj solution new "title" --problem P-4

Summary: 3 open problems, 2 testing solutions, 1 open critique
```

**Flags:** `--all` (show all action items), `--mine` (only authored work), `--json` (structured output)

### 2. Start/Resume Rework

**Remove:** Top-level `jjj start` command

**Enhanced `solution new`:**
- Auto-attaches current jj change to the newly created solution
- Marks the solution as active
- If `--problem` is not provided, interactively lists open problems for selection
- Still accepts `--problem P-1` for non-interactive use

**New `solution resume`:**
- `jjj solution resume S-1` — resumes work on an existing solution
- Attaches current jj change and marks the solution as active

**Migration:**
- `jjj start S-1` → `jjj solution resume S-1`
- `jjj start "title" --problem P-1` → `jjj solution new "title" --problem P-1`
- `jjj start "title"` (no problem) → `jjj solution new "title"` (interactive problem selection)

### 3. Remove Tags

**Remove entirely:**
- `jjj tag` command (new, list, show, edit, delete)
- `tags: HashSet<String>` field from Problem, Solution, Milestone models
- `--tag` and `--add-tag`/`--remove-tag` flags from all new/edit commands
- Tag-related storage/serialization code
- Tag definitions from config

**Rationale:** Tags are write-only scaffolding — nothing reads, filters, or acts on them. Milestones and problem hierarchy already provide organization. YAGNI.

### 4. VS Code Extension Updates

The VS Code extension needs corresponding updates:
- Remove tag-related UI elements and interfaces
- Update Solution interface (remove tags)
- Update Problem interface (remove tags)
- Update tree view to reflect removal of tags
- No new VS Code commands needed for status (CLI-only concern)

## Final CLI Structure

### Top-level commands

| Command | Purpose | Change |
|---------|---------|--------|
| `jjj init` | Initialize project | Unchanged |
| `jjj status` | Personal state + next actions | **NEW** (replaces dashboard + next) |
| `jjj board` | Project-wide solution kanban | Unchanged |
| `jjj submit` | Submit current changes | Unchanged |
| `jjj review <reviewers>` | Shorthand: request review | Unchanged |
| `jjj lgtm` | Shorthand: sign off | Unchanged |
| `jjj resolve <id>` | Resolve conflicts | Unchanged |
| `jjj completion <shell>` | Shell completions | Unchanged |

### Entity subcommands

| Entity | Actions | Changes |
|--------|---------|---------|
| `problem` | new, list, show, edit, tree, solve, dissolve, assign | Remove --tag/--add-tag/--remove-tag |
| `solution` | new, list, show, edit, attach, detach, test, accept, refute, assign, review, lgtm, **resume** | **Add resume**, enhance new (auto-attach, interactive problem), remove tag flags |
| `critique` | new, list, show, edit, address, validate, dismiss, reply | Unchanged |
| `milestone` | new, list, show, edit, add-problem, remove-problem, roadmap, assign | Remove tag flags |

### Removed

- `jjj start` (top-level)
- `jjj dashboard`
- `jjj next`
- `jjj tag` (entire command group)
