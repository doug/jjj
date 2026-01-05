# Work Item Hierarchy Design

## Overview

This document describes the enhanced hierarchical work item structure for jjj.

## Hierarchy Levels

```
Epic (E-1)
  └─ Milestone (M-1)
      ├─ Feature (F-1)
      │   └─ Task (T-1)
      │   └─ Task (T-2)
      ├─ Feature (F-2)
      │   └─ Task (T-3)
      ├─ Bug (B-1) [linked to F-1]
      ├─ Bug (B-2) [standalone]
      └─ Chore (C-1)
```

## Work Item Types

### 1. Epic (E-*)
**Purpose**: Strategic initiatives or large bodies of work spanning multiple milestones

**Properties**:
- Title (e.g., "Multi-tenant Support")
- Description (strategic goals)
- Owner/Champion
- Status (Planning, In Progress, Complete)
- Timeline (start/end dates)
- Child milestones
- Tags/Labels
- Budget/Resources (optional)

**Example**:
```
E-1: Multi-tenant Support
  Description: Enable the platform to support multiple isolated customers
  Owner: alice@example.com
  Status: In Progress
  Timeline: Q1 2025 - Q3 2025
  Milestones: M-1, M-2, M-3
```

### 2. Milestone (M-*)
**Purpose**: Time-boxed releases or project phases

**Properties**:
- Title (e.g., "v1.0 Release", "Q4 2025 Sprint")
- Description
- Target date (release date)
- Status (Planning, Active, Released, Cancelled)
- Parent epic (optional)
- Child features/bugs/chores
- Tags
- Version number (optional)

**Example**:
```
M-1: v1.0 Release
  Description: First production release
  Target Date: 2025-12-31
  Epic: E-1
  Status: Active
  Features: F-1, F-2, F-3
  Bugs: B-5, B-7
```

### 3. Feature (F-*)
**Purpose**: User-visible capabilities or improvements

**Properties**:
- Title (e.g., "User Authentication")
- Description (user story format)
- Parent milestone (optional)
- Parent epic (optional)
- Status (Backlog, In Progress, Review, Done)
- Assignee
- Child tasks
- Related bugs
- Tags
- Priority (High, Medium, Low)
- User story points (optional)

**Example**:
```
F-1: User Authentication
  Description: As a user, I want to log in securely
  Milestone: M-1
  Epic: E-1
  Status: In Progress
  Tasks: T-1, T-2, T-3
  Related Bugs: B-1
  Priority: High
```

### 4. Task (T-*)
**Purpose**: Individual units of technical work

**Properties**:
- Title (technical description)
- Description
- Parent feature (required)
- Status (TODO, In Progress, Review, Done)
- Assignee
- Attached change IDs
- Tags
- Estimate (hours/points)

**Example**:
```
T-1: Implement password hashing
  Feature: F-1
  Status: In Progress
  Assignee: bob@example.com
  Change IDs: kpqxywon
  Tags: backend, security
```

### 5. Bug (B-*)
**Purpose**: Defects or issues reported by users/QA

**Properties**:
- Title (issue description)
- Description (repro steps)
- Severity (Critical, High, Medium, Low)
- Status (New, Confirmed, In Progress, Fixed, Closed)
- Parent feature (optional - if related to specific feature)
- Parent milestone (optional - if targeted for release)
- Assignee
- Reporter
- Attached change IDs
- Tags
- Affected version
- Fixed in version

**Example**:
```
B-1: Login fails with special characters
  Severity: High
  Feature: F-1 (optional)
  Milestone: M-1
  Status: In Progress
  Reporter: user@example.com
  Assignee: alice@example.com
```

### 6. Chore (C-*)
**Purpose**: Technical work with no direct user-visible impact

**Properties**:
- Title (technical description)
- Description
- Parent milestone (optional)
- Status (TODO, In Progress, Done)
- Assignee
- Attached change IDs
- Tags
- Type (Refactoring, Documentation, DevOps, etc.)

**Example**:
```
C-1: Refactor database connection pooling
  Type: Refactoring
  Milestone: M-1
  Status: TODO
  Tags: infrastructure, performance
```

## Relationships

### Parent-Child
```
Epic
  ├─ contains → Milestone(s)
  └─ contains → Feature(s) [can skip milestone]

Milestone
  ├─ contains → Feature(s)
  ├─ contains → Bug(s)
  └─ contains → Chore(s)

Feature
  ├─ contains → Task(s) [required]
  └─ related to → Bug(s) [optional]
```

### Cross-Links
- Bug → Feature (optional)
- Bug → Milestone (optional)
- Feature → Epic (optional, if not in milestone)
- All items → Change IDs (via jj)

## Workflows

### Feature Development
```
1. Create Feature (F-1) under Milestone (M-1)
2. Break down into Tasks (T-1, T-2, T-3)
3. Assign tasks to developers
4. Track task progress on Kanban board
5. When all tasks done → Feature done
6. Link code reviews to tasks/feature
```

### Bug Triage
```
1. Create Bug (B-1)
2. Assess severity
3. Link to Feature (if applicable)
4. Assign to Milestone (if targeting release)
5. Create Task (T-x) to fix bug, or fix directly
6. Track fix progress
```

### Milestone Planning
```
1. Create Milestone (M-1) with target date
2. Add Features to milestone
3. Estimate capacity
4. Add high-priority Bugs
5. Monitor progress
6. Release when done
```

## Status Workflows

### Feature Status Flow
```
Backlog → In Progress → Review → Done
         ↓
      Blocked (can pause)
```

### Bug Status Flow
```
New → Confirmed → In Progress → Fixed → Closed
                      ↓
                  Won't Fix / Duplicate
```

### Milestone Status Flow
```
Planning → Active → Released
                 ↓
             Cancelled
```

## Board Views

### 1. Task Board (Current)
```
Columns: TODO | In Progress | Review | Done
Items: Tasks, Bugs (as tasks)
```

### 2. Feature Board
```
Columns: Backlog | In Progress | Review | Done
Items: Features
Shows: Progress bars (tasks completed/total)
```

### 3. Milestone Roadmap
```
Timeline view:
M-1 (v1.0) ─────────●─────────→ Dec 31
  F-1 ████████░░░░ 60%
  F-2 ████░░░░░░░░ 30%
  B-1 ████████████ 100%

M-2 (v1.1) ─────────────────●──→ Mar 31
  F-3 ░░░░░░░░░░░░ 0%
```

### 4. Epic Tracker
```
E-1: Multi-tenant Support
  M-1 (Q1) ████████████ 100% ✓
  M-2 (Q2) ████░░░░░░░░ 40%
  M-3 (Q3) ░░░░░░░░░░░░ 0%
```

## CLI Commands

### Epic Management
```bash
jjj epic new "Multi-tenant Support" --owner alice
jjj epic list
jjj epic show E-1
jjj epic add-milestone E-1 M-1
jjj epic progress E-1  # Show completion %
```

### Milestone Management
```bash
jjj milestone new "v1.0 Release" --date 2025-12-31
jjj milestone list
jjj milestone show M-1
jjj milestone add-feature M-1 F-1
jjj milestone roadmap  # Timeline view
```

### Feature Management
```bash
jjj feature new "User Authentication" --milestone M-1
jjj feature list --milestone M-1
jjj feature show F-1
jjj feature add-task F-1 T-1
jjj feature progress F-1  # Show task completion
jjj feature board  # Kanban for features
```

### Task Management (Enhanced)
```bash
jjj task new "Implement hashing" --feature F-1
jjj task list --feature F-1
jjj task show T-1
# ... existing commands ...
```

### Bug Management
```bash
jjj bug new "Login fails" --severity high
jjj bug list --severity critical
jjj bug show B-1
jjj bug link B-1 --feature F-1
jjj bug link B-1 --milestone M-1
jjj bug triage  # Interactive triage mode
```

### Chore Management
```bash
jjj chore new "Refactor DB" --type refactoring
jjj chore list
jjj chore show C-1
```

## Storage Structure

```
jjj/meta/
├── config.toml
├── epics/
│   └── E-1.json
├── milestones/
│   └── M-1.toml
├── features/
│   └── F-1.json
├── tasks/
│   └── T-1.json
├── bugs/
│   └── B-1.json
├── chores/
│   └── C-1.json
└── reviews/
    └── kpqxywon.../
```

## Reporting & Analytics

### Milestone Burndown
```bash
jjj milestone burndown M-1
# Shows: Features/tasks completed over time
```

### Feature Velocity
```bash
jjj metrics velocity --milestone M-1
# Shows: Story points completed per week
```

### Bug Trends
```bash
jjj bug trends
# Shows: Bugs opened vs closed over time
```

## Benefits of This Structure

### 1. **Flexibility**
- Epics are optional (for large projects)
- Features can exist without milestones (backlog)
- Bugs can be standalone or linked

### 2. **Clarity**
- Clear separation: Epic (why) → Milestone (when) → Feature (what) → Task (how)
- Bugs are first-class citizens
- Chores acknowledged as real work

### 3. **Tracking**
- Roll-up metrics (epic → milestone → feature → task)
- Multiple views for different audiences
- Progress visibility at all levels

### 4. **Real-World Alignment**
- Matches how teams actually work
- Supports agile/scrum and waterfall
- Scales from solo dev to large teams

## Migration Path

### Phase 1: Core Structure (Now)
- Implement Feature, Bug, Chore types
- Basic parent-child relationships
- Feature board view

### Phase 2: Milestones (Next)
- Milestone entity
- Milestone roadmap view
- Release planning

### Phase 3: Epics (Later)
- Epic entity
- Multi-milestone tracking
- Portfolio management

## Alternative Simplifications

### Option A: Two-Level (Simpler)
```
Feature (user-facing)
  └─ Task (technical work)
Bug (standalone defect)
```
- Pros: Simpler, easier to understand
- Cons: No release planning, no strategic view

### Option B: Three-Level (Balanced)
```
Milestone (release)
  ├─ Feature (capability)
  │   └─ Task (work)
  ├─ Bug (defect)
  └─ Chore (technical)
```
- Pros: Good balance, covers most needs
- Cons: No epic level for long-term planning

### Option C: Full Hierarchy (Recommended)
```
Epic (strategic)
  └─ Milestone (release)
      ├─ Feature → Task
      ├─ Bug
      └─ Chore
```
- Pros: Full flexibility, scales well
- Cons: More complex, steeper learning curve

## Recommendation

**Start with Option B (Three-Level)**, add Epics later if needed.

This gives you:
- ✅ Release planning (Milestones)
- ✅ User stories (Features)
- ✅ Work tracking (Tasks)
- ✅ Bug management (Bugs)
- ✅ Technical debt (Chores)

Most teams won't need Epics initially, but the structure allows adding them later without breaking changes.
