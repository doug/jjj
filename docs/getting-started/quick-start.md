# Quick Start

Get up and running with jjj in 5 minutes!

## Initialize jjj

First, navigate to your Jujutsu repository and initialize jjj:

```bash
cd /path/to/your/jj/repo
jjj init
```

This creates the metadata structure in your repository:

```
.jj/
└── jjj-meta/          # Metadata workspace
    ├── config.toml    # Project configuration
    ├── tasks/         # Task storage
    ├── features/      # Feature storage
    ├── milestones/    # Milestone storage
    ├── bugs/          # Bug storage
    └── reviews/       # Code review data
```

> **What just happened?**
>
> jjj created an orphaned commit history (shadow graph) to store metadata separately from your project code. This bookmark is called `jjj/meta`.
## Create Your First Task

Let's create a simple task:

```bash
# First, create a feature to organize tasks
jjj feature new "Getting Started"

# This creates F-1, now create a task
jjj task new "Set up development environment" --feature F-1
```

You've created your first task! It's assigned ID `T-1`.

## View Your Board

See all tasks on the Kanban board:

```bash
jjj board
```

You'll see an interactive TUI board with your task in the "TODO" column:

```
┌─────────┬──────────────┬──────┬──────┐
│ TODO    │ In Progress  │Review│ Done │
├─────────┼──────────────┼──────┼──────┤
│ T-1     │              │      │      │
│ Set up  │              │      │      │
│ dev env │              │      │      │
└─────────┴──────────────┴──────┴──────┘
```

Press `q` to exit the TUI.

## Work on a Task

### 1. Attach a Change to a Task

Start working on the task by attaching your current change:

```bash
# Make some code changes
echo "# My Project" > README.md

# Attach the current change to the task
jjj task attach T-1
```

Now your change is associated with T-1!

### 2. Move Task Through Workflow

Move the task to "In Progress":

```bash
jjj task move T-1 "In Progress"
```

### 3. Complete the Task

When you're done, move it to "Done":

```bash
jjj task move T-1 "Done"
```

## Request a Code Review

Get feedback on your changes:

```bash
# Request review from teammates
jjj review request @alice @bob

# Check review status
jjj review status
```

## View Dashboard

See everything at a glance:

```bash
jjj dashboard
```

Output:

```
Dashboard

Pending Reviews:
  kpqxywon... - Add README (You requested - @alice, @bob)

Tasks:
  TODO: 0 tasks
  In Progress: 0 tasks
  Review: 0 tasks
  Done: 1 task

Recent Activity:
  T-1 moved to Done
  Review requested for kpqxywon...
```

## Key Concepts

### Work Hierarchy

jjj uses a three-level hierarchy:

```
Milestone (M-1)
  ├── Feature (F-1)
  │     ├── Task (T-1)
  │     ├── Task (T-2)
  │     └── Task (T-3)
  ├── Feature (F-2)
  └── Bug (B-1)
```

- **Milestones**: Release targets (e.g., "v1.0 Release")
- **Features**: User-facing capabilities (e.g., "User Authentication")
- **Tasks**: Individual work items (MUST belong to a feature)
- **Bugs**: Defects (can be standalone or linked)

### Change IDs vs Commit Hashes

jjj uses Jujutsu's **change IDs** (not commit hashes) because they:

- ✅ Stay stable across rebases
- ✅ Survive history rewrites
- ✅ Make metadata tracking robust

This means:

```bash
# Attach task to current change
jjj task attach T-1

# Rebase your changes
jj rebase -d main

# Task is still attached! Change ID hasn't changed.
```

### Shadow Graph

All jjj metadata lives in a separate commit history:

- **Never pollutes your project history**
- **Can be pushed/pulled independently**
- **Easy to reset if needed**

View the shadow graph:

```bash
jj log -r jjj/meta
```

## Syncing with Team

To share your tasks and reviews, you need to sync the `jjj/meta` bookmark.

### Pushing Changes
When you push your code, also push the metadata:

```bash
jj git push -b jjj/meta
```

### Fetching Updates
To see updates from your team:

```bash
jj git fetch
```

> **Tip:** Make sure you're tracking the remote bookmark:
> `jj bookmark track jjj/meta@origin`

## Common Commands

Here are the most frequently used commands:

| Command | Description |
|---------|-------------|
| `jjj board` | Show Kanban board |
| `jjj dashboard` | Show overview dashboard |
| `jjj task new "title" --feature F-1` | Create a task |
| `jjj task attach T-1` | Attach current change to task |
| `jjj task move T-1 "Done"` | Move task to column |
| `jjj task list` | List all tasks |
| `jjj feature new "title"` | Create a feature |
| `jjj feature progress F-1` | Show feature progress |
| `jjj milestone new "title" --date YYYY-MM-DD` | Create milestone |
| `jjj milestone roadmap` | Show release roadmap |
| `jjj bug new "title" --severity high` | Report a bug |
| `jjj bug triage` | View bug triage |
| `jjj review request @user` | Request code review |
| `jjj review status` | Check review status |

## JSON Output

Every command supports `--json` for scripting:

```bash
# Get tasks as JSON
jjj task list --json

# Get board data as JSON
jjj board --json

# Parse with jq
jjj feature list --json | jq '.[] | select(.status == "InProgress")'
```

## Next Steps

You now know the basics! Continue learning:

- [**Your First Project**](first-project.md) - Complete walkthrough of a real workflow
- [**Work Hierarchy Guide**](../guides/work-hierarchy.md) - Master milestones, features, and tasks
- [**Code Review Guide**](../guides/code-review.md) - Learn the review workflow
- [**CLI Reference**](../reference/cli.md) - Complete command documentation

## Quick Tips

> **Use Tags for Organization**
>
> Use tags to organize and filter your tasks:
>
>     jjj task new "Fix login bug" --feature F-1 --tag backend --tag auth
>     jjj task list --tag backend

> **Filter Your Board**
>
> Show only tasks in a specific column or feature:
>
>     # Show only tasks in a specific column
>     jjj task list --column "In Progress"
>
>     # Show tasks for a specific feature
>     jjj feature board F-1

> **Export for Reporting**
>
> Export all data as JSON for external tools:
>
>     jjj board --json > board-snapshot.json
>     jjj milestone list --json > milestones.json

> **Remember: Tasks Require Features**
>
> Every task must belong to a feature. If you try to create a task without `--feature`, you'll get an error. Create a feature first!
