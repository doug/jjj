# RFC-001: Entity-Based Review Workflow

## Summary

Enable users to interact with reviews using **Entity IDs** (Features `F-*`, Tasks `T-*`, Bugs `B-*`) instead of raw **Change IDs** (e.g., `kpqxywon`). This aligns the review workflow with the project management hierarchy and reduces cognitive load.

## Motivation

Currently, the review workflow requires copying and pasting Change IDs:

```bash
jjj review request kpqxywon @alice
jjj review start kpqxywon
```

Change IDs are random strings. Users naturally think in terms of the work item they are addressing:
*   "I'm reviewing the login task" (`T-101`)
*   "I'm fixing the crash bug" (`B-502`)
*   "I'm checking the auth feature" (`F-1`)

## Proposal

### 1. Unified Entity Attachment

Ensure all entities can be linked to changes.

*   **Tasks**: `jjj task attach T-1` (Existing)
*   **Features**: `jjj feature attach F-1` (New)
*   **Bugs**: `jjj bug attach B-1` (New)

### 2. Entity-Based Commands

Update `jjj review` commands to accept any Entity ID as an alias for the associated Change ID.

**Requesting Review:**
```bash
jjj review request T-101 @alice  # Resolves to change attached to T-101
jjj review request B-502 @bob    # Resolves to change attached to B-502
```

**Starting Review:**
```bash
jjj review start F-1             # Resolves to change attached to F-1
```

### 3. Bookmark Naming Convention

Encourage naming bookmarks with the Entity ID:

*   `task/T-101-login-schema`
*   `bug/B-502-crash-fix`
*   `feature/F-1-auth`

## User Experience Improvements

| Action | Current Flow | Proposed Flow |
|--------|--------------|---------------|
| **Identify Work** | Copy `kpqxywon` | Use `T-101` / `B-502` |
| **Context** | "Reviewing change `kpqxywon`" | "Reviewing Task `T-101`: Login Schema" |
| **Discovery** | Must ask author for ID | Can look up on Kanban board |

## Implementation Plan

### 1. Schema Updates

Ensure all entities track their latest change.

**Feature (`src/storage.rs`):**
```rust
pub struct Feature {
    // ...
    pub latest_change_id: Option<String>,
}
```

**Bug (`src/storage.rs`):**
```rust
pub struct Bug {
    // ...
    pub latest_change_id: Option<String>,
}
```

**Task (`src/storage.rs`):**
*   Tasks already support attachment. Ensure `latest_change_id` or `associated_changes` list is efficiently queryable.

### 2. CLI Command Updates

Modify `src/commands/review.rs` to handle `F-*`, `T-*`, and `B-*` arguments.

**Resolution Logic:**

1.  Parse prefix (`F-`, `T-`, `B-`).
2.  Load corresponding metadata file.
3.  Retrieve `latest_change_id`.
4.  If multiple changes attached (e.g., to a Task), prompt user or default to most recent.
5.  Execute review command against the resolved Change ID.

### 3. New Commands

*   `jjj feature attach <ID>`
*   `jjj bug attach <ID>`
