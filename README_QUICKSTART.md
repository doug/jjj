# Quick Start Guide

Get started with **jjj** in 5 minutes!

## Prerequisites

1. **Install Jujutsu**
   ```bash
   # macOS
   brew install jj

   # Or from source
   cargo install --git https://github.com/martinvonz/jj jj-cli
   ```

2. **Build jjj**
   ```bash
   git clone https://github.com/yourusername/jjj
   cd jjj
   cargo build --release
   ```

## Try the Demo

The fastest way to see jjj in action:

```bash
cd demo
./setup.sh
cd demo-repo
../demo-commands.sh
```

This runs an interactive demo showcasing all features.

## Use in Your Project

### 1. Initialize

```bash
cd /path/to/your/jj/repo
jjj init
```

### 2. Create Your First Task

```bash
jjj task new "Implement user login" --tag backend
```

### 3. View the Board

```bash
jjj board
```

Output:
```
┌─ TODO (1)
│
│  T-1 - Implement user login
│    Tags: #backend
│
└─
```

### 4. Attach Work to the Task

```bash
# Make some changes
echo "fn login() {}" >> src/auth.rs
jj describe -m "Add login function"

# Attach current change to task
jjj task attach T-1

# Move to In Progress
jjj task move T-1 "In Progress"
```

### 5. Request a Review

```bash
jjj review request alice bob
```

### 6. Check Your Dashboard

```bash
jjj dashboard
```

## Common Workflows

### Task Management

```bash
# Create task
jjj task new "Fix bug #123" --tag bugfix

# List tasks
jjj task list

# Filter by tag
jjj task list --tag backend

# Move task
jjj task move T-2 "Done"

# View details
jjj task show T-1
```

### Code Review

```bash
# Request review
jjj review request alice

# List reviews
jjj review list

# Add comment
jjj review comment <change-id> --body "Looks good!"

# Approve
jjj review approve <change-id>

# Check status
jjj review status <change-id>
```

## Next Steps

- **Read the full README**: [README.md](README.md)
- **Explore features**: [FEATURES.md](FEATURES.md)
- **Run tests**: `cargo test`
- **Try the demo**: `demo/setup.sh`

## Documentation

- [README.md](README.md) - Full project overview
- [FEATURES.md](FEATURES.md) - Detailed feature descriptions
- [TESTING.md](TESTING.md) - Testing documentation
- [PROJECT_STATUS.md](PROJECT_STATUS.md) - Implementation status
- [demo/README.md](demo/README.md) - Demo instructions

## Getting Help

```bash
# General help
jjj --help

# Command-specific help
jjj task --help
jjj review --help
```

## Tips

1. **Use short Change IDs**: jjj shows abbreviated IDs like `kpqxy...`
2. **Tags are powerful**: Use them to categorize and filter work
3. **Board is your overview**: Run `jjj board` frequently
4. **Dashboard is personal**: Shows only your work
5. **Everything is offline**: No server needed!

Enjoy using jjj! 🚀
