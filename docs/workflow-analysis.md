---
title: Workflow Analysis
description: A deep dive into the jjj workflow, philosophy, and areas for improvement.
---

# jjj Workflow Analysis

## The Complete User Journey

### Core Philosophy

jjj implements Popperian epistemology for software development:

1. **Problems are fundamental** - All work begins with identifying problems
2. **Solutions are conjectures** - Tentative attempts that may be wrong
3. **Criticism drives progress** - Error elimination through explicit critique
4. **Knowledge grows by refutation** - We learn more from failure than success

### Current Workflow

```
                    ┌─────────────────────────────────────────────────┐
                    │                  MILESTONES                      │
                    │  (Time-boxed cycles of problems to solve)        │
                    └─────────────────────────────────────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              PROBLEM DAG                                     │
│                                                                              │
│  "Auth issues" (open) ──┬── "Token refresh" (open)    "Perf" (solved)    │
│                          │                                                   │
│                          └── "Session mgmt" (in_progress)                  │
│                                       │                                      │
│                                       └── "Cookie handling" (open)         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            SOLUTIONS (Conjectures)                          │
│                                                                              │
│  proposed ────► submitted ────► approved                                       │
│                    │              │                                          │
│                    ▼              └───► Problem can be solved               │
│                 withdrawn                                                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CRITIQUES (Error Elimination)                       │
│                                                                              │
│  open ────┬──► addressed (solution modified)                                │
│           │                                                                  │
│           ├──► valid (critique correct → withdraw solution)                 │
│           │                                                                  │
│           └──► dismissed (critique incorrect)                               │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            CODE CHANGES (jj)                                │
│                                                                              │
│  Solution.change_ids[] ←──── jj changes implementing the solution           │
│                                                                              │
│  Review system: request → pending → approved/changes_requested              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Typical User Journey

```bash
# 1. Initialize project
jjj init

# 2. Identify a problem
jjj problem new "User authentication is unreliable"

# 3. Decompose into sub-problems (optional)
jjj problem new "Token refresh fails silently" --parent "authentication"

# 4. Propose a solution
jjj solution new "Use JWT with explicit refresh handling" --problem "Token refresh"

# 5. Start working on the solution
jjj solution resume "JWT with explicit"
# → Creates jj change, attaches to solution (stays "proposed" until solution submit)
# → "Token refresh" moves to "in_progress"

# 6. Write code...
# Edit files, jj tracks changes

# 7. Someone critiques the solution
jjj critique new "JWT with explicit" "JWT tokens are vulnerable to XSS" --severity high

# 8. Address or refute the critique
# Option A: Modify solution to address critique
jjj critique address "vulnerable to XSS"

# Option B: Dismiss critique as invalid
jjj critique dismiss "vulnerable to XSS"

# Option C: Validate critique (solution is flawed)
jjj critique validate "vulnerable to XSS"
jjj solution withdraw "JWT with explicit"
# → Need a new solution approach

# 9. Submit for review (with reviewers assigned at creation or via critique)
jjj solution submit "JWT with explicit"

# 10. Get approved
jjj solution approve "JWT with explicit"
# → Solution approved, prompts to solve problem

# 11. Mark problem solved
jjj problem solve "Token refresh"
# → When all sub-problems solved, can solve parent
```

---

## Critical Analysis: Where It Doesn't Work

### 1. Reviews and Critiques are Disconnected

**The Problem:**
- `Review` is for code changes (line-by-line comments)
- `Critique` is for solutions (conceptual criticism)
- These should be the same thing

**Current State:**
```
Solution "JWT with explicit refresh"
├── Critiques: "XSS vulnerability", "No rate limiting" (conceptual)
└── Changes: abc123, def456
    └── Reviews (code-level comments, separate system)
```

**What's Missing:**
- A code review comment saying "this approach won't scale" IS a critique
- But it lives in the Review system, not linked to the Solution
- The philosophical model says criticism should kill solutions, but code review comments don't

**More Elegant:**
Reviews now auto-create Critiques when "changes requested" via GitHub sync:
```bash
# GitHub "Request Changes" review → imported as critique
# → Solution can't be approved until critique addressed
```

### 2. The Solution ↔ Change Relationship is Weak

**The Problem:**
- A Solution can have multiple `change_ids[]`
- But there's no enforcement that changes match solutions
- You can work on a change that isn't attached to any solution

**Current State:**
```bash
# This works but makes no sense:
jjj solution new "Fix auth" --problem "authentication"
# ... edit code without jjj solution resume ...
jjj solution submit "Fix auth"  # What change implements this?
```

**More Elegant:**
The change's description could embed the solution ID:
```
Use JWT with explicit refresh handling

[Addresses: User authentication is unreliable]
```

Then `solution submit` could auto-detect which solution this implements.

### 3. Status Transitions Require Too Many Commands

**The Problem:**
The user must explicitly transition states that could be inferred:

```bash
jjj solution submit "JWT refresh"  # Why? I'm ready for review
jjj solution approve "JWT refresh"  # Why? My PR was approved
jjj problem solve "Token refresh"  # Why? All solutions approved = solved
```

**More Elegant:**
Infer status from actions:
- `jjj solution resume "JWT refresh"` → stays `proposed`; run `solution submit` when ready
- All critiques resolved + review approved → `solution approve`
- All sub-problems solved → auto-prompt to solve parent

### 4. Critiques Don't Block Actions Strongly Enough

**The Problem:**
Open critiques warn but don't prevent approval:
```bash
jjj solution approve "JWT refresh"
# Warning: Solution has 2 open critiques
# (but proceeds anyway)
```

**Philosophically:**
A solution with unaddressed criticism shouldn't be approved. Period.
That's the whole point of critical rationalism.

**More Elegant:**
```bash
jjj solution approve "JWT refresh"
# Error: Cannot approve - 2 open critiques
# "XSS vulnerability": JWT tokens vulnerable to XSS [high]
# "No rate limiting": Missing rate limiting [medium]
#
# Address with: jjj critique address "XSS vulnerability"
# Dismiss with:  jjj critique dismiss "XSS vulnerability"
# Or force:      jjj solution approve "JWT refresh" --force
```

### 5. No Clear "What Should I Work On?" Flow

**The Problem:**
`jjj dashboard` shows information but doesn't guide action:
```
My Problems (2):
  "Auth issues"   [in_progress]
  "Slow queries"  [open]

My Solutions (1):
  "Add caching"   [review]

Open Critiques on My Solutions (3):
  "XSS vulnerability", "No rate limiting", "Missing pagination"
```

**What's Missing:**
- Which problem is highest priority?
- Which critique should I address first?
- What's blocking progress?

**More Elegant:**
```
Next Actions:

1. [BLOCKED] "Add caching" has 3 unaddressed critiques
   → jjj critique show "XSS vulnerability"  (high severity)

2. [READY] "Auth issues" has approved solution, ready to mark solved
   → jjj problem solve "Auth issues"

3. [WAITING] Review pending for change abc123
   → Waiting on @alice

4. [TODO] "Slow queries" has no solutions proposed yet
   → jjj solution new "Solution title" --problem "Slow queries"
```

### 6. GitHub/External Integration Gap

**The Problem:**
Many teams use GitHub Issues and PRs. jjj is isolated.

**What's Missing:**
- GitHub Issue → jjj Problem mapping
- GitHub PR → jjj Solution + Review mapping
- PR comments → jjj Critiques

**More Elegant:**
```bash
jjj problem import github#123
# → Creates problem from GitHub issue

jjj solution submit "JWT refresh"
# → Creates GitHub PR with description from Solution
# → PR comments sync back as Critiques
```

### 7. The "Dissolved" State is Underutilized

**The Problem:**
"Dissolved" means the problem was based on false premises.
But there's no workflow for discovering this.

**Current State:**
```bash
jjj problem dissolve "Auth issues"  # Just a manual status change
```

**What's Missing:**
- When do you realize a problem should be dissolved?
- Often it's because a critique reveals the premise was wrong
- This should be a first-class workflow

**More Elegant:**
```bash
jjj critique validate "false premise"
# "This critique shows the problem is based on false premises"
# → Prompt: Dissolve problem "Slow queries"? [y/N]
```

---

## What Would Make This More Elegant

### 1. Unify Review and Critique

```rust
enum Criticism {
    Critique {       // Conceptual criticism of the approach
        solution_id: String,
        argument: String,
    },
    CodeComment {    // Line-level criticism of implementation
        change_id: String,
        file: String,
        line: usize,
        body: String,
    }
}
```

Both block solution approval until resolved.

### 2. Semantic Change Descriptions

```
jjj solution resume "Use connection pooling" --problem "Slow queries"
```

Creates change with description:
```
Use connection pooling

Problem: Database queries are slow
Approach: Implement connection pooling to reduce connection overhead
```

Then all jj operations carry context.

### 3. Smart Status Inference

```rust
// Automatically move to review when work starts
fn start(solution_id) {
    solution.status = Review;
    problem.status = InProgress;
}

// Automatically approve when review passes + no critiques
fn approve() {
    if review.approved && critiques.all_resolved() {
        solution.approve();
        if problem.can_solve() {
            prompt_solve_problem();
        }
    }
}
```

### 4. First-Class "What Next?" Command

```bash
jjj next
# Shows prioritized action list based on:
# - Blocking critiques (highest priority)
# - Pending reviews (time-sensitive)
# - Solvable problems (quick wins)
# - Open problems (new work)
```

### 5. Bidirectional GitHub Sync

```yaml
# .jjj/config.toml
[github]
repo = "owner/repo"
sync_issues = true
sync_prs = true
```

Then:
- `jjj problem new` → GitHub Issue
- `jjj solution submit` → GitHub PR (via automation rules)
- GitHub PR comments → jjj Critiques
- GitHub PR merge → jjj solution approve

---

## Summary

The philosophical model is sound and well-implemented. The friction points are:

| Issue | Impact | Fix Complexity |
|-------|--------|----------------|
| Reviews ≠ Critiques | High | Done (unified) |
| Weak Solution↔Change link | Medium | Low |
| Too many manual transitions | Medium | Done (auto-solve) |
| Critiques don't block | High | Done (enforced) |
| No guided workflow | High | Done (`jjj next`) |
| No GitHub integration | High | Done (`jjj github`) |
| Dissolved underutilized | Low | Low |

The most impactful improvements were:
1. **Unify Review/Critique** - Done: review requests are now critiques with `--reviewer`
2. **Add `jjj next`** - Done: guided workflow with prioritized actions
3. **Enforce critique resolution** - Done: `solution approve` blocks on open critiques
4. **GitHub integration** - Done: `jjj github` with bidirectional sync

The elegance lies not in adding features, but in making the philosophy *inescapable* through the tooling. Open critiques now block approval — you must explicitly `--force` to override.
