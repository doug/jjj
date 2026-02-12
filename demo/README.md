# jjj Demo Environment

This directory contains scripts to set up and run a demo of **jjj** (Jujutsu Project Manager).

## Prerequisites

1. **Jujutsu (jj)** must be installed
   - Install from: https://github.com/martinvonz/jj
   - Or via Homebrew: `brew install jj`

2. **Build jjj**
   ```bash
   cd ..
   cargo build --release
   ```

## Quick Start

### 1. Set Up the Demo Repository

```bash
cd demo
./setup.sh
```

This script will:
- Create a new jj repository in `demo-repo/`
- Initialize it with sample source files
- Create several commits with different changes
- Initialize jjj in the repository
- Prepare the environment for testing

### 2. Run the Interactive Demo

```bash
cd demo-repo
../demo-commands.sh
```

This interactive script will walk you through:
- Creating and managing tasks
- Using the Kanban board
- Requesting code reviews
- Adding review comments
- Tracking work with the dashboard
- Moving tasks through the workflow

## Manual Exploration

After running `setup.sh`, you can explore jjj manually:

```bash
cd demo-repo

# View your Kanban board
../target/release/jjj board

# Create a new task
../target/release/jjj task new "My new feature" --tag backend

# List all tasks
../target/release/jjj task list

# Request a code review
../target/release/jjj review request alice bob

# View your dashboard
../target/release/jjj dashboard

# Get help
../target/release/jjj --help
../target/release/jjj task --help
../target/release/jjj review --help
```

## Demo Scenarios

### Scenario 1: Task Management Workflow

```bash
cd demo-repo

# Create a task
../target/release/jjj task new "Implement feature X" --tag backend --tag api

# Attach current change to the task
../target/release/jjj task attach T-1

# Move task through the workflow
../target/release/jjj task move T-1 "In Progress"
# ... do some work ...
../target/release/jjj task move T-1 "Review"
# ... get approval ...
../target/release/jjj task move T-1 "Done"

# View progress
../target/release/jjj board
```

### Scenario 2: Code Review Workflow

```bash
cd demo-repo

# Make some changes
echo "fn new_feature() {}" >> src/main.rs
jj describe -m "Add new feature"

# Request review
../target/release/jjj review request alice

# Simulate reviewer adding comments
CHANGE_ID=$(jj log -r @ -T change_id --no-graph)
../target/release/jjj review comment "$CHANGE_ID" --body "Looks great!"

# Approve the change
../target/release/jjj review approve "$CHANGE_ID"

# Check review status
../target/release/jjj review status "$CHANGE_ID"
```

### Scenario 3: Team Collaboration

```bash
cd demo-repo

# Create tasks for team members
../target/release/jjj task new "API endpoints" --tag backend
../target/release/jjj task new "Frontend UI" --tag frontend
../target/release/jjj task new "Database migrations" --tag database

# View the board
../target/release/jjj board

# Filter by tag
../target/release/jjj task list --tag backend

# View team dashboard
../target/release/jjj dashboard
```

## Understanding the Demo

### Repository Structure

After running `setup.sh`, the demo repository contains:

```
demo-repo/
├── .jj/                    # Jujutsu working copy state
├── .git/                   # Git storage (colocated)
├── README.md               # Demo project readme
└── src/
    ├── main.rs            # Main application
    ├── auth.rs            # Authentication module
    ├── api.rs             # API endpoints
    └── db.rs              # Database layer
```

### Metadata Location

All jjj metadata lives in the `jjj` bookmark:
- Tasks: `tasks/T-*.json`
- Reviews: `reviews/<change-id>/manifest.toml`
- Comments: `reviews/<change-id>/comments/c-*.json`
- Configuration: `config.toml`

To view the metadata:

```bash
jj bookmark list  # See jjj bookmark
jj log -r 'jjj'  # View metadata history
```

### Demo Data

The setup script creates:
- 3 commits with different features
- Sample source files in `src/`
- A configured jj repository
- Initialized jjj metadata structure

## Cleanup

To reset the demo and start fresh:

```bash
cd demo
rm -rf demo-repo
./setup.sh
```

## Troubleshooting

### "jj: command not found"

Install Jujutsu:
```bash
# macOS
brew install jj

# From source
cargo install --git https://github.com/martinvonz/jj jj-cli
```

### "jjj binary not found"

Build jjj first:
```bash
cd ..
cargo build --release
```

### "Not in a jj repository"

Make sure you're in the `demo-repo` directory:
```bash
cd demo-repo
```

### Demo script fails

Ensure you ran `setup.sh` first:
```bash
./setup.sh
```

## Next Steps

After exploring the demo:

1. **Read the documentation**
   - [FEATURES.md](../FEATURES.md) - Detailed feature overview
   - [README.md](../README.md) - Project overview

2. **Run the tests**
   ```bash
   cd ..
   cargo test
   ```

3. **Try in your own repository**
   ```bash
   cd /path/to/your/jj/repo
   /path/to/jjj/target/release/jjj init
   /path/to/jjj/target/release/jjj board
   ```

4. **Contribute**
   - Check the issues on GitHub
   - Suggest new features
   - Submit pull requests

## Learning Resources

- **Jujutsu Documentation**: https://github.com/martinvonz/jj/tree/main/docs
- **jjj Architecture**: See [FEATURES.md](../FEATURES.md)
- **Test Examples**: See `tests/` directory for usage examples

## Demo Tips

- Use `jj log` to see your change history
- Use `jj diff` to see what changed in each commit
- The `jjj` bookmark syncs with `jj push/pull`
- All jjj operations work offline
- Task IDs and Change IDs are stable across rebases

Enjoy exploring jjj! 🚀
