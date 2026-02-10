# Welcome to jjj

**jjj** is distributed project management for [Jujutsu](https://github.com/martinvonz/jj), built on Popperian epistemology.

All project metadata lives in your repository. No server, no cloud service, no external database. Sync via standard `jj git push/pull`.

## Core Model

jjj organizes work around Karl Popper's theory of knowledge growth: we make progress by proposing bold conjectures and subjecting them to rigorous criticism.

### Problems

Things that need solving. Problems can be decomposed into sub-problems to break down complexity.

### Solutions

Conjectures -- tentative attempts to solve a problem. A solution proposes a specific approach and is linked to the problem it addresses.

### Critiques

Error elimination. A critique identifies flaws in a solution, blocking or refining it until the issues are addressed.

This cycle -- problem, conjecture, criticism -- drives the project forward through iterative refinement rather than upfront specification.

## Quick Example

Initialize a project and define a problem with a proposed solution:

```bash,test
jjj init
jjj problem new "Search is slow" --priority P1
jjj solution new "Add search index" --problem p1
```

Then work through the critique cycle:

```bash
jjj solution resume s1
jjj critique new s1 "Missing error handling" --severity medium
jjj critique address c1
jjj submit
```

## Key Features

- **Offline-first** -- metadata lives in the repository, no server required
- **Change ID stability** -- Jujutsu change IDs survive rebases and history rewrites, so references never break
- **Critique-driven review** -- solutions are refined through structured criticism, not ad-hoc comments
- **`jjj status` guided workflow** -- always know what to work on next
- **Priority-based triage** -- focus on what matters with P0-P3 priority levels

## Getting Started

- [Installation](getting-started/installation.md) -- get jjj installed
- [Quick Start](getting-started/quick-start.md) -- your first jjj project

## Reference

- [Problem Commands](reference/cli-problem.md) -- problem management
- [Solution Commands](reference/cli-solution.md) -- solution lifecycle
- [Critique Commands](reference/cli-critique.md) -- critique operations
- [Workflow Commands](reference/cli-workflow.md) -- init, status, submit, push, fetch, ui
- [Configuration](reference/configuration.md) -- project and user settings

## Architecture Highlights

### Change ID Stability

Unlike Git commit hashes, Jujutsu's **change IDs** remain stable across rebases and history rewrites. This makes them ideal for attaching persistent metadata:

- Problem, solution, and critique associations
- Workflow state transitions
- Cross-reference links between items

### Shadow Graph

jjj stores all metadata in a **shadow graph** -- an orphaned commit history separate from your project. This means:

- Metadata never pollutes your project history
- Can be pushed and pulled independently
- No merge conflicts with code changes
- Easy to reset if needed

## Architecture

- [Design Philosophy](architecture/design-philosophy.md)
- [Storage and Metadata](architecture/storage.md)
- [Change ID Tracking](architecture/change-tracking.md)

## License

jjj is open source software licensed under the MIT License.
