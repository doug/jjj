# Your First Project

This guide walks you through a complete workflow using jjj to plan and track a release.

## Scenario

We're building a web application and need to release v1.0 with user authentication and basic CRUD operations.

## Step 1: Initialize jjj

```bash
# Navigate to your jj repository
cd ~/projects/my-web-app

# Initialize jjj
jjj init
```

Output:
```
Initialized jjj in ~/projects/my-web-app
Created metadata bookmark: jjj/meta
```

## Step 2: Create a Milestone

Plan your v1.0 release:

```bash
jjj milestone new "v1.0 Release" \
  --date 2025-12-31 \
  --description "Initial public release with core features"
```

Output:
```
Created milestone M-1 (v1.0 Release)
```

## Step 3: Define Features

Break down the work into features:

```bash
# Feature 1: User Authentication
jjj feature new "User Authentication" \
  --milestone M-1 \
  --priority critical \
  --description "Allow users to register, login, and manage their accounts"

# Feature 2: CRUD Operations
jjj feature new "Article CRUD" \
  --milestone M-1 \
  --priority high \
  --description "Create, read, update, delete articles"

# Feature 3: Nice-to-have
jjj feature new "Dark Mode" \
  --milestone M-1 \
  --priority low \
  --description "Toggle between light and dark themes"
```

Output:
```
Created feature F-1 (User Authentication) in milestone M-1
Created feature F-2 (Article CRUD) in milestone M-1
Created feature F-3 (Dark Mode) in milestone M-1
```

## Step 4: Break Features into Tasks

### Authentication Tasks

```bash
jjj task new "Set up database schema for users" \
  --feature F-1 \
  --tag backend \
  --tag database

jjj task new "Implement password hashing" \
  --feature F-1 \
  --tag backend \
  --tag security

jjj task new "Create registration API endpoint" \
  --feature F-1 \
  --tag backend \
  --tag api

jjj task new "Create login API endpoint" \
  --feature F-1 \
  --tag backend \
  --tag api

jjj task new "Build registration form" \
  --feature F-1 \
  --tag frontend

jjj task new "Build login form" \
  --feature F-1 \
  --tag frontend
```

### CRUD Tasks

```bash
jjj task new "Create articles table schema" \
  --feature F-2 \
  --tag backend \
  --tag database

jjj task new "Implement article create API" \
  --feature F-2 \
  --tag backend \
  --tag api

jjj task new "Implement article read API" \
  --feature F-2 \
  --tag backend \
  --tag api

jjj task new "Implement article update API" \
  --feature F-2 \
  --tag backend \
  --tag api

jjj task new "Implement article delete API" \
  --feature F-2 \
  --tag backend \
  --tag api

jjj task new "Build article editor UI" \
  --feature F-2 \
  --tag frontend

jjj task new "Build article list view" \
  --feature F-2 \
  --tag frontend
```

## Step 5: View Your Roadmap

Check your release plan:

```bash
jjj milestone roadmap
```

Output:
```
Roadmap:

📋 M-1 - v1.0 Release [2025-12-31]
   Initial public release with core features
   3 features, 0 bugs
```

View feature breakdown:

```bash
jjj feature list
```

Output:
```
Features:
  F-1 - User Authentication [Critical] [Backlog] (M-1) (6 tasks, 0 bugs)
  F-2 - Article CRUD [High] [Backlog] (M-1) (7 tasks, 0 bugs)
  F-3 - Dark Mode [Low] [Backlog] (M-1) (0 tasks, 0 bugs)
```

## Step 6: Start Working

### Begin with Task T-1

```bash
# View task details
jjj task show T-1
```

Output:
```
Task: T-1 - Set up database schema for users
Column: TODO
Feature: F-1
Tags: backend, database
Changes: []
Created: 2025-11-23 10:30:00
```

### Make Changes

```bash
# Create a new change
jj new -m "Add users table schema"

# Edit migration file
cat > migrations/001_create_users.sql <<EOF
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
EOF

# Attach change to task
jjj task attach T-1

# Move task to in progress
jjj task move T-1 "In Progress"
```

### View Progress

```bash
jjj feature progress F-1
```

Output:
```
Progress for F-1 - User Authentication

Status: Backlog
Priority: Critical

Task Breakdown:
  TODO: 5 (83%)
  In Progress: 1 (16%)
  Review: 0 (0%)
  Done: 0 (0%)

Overall: 0 / 6 tasks complete (0%)
Milestone: M-1
```

## Step 7: Complete and Review

### Finish the Task

```bash
# Mark task as ready for review
jjj task move T-1 "Review"

# Request code review
jjj review request @alice
```

### Alice Reviews

Alice can view and comment:

```bash
# Alice starts reviewing
jjj review start kpqxywon...

# Alice adds inline comment
jjj review comment kpqxywon... \
  --file migrations/001_create_users.sql \
  --line 3 \
  --body "Consider adding an index on email for faster lookups"

# Alice approves
jjj review approve kpqxywon...
```

### Move to Done

```bash
# After approval and merge
jjj task move T-1 "Done"
```

## Step 8: Handle a Bug

A bug is discovered during testing:

```bash
# Report the bug
jjj bug new "Login fails with special characters in email" \
  --severity high \
  --repro "1. Enter email with '+' sign\n2. Click login\n3. Observe error"

# Link to the authentication feature
jjj bug link B-1 --feature F-1 --milestone M-1

# Create a task to fix it
jjj task new "Fix email validation to allow RFC-compliant addresses" \
  --feature F-1 \
  --tag backend \
  --tag bugfix

# Work on the fix...
# ... make changes ...

# Link the fix to the bug
jjj task attach T-14

# Update bug status
jjj bug status B-1 fixed
```

## Step 9: Track Overall Progress

### View Dashboard

```bash
jjj dashboard
```

Output:
```
Dashboard

Pending Reviews:
  kpqxywon... - Add articles API endpoint (@alice)

Tasks:
  TODO: 10 tasks
  In Progress: 2 tasks
  Review: 1 task
  Done: 1 task

Recent Activity:
  T-1 moved to Done
  B-1 reported (high severity)
  T-14 created for bug fix
```

### View Feature Board

```bash
jjj feature board
```

Output:
```
Feature Board:

F-1 - User Authentication [Backlog] - 1 / 6 tasks (16%)
F-2 - Article CRUD [Backlog] - 0 / 7 tasks (0%)
F-3 - Dark Mode [Backlog] - 0 / 0 tasks (0%)
```

### Check Bug Triage

```bash
jjj bug triage
```

Output:
```
Bug Triage View

🔴 Critical (0):

🟠 High (1):
  B-1 - Login fails with special characters in email [Fixed]

🟡 Medium (0):

🟢 Low (0):

📋 Needs Attention:
  New bugs: 0
  Unassigned: 1

📊 Summary:
  Total bugs: 1
  Open: 0
  Resolved: 1
```

## Step 10: Release Planning

As the release date approaches:

```bash
# Check milestone progress
jjj milestone show M-1
```

Output:
```
Milestone: M-1 - v1.0 Release
Status: Active
Description: Initial public release with core features
Target Date: 2025-12-31
  40 days remaining

Features (3):
  - F-1
  - F-2
  - F-3

Bugs (1):
  - B-1

Created: 2025-11-23 10:00:00
Updated: 2025-11-23 15:30:00
```

### Update Feature Status

```bash
# Mark authentication as in progress
jjj feature move F-1 inprogress

# Mark CRUD as in progress
jjj feature move F-2 inprogress
```

## Step 11: Export for Reporting

Generate reports for stakeholders:

```bash
# Export milestone progress
jjj milestone show M-1 --json > reports/v1.0-progress.json

# Export all features
jjj feature list --json > reports/features.json

# Export bug triage
jjj bug triage --json > reports/bugs.json

# Use jq to create custom reports
jq '{
  milestone: "v1.0",
  features: length,
  tasks_done: [.[] | .task_ids] | flatten | length
}' reports/features.json
```

## Step 12: Sync with Team

Push metadata to share with your team:

```bash
# Push metadata bookmark
jj git push --bookmark jjj/meta

# Teammates can pull and see all tasks/features
jj git fetch
jj bookmark track jjj/meta@origin
```

## Workflow Summary

This complete workflow demonstrated:

1. ✅ Creating a milestone for release planning
2. ✅ Breaking down work into features and tasks
3. ✅ Tracking progress through Kanban workflow
4. ✅ Code review integration
5. ✅ Bug tracking and linking
6. ✅ Progress reporting and dashboards
7. ✅ Team collaboration via git push/pull

## Next Steps

Now that you've completed a full workflow:

- [**Work Hierarchy Guide**](../guides/work-hierarchy.md) - Deep dive into milestones, features, tasks, and bugs
- [**Code Review Guide**](../guides/code-review.md) - Master the review workflow
- [**CLI Reference**](../reference/cli.md) - Explore all available commands
- [**Examples**](../examples/feature-workflow.md) - See more real-world workflows

## Tips for Success

!!! tip "Start with Milestones"
    Always create milestones first to give your features context and deadlines.

!!! tip "Keep Features Small"
    Aim for 3-8 tasks per feature. Larger features should be split.

!!! tip "Use Tags Consistently"
    Establish tag conventions (e.g., `backend`, `frontend`, `api`, `ui`) early.

!!! tip "Review Regularly"
    Run `jjj dashboard` daily to stay on top of reviews and tasks.

!!! tip "Export Before Releases"
    Create JSON snapshots before major releases for historical records.
