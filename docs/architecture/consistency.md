---
title: Shadow Graph Consistency
description: How jjj ensures metadata stays in sync with code across branches and clones.
---

# Shadow Graph Consistency

One of the biggest challenges in distributed project management is maintaining consistency between the project metadata (problems, solutions) and the actual code branches (Jujutsu changes).

## The Synchronization Challenge

Unlike GitHub, where metadata is central and code is distributed, `jjj` distributes both. This creates several consistency requirements:
1.  **Ref-Internal Consistency**: Every `Solution` must point to a valid `ChangeId` in Jujutsu.
2.  **Concurrency**: Multiple developers must be able to add critiques or propose solutions without clobbering each other.
3.  **Branch Awareness**: When you switch branches in `jj`, `jjj status` should know which entities are relevant to your current working copy.

## How `jjj` Maintains Consistency

### 1. The Event Log as Source of Truth
`jjj` does not maintain a mutable "state" that can get corrupted. Instead, it maintains a log of immutable events. 
- If two developers create an event at the same time, they simply result in two separate files in the `.jj/.jjj/events` directory.
- `jjj sync` (push/fetch) handles the distribution of these event files using standard file-sync mechanisms, often leveraging Jujutsu’s own metadata handling.

### 2. Causality and Timestamps
Events are ordered by their timestamps and parent relationships.
- If an event "Accepts" a solution, but a concurrent event "Critiques" it, the critique "wins" in the sense that it must be addressed before the solution can be successfully accepted in a final state.
- The SQLite index is re-indexed from the combined event log to produce a consistent view.

### 3. Change ID Mapping
`jjj` stores the *Jujutsu Change ID*, not the Git Commit ID. 
- Git Commit IDs change when you rebase or amend.
- Jujutsu Change IDs are stable through rebases.
- This allows a Solution to remain attached to "logical work" even as the physical commits are manipulated by Jujutsu's advanced version control features.

### 4. Verified Transitions
Every state change (proposed $\rightarrow$ testing $\rightarrow$ accepted) is verified at the moment of the request.
- `jjj solution accept` doesn't just check the local database. 
- It scans the shadow graph for any critiques that might have been fetched recently and have not yet been addressed.
- The "accepted" state is a logical conclusion of the surviving events, not just a row in a database.
