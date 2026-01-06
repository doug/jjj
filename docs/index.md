# Welcome to jjj

**jjj** (Jujutsu Project Manager) is a distributed project management and code review system designed specifically for [Jujutsu](https://github.com/martinvonz/jj) version control.

## What is jjj?

jjj brings modern project management capabilities directly into your Jujutsu repository without requiring external servers or services. It leverages Jujutsu's unique features—particularly **change IDs**—to provide robust code review and task tracking that survives rebases, squashes, and other history rewrites.

## Key Features

### 🎯 Work Hierarchy

Organize your work with a three-level hierarchy:

- **Milestones** - Release targets and delivery dates
- **Features** - User-facing capabilities
- **Tasks** - Individual units of work
- **Bugs** - Defect tracking (can be standalone or linked)

### 💬 Code Review

Built-in code review with intelligent comment tracking:

- Comments stay attached to code through rebases
- Context-aware comment relocation using fuzzy matching
- Review status tracking (pending, approved, changes requested)
- Multi-reviewer support with @mentions

### 📊 Kanban Board

Visual task management with customizable columns:

- Interactive TUI board view
- JSON output for IDE integration
- Filter by tags, assignees, and columns
- Track task progress across features

### 🌍 Distributed & Offline-First

- No server required - metadata lives in your repository
- Works offline by default
- Sync via standard `jj git push/pull`
- Shadow graph keeps metadata separate from code history

## Why jjj?

### Built for Jujutsu

Unlike tools designed for Git, jjj embraces Jujutsu's philosophy:

- **Change IDs** provide stable references even as history evolves
- **Shadow graph** stores metadata without polluting your project history
- **Offline-first** design matches Jujutsu's distributed nature

### No External Dependencies

- No GitHub/GitLab/etc. required
- No database server to maintain
- No cloud service subscription needed
- Just Jujutsu and jjj

### Developer-Centric

- Command-line first, with IDE integration support
- JSON output for scripting and tooling
- Designed by developers, for developers
- Minimal friction, maximum productivity

## Quick Example

```bash
# Initialize jjj in your repository
jjj init

# Create a milestone for your next release
jjj milestone new "v1.0 Release" --date 2025-12-31

# Create a feature
jjj feature new "User Authentication" --milestone M-1 --priority high

# Break it into tasks
jjj task new "Implement password hashing" --feature F-1
jjj task new "Add login API" --feature F-1
jjj task new "Create login UI" --feature F-1

# Start working on a task
jjj task attach T-1
# ... make changes ...
jjj task move T-1 "In Progress"

# Request code review
jjj review request @alice @bob

# View your board
jjj board

# Track feature progress
jjj feature progress F-1
```

## Getting Started

Ready to dive in? Check out our guides:

- [**Installation**](getting-started/installation.md) - Get jjj installed
- [**Quick Start**](getting-started/quick-start.md) - Your first jjj project
- [**Work Hierarchy Guide**](guides/work-hierarchy.md) - Understanding milestones, features, and tasks
- [**Code Review Guide**](guides/code-review.md) - How code review works in jjj

## Architecture Highlights

### Change ID Stability

Unlike Git commit hashes, Jujutsu's **change IDs** remain stable across rebases and history rewrites. This makes them perfect for attaching metadata like:

- Task associations
- Code review comments
- Bug fix references

### Shadow Graph

jjj stores all metadata in a **shadow graph**—an orphaned commit history separate from your project. This means:

- ✅ Metadata never pollutes your project history
- ✅ Can be pushed/pulled independently
- ✅ No merge conflicts with code changes
- ✅ Easy to reset if needed

### Context Fingerprinting

Code review comments use **SHA-256 hashing** of surrounding code context to intelligently relocate when files change. When you rebase or edit code:

1. Exact match: Comment stays at same line (fast path)
2. Fuzzy match: Uses similarity scoring to find new location
3. Orphaned: Comment marked as unresolved if context disappears

## Community & Support

- **Documentation**: You're reading it! 📖
- **Issues**: [GitHub Issues](https://github.com/doug/jjj/issues)
- **Discussions**: [GitHub Discussions](https://github.com/doug/jjj/discussions)

## License

jjj is open source software licensed under the MIT License.

---

**Ready to get started?** → [Installation Guide](getting-started/installation.md)
