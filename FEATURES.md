# Features & Motivations

## Overview

**jjj** (Jujutsu Project Manager) is a distributed project management and code review system built exclusively for the Jujutsu version control system. It solves the fundamental problem that plagued previous distributed review systems like git-appraise: **the fragility of commit hashes**.

## Core Innovation: Change ID Stability

### The Problem with Git-based Systems

In Git, every rebase or history rewrite changes commit hashes. When you attach metadata (reviews, comments, tasks) to a commit hash, that metadata becomes orphaned the moment you clean up your history. Previous systems tried to solve this with complex heuristics to "re-attach" metadata, but this was fragile and error-prone.

### The Jujutsu Advantage

Jujutsu treats **changes as first-class citizens** with stable Change IDs that persist across:
- Rebases
- Squashes
- Amendments
- Description edits
- Any other history rewrites

This stability means jjj can anchor all metadata to the *identity* of a change, not its momentary snapshot.

## Architecture

### The Shadow Graph

All metadata lives in an **orphaned history root** tracked by the `jjj/meta` bookmark. This design ensures:

1. **Separation of Concerns**: Metadata never pollutes your working copy
2. **Distributed Sync**: Push/pull `jjj/meta` just like any other bookmark
3. **Offline Capability**: All operations work without a server
4. **Conflict Resolution**: Uses standard jj conflict resolution for competing updates

### Storage Layout

```
jjj/meta (bookmark)
├── config.toml              # Project configuration
├── tasks/
│   ├── T-1024.json         # Task metadata (JSON for machines)
│   └── T-1025.json
└── reviews/
    └── kpzszn.../          # Directory named by Change ID
        ├── manifest.toml   # Review status (TOML for humans)
        └── comments/
            └── c-998.json  # Individual comment objects
```

**Design Choices**:
- **JSON** for machine-readable data that changes frequently (tasks, comments)
- **TOML** for human-editable configuration and review status
- **Change ID directories** for natural grouping of review data

## Feature Set

### 1. Kanban Board (`jjj board`)

**Motivation**: Developers shouldn't need to context-switch to a browser to track work.

**Features**:
- Terminal-based Kanban board
- Configurable columns (default: TODO, In Progress, Review, Done)
- Tag-based categorization
- Assignee tracking
- Change ID associations
- Comment indicators

**Example Output**:
```
┌─ TODO (2)
│
│  T-101 - Database Schema Migration
│    Tags: #backend
│
│  T-102 - Update API Documentation
│    Tags: #docs
│
└─

┌─ In Progress (1)
│
│  T-105 - Auth API Implementation
│    @james
│    Changes: yqosq...
│
└─
```

### 2. Task Management (`jjj task`)

**Motivation**: Work items should be as version-controlled as the code itself.

**Commands**:
- `new` - Create tasks with tags and initial columns
- `list` - Filter by column, tag, or assignee
- `show` - View detailed task information
- `attach` - Link the current change to a task
- `detach` - Unlink a change from a task
- `move` - Move tasks between columns
- `edit` - Update task title and tags
- `delete` - Remove tasks (with confirmation)

**Key Features**:
- Multiple changes can be attached to a single task (for multi-commit features)
- Tasks track their version for conflict detection
- Timestamps for creation and updates

### 3. Code Review (`jjj review`)

**Motivation**: Enable asynchronous, distributed code review without GitHub/GitLab.

**Commands**:
- `request` - Request review from team members
- `list` - View pending reviews
- `start` - Begin reviewing a change (shows diff)
- `comment` - Add inline or general comments
- `status` - Check review state and all comments
- `approve` - Approve a change
- `request-changes` - Request modifications

**Innovative Features**:

#### Context Fingerprinting
When you comment on line 42 of `src/auth.rs`, jjj stores:
- The exact line number
- A hash of surrounding context
- The actual context lines

If the author rebases and line 42 moves to line 50, jjj uses fuzzy matching to **relocate the comment automatically**.

**Implementation Details**:
```rust
pub struct CommentLocation {
    pub start_line: usize,
    pub end_line: usize,
    pub context_hash: String,
    pub context_lines: Vec<String>,
}
```

The `try_relocate()` method:
1. Tries exact line number match first
2. Falls back to fuzzy matching using similarity scoring
3. Accepts matches above 70% similarity threshold

#### Review Status Tracking
Reviews progress through states:
- **Pending**: Awaiting reviewer action
- **Approved**: Change is ready to merge
- **ChangesRequested**: Author needs to address feedback
- **Dismissed**: Review cancelled/obsolete

### 4. Dashboard (`jjj dashboard`)

**Motivation**: Provide a personalized view of work and responsibilities.

**Shows**:
- Tasks assigned to you
- Reviews requesting your input
- Your submitted reviews and their status

**Smart Filtering**:
- Matches user identity from `jj config`
- Groups by relevance
- Shows relative timestamps

### 5. Conflict Resolution (`jjj resolve`)

**Motivation**: Handle concurrent metadata updates gracefully.

**Scenario**: Alice moves task T-105 to "Done" while Bob moves it to "Blocked".

**Resolution**:
1. jj detects conflict in `tasks/T-105.json`
2. `jjj board` renders the task with a conflict indicator
3. `jjj resolve T-105 --pick "Done"` chooses Alice's version
4. Standard jj merge tools can also be used

**Note**: Full implementation pending; currently shows guidance to use `jj resolve`.

## Technical Implementation Highlights

### 1. Error Handling

Custom error types with `thiserror`:
```rust
pub enum JjjError {
    JjExecution(String),
    JjNotFound,
    NotInRepository,
    TaskNotFound(String),
    ReviewNotFound(String),
    // ... more variants
}
```

All operations return `Result<T, JjjError>` for comprehensive error handling.

### 2. JJ Integration

The `JjClient` wraps jj commands:
- Auto-discovers jj executable and repository root
- Provides type-safe operations (get change ID, show diff, etc.)
- Handles command execution and error parsing

### 3. Storage Abstraction

The `MetadataStore`:
- Manages the `jjj/meta` bookmark lifecycle
- Ensures metadata directory is checked out before operations
- Handles serialization/deserialization
- Generates sequential IDs for tasks and comments

### 4. CLI Design

Built with `clap` derive macros for:
- Automatic help generation
- Type-safe argument parsing
- Subcommand routing
- Optional parameter handling

## Performance Characteristics

- **Fast**: All operations are local filesystem reads/writes
- **Scalable**: Linear time complexity for most operations
- **Efficient**: Only loads data as needed (lazy evaluation)
- **Lightweight**: No database, no background processes

## Future Enhancements

### Planned Features

1. **Interactive TUI** (using ratatui)
   - Drag-and-drop task cards
   - Keyboard navigation
   - Inline diff viewer
   - Comment thread expansion

2. **Advanced Conflict Resolution**
   - Custom merge strategies for metadata
   - Three-way merge visualization
   - Automated conflict detection

3. **Review Analytics**
   - Review response times
   - Approval rates
   - Comment density heatmaps

4. **GitHub/GitLab Integration**
   - Optional bridge for legacy compatibility
   - Sync review status to PRs
   - Mirror comments bi-directionally

5. **Notification System**
   - Watch specific tasks or reviews
   - Digest emails for pending work
   - Hook system for custom integrations

6. **Stacked Diff Support**
   - Review entire change stacks
   - Track dependencies between changes
   - Stack-wide approval workflows

7. **Enhanced Search**
   - Full-text search across tasks and comments
   - Filter by date ranges
   - Saved search queries

8. **CI/CD Integration**
   - Approval gates for deployments
   - Test result annotations on reviews
   - Status checks for required approvals

## Philosophy & Design Principles

### 1. **Offline First**
Every operation works without network connectivity. Synchronization is explicit (via `jj push/pull`).

### 2. **Data as Code**
Project management metadata is version-controlled alongside the code it describes.

### 3. **Developer-Centric**
Terminal-first interface for minimal context switching. Browser optional, not required.

### 4. **Conflict-Aware**
Concurrent edits are expected and handled gracefully through jj's conflict resolution.

### 5. **Change-Centric**
Everything revolves around Jujutsu's stable Change IDs, not fragile commit hashes.

### 6. **Simple Storage**
Human-readable files (JSON/TOML) that can be manually edited if needed.

### 7. **No Magic**
Operations are explicit and predictable. No background processes or hidden state.

## Use Cases

### Solo Developer
- Track TODOs directly in the repository
- Organize work with Kanban columns
- Self-review with structured comments

### Small Team (2-10 developers)
- Distributed code review without GitHub
- Offline-capable workflow
- Conflict-free concurrent task updates

### Open Source Projects
- Contributors can review locally
- Maintainers control review data (no vendor lock-in)
- Review history preserved in repository

### Research/Academic
- Review iterations tracked alongside experiments
- Reproducible review process
- Long-term archival in version control

## Comparison with Alternatives

| Feature | jjj | GitHub PR | git-appraise | Gerrit |
|---------|-----|-----------|--------------|--------|
| Offline | ✓ | ✗ | ✓ | ✗ |
| Distributed | ✓ | ✗ | ✓ | ✗ |
| Change ID Stable | ✓ | ✗ | ✗ | ✓ |
| Kanban Board | ✓ | Limited | ✗ | ✗ |
| Comment Relocation | ✓ | ✗ | ✗ | ✗ |
| Server Required | ✗ | ✓ | ✗ | ✓ |
| VCS Specific | jj only | git | git | git |

## Conclusion

**jjj** leverages Jujutsu's unique properties to create a project management system that is:
- **Resilient**: Metadata survives history rewrites
- **Distributed**: No central server required
- **Integrated**: Lives in your repository
- **Powerful**: Full review and task management
- **Simple**: Plain text files and standard tools

It represents a new paradigm where project management is as distributed, version-controlled, and resilient as the code itself.
