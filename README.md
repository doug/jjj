# jjj: Distributed Project Management for Jujutsu

**jjj** is a distributed project management and code review system built for [Jujutsu (jj)](https://github.com/jj-vcs/jj). It implements Popperian epistemology — Problems, Solutions, Critiques — as a workflow that lives entirely in your repository.

No server. No database. No browser. Sync via standard `jj git push/pull`.

## Why Jujutsu?

Previous attempts at distributed review (like git-appraise) suffered a fatal flaw: **the fragility of the commit hash**. In Git, rebasing changes every commit hash, orphaning any attached metadata.

**Jujutsu solves this.** Its Change IDs persist across rebases, squashes, and history rewrites. jjj anchors metadata to change *identity*, not its momentary snapshot.

## Core Model

jjj organizes work around Karl Popper's theory of knowledge growth: bold conjectures subjected to rigorous criticism.

- **Problems** — Things that need solving. Can form hierarchies via parent/child relationships.
- **Solutions** — Conjectures to solve problems. Linked to jj Change IDs.
- **Critiques** — Error elimination. Block solution acceptance until addressed.
- **Milestones** — Time-based goals grouping problems.

## Quick Start

```bash
# Initialize jjj in your repository
jjj init

# Define a problem
jjj problem new "Search is slow" --priority high

# Propose a solution
jjj solution new "Add search index" --problem p1

# Attach your current jj change to the solution
jjj solution attach s1

# Add a critique during review
jjj critique new s1 "Missing error handling" --severity medium

# Address the critique after fixing
jjj critique address c1

# Accept the solution and mark problem solved
jjj solution accept s1
jjj problem solve p1
```

## Commands

### Workflow
```bash
jjj init                    # Initialize jjj/meta bookmark
jjj status                  # Show next actions (what to work on)
jjj ui                      # Launch interactive TUI
jjj submit                  # Squash changes and complete solution
jjj fetch                   # Fetch code and metadata from remote
jjj push                    # Push code and metadata to remote
```

### Problems
```bash
jjj problem new "Title"     # Create problem
jjj problem list            # List all problems
jjj problem show p1         # Show details
jjj problem tree            # Hierarchical view
jjj problem solve p1        # Mark solved (requires accepted solution)
jjj problem dissolve p1     # Mark dissolved (false premises)
```

### Solutions
```bash
jjj solution new "Title" --problem p1     # Create solution
jjj solution attach s1                    # Link current change
jjj solution resume s1                    # Resume working on solution
jjj solution test s1                      # Move to testing status
jjj solution accept s1                    # Accept (no open critiques)
jjj solution refute s1                    # Refute (criticism showed it won't work)
```

### Critiques
```bash
jjj critique new s1 "Issue" --severity high     # Add critique
jjj critique list --solution s1                 # List critiques
jjj critique address c1                         # Mark addressed
jjj critique dismiss c1                         # Dismiss (incorrect/irrelevant)
jjj critique validate c1                        # Validate (solution should be refuted)
```

### Milestones
```bash
jjj milestone new "Q1 Release" --date 2024-03-31
jjj milestone add-problem m1 p1
jjj milestone roadmap
```

## Architecture

### Shadow Graph

All metadata lives in an orphaned `jjj/meta` bookmark, separate from your project history:

```
.jjj/
├── config.toml
├── problems/
│   └── p1.md
├── solutions/
│   └── s1.md
├── critiques/
│   └── c1.md
└── milestones/
    └── m1.md
```

This means:
- Metadata never pollutes project history
- No merge conflicts between code and metadata
- Can be synced independently

### Syncing with Team

```bash
# Push your changes and metadata
jjj push

# Or manually:
jj git push -b jjj/meta

# Fetch updates
jjj fetch

# One-time setup: track remote metadata
jj bookmark track jjj/meta@origin
```

## Installation

```bash
# Build from source
cargo build --release

# Install
cargo install --path .

# Generate shell completions
jjj completion bash > ~/.local/share/bash-completion/completions/jjj
```

## VS Code Extension

A VS Code extension provides sidebar views for Next Actions and Project Tree:

```bash
cd vscode
npm install
npm run package
npm run install-ext
```

## Documentation

Full documentation available via mdBook:

```bash
mdbook serve
```

## License

MIT OR Apache-2.0
