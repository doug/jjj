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
│  p1 (open) ─────┬──── p2 (open)      p5 (solved)                        │
│                  │                                                           │
│                  └──── p3 (in_progress)                                    │
│                              │                                               │
│                              └──── p4 (open)                               │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            SOLUTIONS (Conjectures)                          │
│                                                                              │
│  proposed ────► testing ────► accepted                                      │
│                    │              │                                          │
│                    ▼              └───► Problem can be solved               │
│                 refuted                                                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CRITIQUES (Error Elimination)                       │
│                                                                              │
│  open ────┬──► addressed (solution modified)                                │
│           │                                                                  │
│           ├──► valid (critique correct → refute solution)                   │
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
# → p1 created

# 3. Decompose into sub-problems (optional)
jjj problem new "Token refresh fails silently" --parent p1
# → p2 created as child of p1

# 4. Propose a solution
jjj solution new "Use JWT with explicit refresh handling" --problem p2
# → s1 created (status: proposed)

# 5. Start working on the solution
jjj start s1
# → Creates jj change, attaches to s1, moves to "testing"
# → p2 moves to "in_progress"

# 6. Write code...
# Edit files, jj tracks changes

# 7. Someone critiques the solution
jjj critique new s1 "JWT tokens are vulnerable to XSS" --severity high
# → c1 created

# 8. Address or refute the critique
# Option A: Modify solution to address critique
jjj critique address c1

# Option B: Dismiss critique as invalid
jjj critique dismiss c1

# Option C: Validate critique (solution is flawed)
jjj critique validate c1
jjj solution refute s1
# → Need a new solution approach

# 9. Request code review
jjj review request @alice @bob
# → Review created for current change

# 10. Get approved and submit
jjj review approve
jjj submit
# → s1 accepted, prompts to solve p2

# 11. Mark problem solved
jjj problem solve p2
# → When all sub-problems solved, can solve p1
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
Solution s1
├── Critiques: c1, c2 (conceptual)
└── Changes: abc123, def456
    └── Reviews (code-level comments, separate system)
```

**What's Missing:**
- A code review comment saying "this approach won't scale" IS a critique
- But it lives in the Review system, not linked to the Solution
- The philosophical model says criticism should kill solutions, but code review comments don't

**More Elegant:**
Reviews should auto-create Critiques when "changes requested":
```bash
jjj review request-changes --message "This approach won't scale"
# → Automatically creates c-N linked to solution
# → Solution can't be accepted until critique addressed
```

### 2. The Solution ↔ Change Relationship is Weak

**The Problem:**
- A Solution can have multiple `change_ids[]`
- But there's no enforcement that changes match solutions
- You can `jjj submit` a change that isn't attached to any solution

**Current State:**
```bash
# This works but makes no sense:
jjj solution new "Fix auth" --problem p1  # s1
# ... edit code without jjj start ...
jjj submit --force  # What solution did this implement?
```

**More Elegant:**
The change's description could embed the solution ID:
```
s1: Fix authentication by adding JWT refresh

[Addresses p1: User authentication is unreliable]
```

Then `submit` could auto-detect which solution this implements.

### 3. Status Transitions Require Too Many Commands

**The Problem:**
The user must explicitly transition states that could be inferred:

```bash
jjj solution test s1      # Why? I started working = testing
jjj solution accept s1    # Why? My PR was approved = accepted
jjj problem solve p1      # Why? All solutions accepted = solved
```

**More Elegant:**
Infer status from actions:
- `jjj start s1` → auto `testing`
- `jjj submit` + review approved → auto `accept`
- All sub-problems solved → auto-prompt to solve parent

### 4. Critiques Don't Block Actions Strongly Enough

**The Problem:**
Open critiques warn but don't prevent acceptance:
```bash
jjj solution accept s1
# Warning: Solution has 2 open critiques
# (but proceeds anyway)
```

**Philosophically:**
A solution with unaddressed criticism shouldn't be accepted. Period.
That's the whole point of critical rationalism.

**More Elegant:**
```bash
jjj solution accept s1
# Error: Cannot accept - 2 open critiques
# c1: JWT tokens vulnerable to XSS [high]
# c2: No rate limiting [medium]
#
# Address with: jjj critique address c1
# Dismiss with:  jjj critique dismiss c1
# Or force:      jjj solution accept s1 --force
```

### 5. No Clear "What Should I Work On?" Flow

**The Problem:**
`jjj dashboard` shows information but doesn't guide action:
```
My Problems (2):
  p1 - Auth issues [in_progress]
  p3 - Slow queries [open]

My Solutions (1):
  s2 - Add caching [testing]

Open Critiques on My Solutions (3):
  c1, c2, c3
```

**What's Missing:**
- Which problem is highest priority?
- Which critique should I address first?
- What's blocking progress?

**More Elegant:**
```
Next Actions:

1. [BLOCKED] s2 has 3 unaddressed critiques
   → jjj critique show c1  (high severity)

2. [READY] p1 has accepted solution, ready to mark solved
   → jjj problem solve p1

3. [WAITING] Review pending for change abc123
   → Waiting on @alice

4. [TODO] p3 has no solutions proposed yet
   → jjj start "Solution title" --problem p3
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
# → Creates P-N from GitHub issue

jjj solution submit s1
# → Creates GitHub PR with description from Solution
# → PR comments sync back as Critiques
```

### 7. The "Dissolved" State is Underutilized

**The Problem:**
"Dissolved" means the problem was based on false premises.
But there's no workflow for discovering this.

**Current State:**
```bash
jjj problem dissolve p1  # Just a manual status change
```

**What's Missing:**
- When do you realize a problem should be dissolved?
- Often it's because a critique reveals the premise was wrong
- This should be a first-class workflow

**More Elegant:**
```bash
jjj critique validate c5
# "This critique shows the problem p3 is based on false premises"
# → Prompt: Dissolve problem p3? [y/N]
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

Both block solution acceptance until resolved.

### 2. Semantic Change Descriptions

```
jjj start "Use connection pooling" --problem p3
```

Creates change with description:
```
s7: Use connection pooling

Problem: p3 - Database queries are slow
Approach: Implement connection pooling to reduce connection overhead
```

Then all jj operations carry context.

### 3. Smart Status Inference

```rust
// Automatically move to testing when work starts
fn start(solution_id) {
    solution.status = Testing;
    problem.status = InProgress;
}

// Automatically accept when review passes + no critiques
fn submit() {
    if review.approved && critiques.all_resolved() {
        solution.accept();
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
- `jjj submit` → GitHub PR
- GitHub PR comments → jjj Critiques
- GitHub PR merge → jjj solution accept

---

## Summary

The philosophical model is sound and well-implemented. The friction points are:

| Issue | Impact | Fix Complexity |
|-------|--------|----------------|
| Reviews ≠ Critiques | High | Medium |
| Weak Solution↔Change link | Medium | Low |
| Too many manual transitions | Medium | Low |
| Critiques don't block | High | Low |
| No guided workflow | High | Medium |
| No GitHub integration | High | High |
| Dissolved underutilized | Low | Low |

The most impactful improvements would be:
1. **Unify Review/Critique** - One criticism system
2. **Add `jjj next`** - Guided workflow
3. **Enforce critique resolution** - Philosophy demands it
4. **GitHub integration** - Meet teams where they are

The elegance lies not in adding features, but in making the philosophy *inescapable* through the tooling. Right now, you can ignore critiques. That shouldn't be possible without explicit `--force` overrides.
