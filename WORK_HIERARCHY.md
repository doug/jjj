# Work Hierarchy: User-Centric Design

## Philosophy

Users don't think in abstract hierarchies. They think in **releases**, **features**, and **work**. Our structure should match how people naturally organize and communicate about software projects.

## The Simple Three-Level Structure

```
Milestone (When we ship)
  ├─ Feature (What users get)
  │   └─ Task (How we build it)
  └─ Bug (What we fix)
```

**That's it.** Clean, understandable, practical.

---

## From the User's Perspective

### "What are we shipping next?"
**Answer**: Milestones

```bash
jjj milestone list

Upcoming Milestones:
  M-1: v1.0 Release        [Dec 31, 2025]  ████████░░ 75%
  M-2: v1.1 Polish         [Mar 15, 2026]  ██░░░░░░░░ 20%
  M-3: Mobile Support      [Jun 30, 2026]  ░░░░░░░░░░  0%
```

**User story**: "I want to know when the next release is and what's in it."

### "What new capabilities are we building?"
**Answer**: Features

```bash
jjj feature list --milestone M-1

Features in v1.0:
  F-1: User Authentication     ████████░░ 80%  [bob]
  F-2: Export to PDF          ████░░░░░░ 40%  [alice]
  F-3: Dark Mode              ██████████ 100% ✓
```

**User story**: "I want to see what features we're building and who's working on them."

### "What am I working on today?"
**Answer**: Tasks

```bash
jjj task list --mine

My Tasks:
  T-5: Add password reset API        [In Progress]  F-2
  T-8: Write migration script         [TODO]         F-1
  T-12: Update login UI               [Review]       F-1
```

**User story**: "I want to see my work items and what feature they contribute to."

### "What problems need fixing?"
**Answer**: Bugs

```bash
jjj bug list --open

Open Bugs:
  B-3: [CRITICAL] Login timeout          [alice]    F-1
  B-7: [HIGH] Export crashes on large files [bob]   F-2
  B-9: [MEDIUM] Dark mode flickers       [unassigned]
```

**User story**: "I need to see what's broken and how serious it is."

---

## Work Item Types Explained

### 1. Milestone 📅

**What it is**: A release, sprint, or delivery target

**When to use**:
- Planning a release (v1.0, v2.0)
- Setting a sprint goal (Sprint 23)
- Organizing work by time (Q4 2025)

**What it contains**:
- Features you're shipping
- Bugs you're fixing
- Target date

**Real-world examples**:
- "v1.0 Beta" (first release)
- "Bug Bash Sprint" (focused on fixes)
- "Q4 Roadmap" (quarterly plan)

**Properties**:
```
M-1: v1.0 Release
  Target Date: December 31, 2025
  Status: Active
  Contains:
    - 5 features
    - 12 bugs
  Progress: 75% (15/20 items done)
```

### 2. Feature 🎯

**What it is**: A user-facing capability or improvement

**When to use**:
- Building something users will notice
- Improving existing functionality
- Adding new user-visible behavior

**What it contains**:
- Tasks (the technical work)
- Related bugs (issues with this feature)

**Real-world examples**:
- "User Authentication" (new capability)
- "Export to PDF" (new format)
- "Performance Improvements" (user-visible enhancement)
- "Redesigned Dashboard" (UX improvement)

**How to describe it**:
- ✅ "User Authentication" (clear capability)
- ✅ "Two-factor login" (specific feature)
- ❌ "Implement OAuth" (too technical - that's a task)
- ❌ "Backend refactor" (not user-facing - that's a chore)

**Properties**:
```
F-1: User Authentication
  Description: "Allow users to securely log in"
  Milestone: M-1 (v1.0 Release)
  Status: In Progress
  Assignee: bob@example.com
  Tasks: 8 total, 6 done
  Progress: 75%
  Priority: High
```

### 3. Task ✓

**What it is**: A specific piece of technical work

**When to use**:
- Breaking down a feature into doable chunks
- Tracking individual developer work
- Linking code changes to features

**What it belongs to**:
- **Always** belongs to a Feature
- Represents the "how" of building the feature

**Real-world examples**:
- "Add password hashing function" (under F-1: User Auth)
- "Create login API endpoint" (under F-1: User Auth)
- "Write authentication tests" (under F-1: User Auth)

**How to describe it**:
- ✅ "Implement password hashing" (specific, technical)
- ✅ "Add login API endpoint" (clear, actionable)
- ❌ "Authentication" (too broad - that's a feature)
- ❌ "Write some code" (too vague)

**Properties**:
```
T-5: Implement password hashing
  Feature: F-1 (User Authentication)
  Status: In Progress
  Assignee: bob@example.com
  Change IDs: kpqxywon
  Tags: backend, security
  Estimate: 4 hours
```

### 4. Bug 🐛

**What it is**: Something that's broken or not working as expected

**When to use**:
- Users report issues
- QA finds problems
- Developers discover defects

**What it links to**:
- **Optionally** links to Feature (if related)
- **Optionally** links to Milestone (if targeting fix)
- Can be standalone (not all bugs relate to features)

**Real-world examples**:
- "Login fails with special characters" (related to F-1)
- "App crashes on startup" (critical, standalone)
- "Dark mode colors are off" (related to F-3)

**How to describe it**:
- ✅ "Login fails with @ symbol in email" (specific)
- ✅ "Export hangs on files >10MB" (reproducible)
- ❌ "It doesn't work" (too vague)
- ❌ "Please add dark mode" (that's a feature request)

**Severity Levels**:
- **Critical**: System down, data loss
- **High**: Major functionality broken
- **Medium**: Feature impaired, workaround exists
- **Low**: Minor issue, cosmetic

**Properties**:
```
B-3: Login fails with special characters
  Severity: High
  Status: In Progress
  Feature: F-1 (optional link)
  Milestone: M-1 (fix target)
  Reporter: user@example.com
  Assignee: alice@example.com
  Repro: "Try email: test@domain.com"
```

---

## Relationship Rules

### Milestone ↔ Feature
- **One milestone** contains **many features**
- **One feature** can belong to **one milestone** (or none)
- Features without milestones go in the backlog

**Example**:
```
M-1: v1.0 Release
  ├─ F-1: User Authentication
  ├─ F-2: Export to PDF
  └─ F-3: Dark Mode

Backlog (no milestone):
  ├─ F-4: Mobile App
  └─ F-5: Advanced Reporting
```

### Feature ↔ Task
- **One feature** contains **many tasks**
- **One task** MUST belong to **one feature**
- No orphan tasks (every task has a feature)

**Why**: Tasks without features are confusing. If work doesn't fit a feature, it's probably:
- A bug fix (create a Bug)
- Infrastructure work (create a Feature called "Infrastructure Improvements")
- Technical debt (create a Feature called "Code Quality")

**Example**:
```
F-1: User Authentication
  ├─ T-1: Create user database schema
  ├─ T-2: Implement password hashing
  ├─ T-3: Add login API endpoint
  ├─ T-4: Create login UI
  └─ T-5: Write authentication tests
```

### Bug ↔ Feature
- **One bug** can relate to **one feature** (optional)
- **One feature** can have **many bugs**
- Bugs can be standalone (not related to any feature)

**When to link**:
- ✅ Bug in a specific feature → Link it
- ✅ General system bug → Leave standalone
- ✅ Bug blocks a feature → Link it

**Example**:
```
Linked Bugs:
  B-3: Login fails with @ symbol → F-1: User Auth
  B-7: Export crashes → F-2: Export to PDF

Standalone Bugs:
  B-9: App slow on startup (general performance)
  B-12: Typo in help text (cosmetic)
```

### Bug ↔ Milestone
- **One bug** can target **one milestone** (optional)
- **One milestone** can have **many bugs**
- Bugs without milestones are in the triage queue

**When to assign**:
- ✅ Critical bug → Assign to next milestone
- ✅ Bug blocking release → Assign to that release
- ✅ Nice-to-fix bug → Leave unassigned (backlog)

**Example**:
```
M-1: v1.0 Release
  Bugs to fix before release:
    ├─ B-3: Login fails [CRITICAL]
    ├─ B-7: Export crashes [HIGH]
    └─ B-15: UI flicker [MEDIUM]

Backlog Bugs (fix someday):
  ├─ B-9: Minor typo [LOW]
  └─ B-11: Edge case issue [LOW]
```

---

## User Workflows

### Workflow 1: Planning a Release

**User**: Project Manager

**Steps**:
```bash
# 1. Create milestone
jjj milestone new "v1.0 Release" --date 2025-12-31

# 2. Add features to milestone
jjj feature new "User Authentication" --milestone M-1
jjj feature new "Export to PDF" --milestone M-1

# 3. Check capacity
jjj milestone show M-1
# Shows: 2 features, 0% complete

# 4. Break features into tasks
jjj feature show F-1
# See what tasks are needed

# 5. Monitor progress
jjj milestone roadmap
# See timeline and completion
```

**User says**: "I want to plan what we're shipping and when."

### Workflow 2: Building a Feature

**User**: Developer

**Steps**:
```bash
# 1. See features to work on
jjj feature list --status "In Progress"

# 2. Pick a feature
jjj feature show F-1
# See: "User Authentication" needs 5 tasks

# 3. Create tasks
jjj task new "Implement password hashing" --feature F-1
jjj task new "Add login API" --feature F-1

# 4. Start work
jjj task attach T-1  # Attach current change
jjj task move T-1 "In Progress"

# 5. Submit for review
jjj review request alice
jjj task move T-1 "Review"

# 6. Complete
jjj task move T-1 "Done"

# 7. Check feature progress
jjj feature progress F-1
# Shows: 3/5 tasks done (60%)
```

**User says**: "I want to work on tasks and see how they contribute to features."

### Workflow 3: Triaging Bugs

**User**: QA Lead

**Steps**:
```bash
# 1. New bug reported
jjj bug new "Login fails with special chars" --severity high

# 2. Triage bugs
jjj bug list --status new

# 3. Link to feature if applicable
jjj bug link B-3 --feature F-1

# 4. Assign to milestone if blocking
jjj bug link B-3 --milestone M-1

# 5. Assign to developer
jjj bug assign B-3 alice

# 6. Track critical bugs
jjj bug list --severity critical --open
```

**User says**: "I need to organize bugs by severity and make sure blockers get fixed."

### Workflow 4: Daily Standup

**User**: Team Member

**Steps**:
```bash
# What did I do yesterday?
jjj task list --mine --status done --since yesterday
# Shows: T-5: "Implement hashing" ✓

# What am I doing today?
jjj task list --mine --status "In Progress"
# Shows: T-8: "Add login API"

# Any blockers?
jjj bug list --assigned-to me --severity high
# Shows: B-3: "Login fails" (blocking F-1)
```

**User says**: "I want a quick view of my work for standup."

### Workflow 5: Release Planning Meeting

**User**: Product Team

**Steps**:
```bash
# 1. Review current milestone
jjj milestone show M-1
# Status: 75% complete, 2 weeks left

# 2. Check feature status
jjj feature list --milestone M-1
# F-1: 80% done, F-2: 40% done, F-3: 100% done

# 3. Review bugs
jjj bug list --milestone M-1 --open
# 3 critical bugs still open

# 4. Decision: What's in/out?
jjj feature move F-2 --milestone M-2  # Push to next release
jjj bug link B-15 --milestone M-2      # Deprioritize

# 5. Create next milestone
jjj milestone new "v1.1" --date 2026-03-31
```

**User says**: "We need to decide what makes this release and what's pushed."

---

## Board Views for Different Audiences

### Developer View: Task Board
```bash
jjj board

┌─ TODO ────────┐ ┌─ In Progress ─┐ ┌─ Review ──────┐ ┌─ Done ────────┐
│ T-8: Add API  │ │ T-5: Hashing  │ │ T-12: Login UI│ │ T-1: Schema   │
│   [F-1]       │ │   [F-1]       │ │   [F-1]       │ │   [F-1]       │
│   @bob        │ │   @bob        │ │   @alice      │ │   ✓           │
│               │ │               │ │   ⚠ 2 comments│ │               │
└───────────────┘ └───────────────┘ └───────────────┘ └───────────────┘
```

### PM View: Feature Board
```bash
jjj feature board

┌─ Backlog ─────┐ ┌─ In Progress ─┐ ┌─ Review ──────┐ ┌─ Done ────────┐
│ F-4: Mobile   │ │ F-1: Auth     │ │ F-2: Export   │ │ F-3: Dark Mode│
│   0/0 tasks   │ │   6/8 tasks   │ │   3/5 tasks   │ │   4/4 tasks   │
│               │ │   ████████░░  │ │   ██████░░░░  │ │   ██████████  │
│               │ │   @bob        │ │   @alice      │ │   ✓           │
└───────────────┘ └───────────────┘ └───────────────┘ └───────────────┘
```

### Leadership View: Milestone Roadmap
```bash
jjj milestone roadmap

v1.0 Release ─────────────●────→ Dec 31, 2025
  ████████████████░░░░ 75% complete
  Features: 5 total, 4 in progress
  Bugs: 12 total, 3 critical open
  On track ✓

v1.1 Polish ──────────────────●→ Mar 31, 2026
  ████░░░░░░░░░░░░░░░░ 20% complete
  Features: 3 total, 1 in progress
  Bugs: 5 total, 0 critical
  On track ✓

Mobile Support ───────────────●→ Jun 30, 2026
  ░░░░░░░░░░░░░░░░░░░░ 0% complete
  Features: 8 planned
  Not started
```

---

## Naming and IDs

### ID Prefixes
- `M-1`, `M-2` - Milestones
- `F-1`, `F-2` - Features
- `T-1`, `T-2` - Tasks
- `B-1`, `B-2` - Bugs

**Why these prefixes?**
- **M**: Milestone (when)
- **F**: Feature (what)
- **T**: Task (how)
- **B**: Bug (broken)

Clear, unambiguous, easy to say aloud: "Em-one", "Eff-two", "Tee-three", "Bee-four"

### File Storage
```
jjj/meta/
├── milestones/
│   ├── M-1.toml          # Human-readable
│   └── M-2.toml
├── features/
│   ├── F-1.json
│   └── F-2.json
├── tasks/
│   ├── T-1.json
│   └── T-2.json
└── bugs/
    ├── B-1.json
    └── B-2.json
```

---

## Common Questions

### "Do I need to create a Milestone for everything?"
**No.** Features can live in the backlog without a milestone. Assign milestones when planning releases.

### "What if a task doesn't fit any feature?"
**Create a feature.** Common patterns:
- `F-99: Technical Improvements` (for refactoring)
- `F-100: Bug Fixes` (for standalone bugs)
- `F-101: Documentation` (for docs work)

### "Can a bug be a task?"
**No.** Bugs are reported problems. Tasks are planned work. But you can:
- Create a Bug (B-1: "Login fails")
- Fix it directly, OR
- Create a Task (T-5: "Fix login bug") under a feature
- Link them: `jjj bug link B-1 --task T-5`

### "What about technical debt?"
Create a Feature:
- `F-42: Code Quality Improvements`
- Add tasks: Refactor X, Update Y, Clean up Z

### "How do I track documentation?"
Create a Feature:
- `F-50: User Documentation`
- Add tasks: Write guide, Update API docs, Add examples

---

## Summary: The User-Centric View

Users think in terms of:

1. **"What are we shipping?"** → Milestones
2. **"What capabilities are we building?"** → Features
3. **"What work am I doing?"** → Tasks
4. **"What's broken?"** → Bugs

That's the mental model. That's what jjj should match.

**Simple. Clear. Practical.**
