# jjj: Distributed Project Management for Jujutsu

**jjj** is a distributed project management and code review system built for [Jujutsu (jj)](https://github.com/jj-vcs/jj). It implements Popperian epistemology — Problems, Solutions, Critiques — as a workflow that lives entirely in your repository.

No server. No database. No browser. Sync via standard `jj git push/pull`.

**[Documentation → jjj.recursivewhy.com](https://jjj.recursivewhy.com)**

![jjj workflow demo](demo/workflow.gif)

## Why Jujutsu?

Previous attempts at distributed review (like git-appraise) suffered a fatal flaw: **the fragility of the commit hash**. In Git, rebasing changes every commit hash, orphaning any attached metadata.

**Jujutsu solves this.** Its Change IDs persist across rebases, squashes, and history rewrites. jjj anchors metadata to change *identity*, not its momentary snapshot.

## Core Model

jjj organizes work around Karl Popper's theory of knowledge growth: bold conjectures subjected to rigorous criticism.

- **Problems** — Things that need solving. Can form hierarchies via parent/child relationships.
- **Solutions** — Conjectures to solve problems. Linked to jj Change IDs.
- **Critiques** — Error elimination. Block solution approval until addressed.
- **Milestones** — Time-based goals grouping problems.

## Quick Start

```bash
# Initialize jjj in your repository
jjj init

# Define a problem
jjj problem new "Search is slow" --priority high

# Propose a solution (references problem by title)
jjj solution new "Add search index" --problem "Search is slow"

# Attach your current jj change to the solution
jjj solution attach "search index"

# Add a critique during review
jjj critique new "search index" "Missing error handling" --severity medium

# Address the critique after fixing
jjj critique address "Missing error"

# Submit for review, then approve when critiques are resolved
jjj solution submit "search index"
jjj solution approve "search index"
jjj problem solve "Search is slow"
```

## Interactive TUI

Launch `jjj ui` for a full terminal interface with project tree, detail pane, and keyboard-driven actions.

![jjj TUI demo](demo/tui.gif)

## Commands

### Workflow
```bash
jjj init                    # Initialize jjj bookmark
jjj status                  # Show next actions (what to work on)
jjj next                    # Top next actions (--top N, --mine, --json)
jjj next --claim            # Claim the top item (assign to yourself)
jjj overlaps                # Detect files touched by multiple solutions
jjj insights                # Show project statistics (approval rate, cycle times)
jjj ui                      # Launch interactive TUI
jjj fetch                   # Fetch code and metadata from remote
jjj push                    # Push code and metadata to remote
jjj github push             # Refresh PR bodies and sync issue state
```

### Problems
```bash
jjj problem new "Title"                # Create problem
jjj problem list                       # List all problems
jjj problem show "Search is slow"      # Show details (by title)
jjj problem show 01957d                # Show details (by UUID prefix)
jjj problem tree                       # Hierarchical view
jjj problem solve "Search is slow"     # Mark solved (requires approved solution)
jjj problem dissolve "Search"          # Mark dissolved (false premises)
jjj problem reopen "Search"           # Reopen a solved/dissolved problem
jjj problem duplicate "Search" "Other" # Mark problem as duplicate
```

### Solutions
```bash
jjj solution new "Title" --problem "Search"       # Create solution
jjj solution attach "search index"                 # Link current change
jjj solution resume "search index"                 # Resume working on solution
jjj solution submit "search index"                 # Submit for review
jjj solution approve "search index"                # Approve (no open critiques)
jjj solution withdraw "search index"               # Withdraw (criticism showed it won't work)
jjj solution lgtm "search index"                   # Sign off as reviewer (LGTM)
jjj solution comment "search index" --critique ID "reply"  # Reply to a critique
```

### Critiques
```bash
jjj critique new "search index" "Issue" --severity high  # Add critique
jjj critique list --solution "search index"              # List critiques
jjj critique address "Missing error"                     # Mark addressed
jjj critique dismiss "Missing error"                     # Dismiss (incorrect/irrelevant)
jjj critique validate "Missing error"                    # Validate (solution should be withdrawn)
```

### Milestones
```bash
jjj milestone new "Q1 Release" --date 2025-03-31
jjj milestone add-problem "Q1 Release" "Search is slow"
jjj milestone roadmap
```

## Architecture

### Shadow Graph

All metadata lives in an orphaned `jjj` bookmark, separate from your project history:

```
config.toml
problems/
  01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a.md
solutions/
  01958a1b-c3d4-7e5f-9a0b-1c2d3e4f5a6b.md
critiques/
  01959b2c-d4e5-7f6a-0b1c-2d3e4f5a6b7c.md
milestones/
  01960c3d-e5f6-7a0b-1c2d-3e4f5a6b7c8d.md
events.jsonl
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
jj git push -b jjj

# Fetch updates
jjj fetch

# One-time setup: track remote metadata
jj bookmark track jjj@origin
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

## AI Agent Integration

jjj ships a skill file that teaches Claude Code, Gemini CLI, or any AI coding assistant to use jjj commands natively.

```bash
# Claude Code
mkdir -p ~/.claude/skills/jjj && \
  curl -fsSL https://jjj.recursivewhy.com/SKILL.md \
    -o ~/.claude/skills/jjj/SKILL.md

# Gemini CLI
mkdir -p ~/.gemini/skills/jjj && \
  curl -fsSL https://jjj.recursivewhy.com/SKILL.md \
    -o ~/.gemini/skills/jjj/SKILL.md
```

Once installed, invoke with `/jjj` or let the agent detect it automatically. See the [AI Agents guide](https://jjj.recursivewhy.com/guides/ai-agents/) for details.

## Documentation

Full documentation available at [jjj.recursivewhy.com](https://jjj.recursivewhy.com), or serve locally:

```bash
cd docs-site
npm install
npm run dev
```

## Note for Previous `jjj` Crate Users

This crate previously hosted a modal interface for Jujutsu by [@icorbrey](https://github.com/icorbrey). That project has been renamed and is now available as [**megamerge**](https://crates.io/crates/megamerge).

## License

Apache-2.0
