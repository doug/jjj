# Storage & Metadata

jjj stores all project management metadata in a **shadow graph**—a separate, orphaned commit history in your Jujutsu repository.

## The Shadow Graph

### What is it?

A shadow graph is an orphaned commit history that exists in your repository but is completely separate from your project code:

```
Project History          Shadow Graph (jjj/meta)
─────────────────        ───────────────────────
main                     jjj/meta
 ◯ Feature C              ◯ Update metadata
 │                        │
 ◯ Feature B              ◯ Add tasks
 │                        │
 ◯ Feature A              ◯ Initialize jjj
 │                        │
 ◯ Initial commit         ◯ (orphaned root)
```

These histories never merge. They coexist peacefully in the same repository.

### Why Use a Shadow Graph?

Traditional approaches to storing metadata have problems:

#### ❌ Polluting Project History

```
# Bad: Metadata mixed with code
◯ Add user authentication
│
◯ jjj: Update task T-1 status   ← Noise!
│
◯ Fix login bug
│
◯ jjj: Create feature F-1        ← More noise!
```

This clutters `git log` and makes project history messy.

#### ❌ Separate Git Repository

```
project/          # Main code
project-meta/     # Metadata in separate repo
```

Problems:
- Have to sync two repositories
- Lose atomic operations
- Complex deployment

#### ✅ Shadow Graph (jjj's Approach)

```
# Same repo, separate histories
jj log -r main                    # Clean project history
jj log -r jjj/meta                # Metadata history

jj git push --all                 # Push both at once
```

Benefits:
- ✅ One repository to manage
- ✅ Atomic push/pull of code + metadata
- ✅ Clean project history
- ✅ Easy to reset or delete metadata

## File Structure

When you run `jjj init`, it creates this structure:

```
.jj/
└── jjj-meta/                    # Metadata workspace
    ├── config.toml              # Project configuration
    ├── milestones/              # Milestone storage
    │   ├── M-1.json
    │   └── M-2.json
    ├── features/                # Feature storage
    │   ├── F-1.json
    │   ├── F-2.json
    │   └── F-3.json
    ├── tasks/                   # Task storage
    │   ├── T-1.json
    │   ├── T-2.json
    │   └── ...
    ├── bugs/                    # Bug storage
    │   ├── B-1.json
    │   └── B-2.json
    └── reviews/                 # Review storage
        ├── kpqxywon.../
        │   ├── manifest.toml
        │   └── comments/
        │       ├── c-1.json
        │       └── c-2.json
        └── lmnopqrs.../
```

## Storage Layer Implementation

### MetadataStore

The `MetadataStore` struct manages all metadata operations:

```rust
pub struct MetadataStore {
    meta_path: PathBuf,          // Path to .jj/jjj-meta
    jj_client: JjClient,         // Main repo client
    meta_client: JjClient,       // Metadata workspace client
}
```

### Initialization

When you run `jjj init`:

1. **Create orphaned root**:
   ```bash
   jj new --no-parent -m "Initialize jjj metadata"
   ```

2. **Create bookmark**:
   ```bash
   jj bookmark create jjj/meta
   ```

3. **Create workspace**:
   ```bash
   jj workspace add .jj/jjj-meta -r jjj/meta
   ```

4. **Initialize directories**:
   ```
   mkdir -p .jj/jjj-meta/{tasks,features,milestones,bugs,reviews}
   ```

5. **Create default config**:
   ```toml
   # .jj/jjj-meta/config.toml
   [board]
   columns = ["TODO", "In Progress", "Review", "Done"]

   [tags]
   allowed = ["backend", "frontend", "docs", "tests"]
   ```

### File Formats

#### TOML for Configuration

```toml
# config.toml
[board]
columns = ["TODO", "In Progress", "Review", "Done"]

[tags]
allowed = ["backend", "frontend", "api", "ui"]

[review]
required_approvals = 1
auto_approve_owner = false
```

#### JSON for Work Items

Tasks, features, milestones, and bugs use JSON:

```json
// tasks/T-1.json
{
  "id": "T-1",
  "title": "Implement password hashing",
  "feature_id": "F-1",
  "column": "In Progress",
  "tags": ["backend", "security"],
  "change_ids": ["kpqxywon"],
  "assignee": "alice@example.com",
  "version": 3,
  "created_at": "2025-11-23T10:00:00Z",
  "updated_at": "2025-11-23T15:30:00Z"
}
```

Why JSON?
- ✅ Easy to parse
- ✅ Human-readable
- ✅ Standard tooling (jq, etc.)
- ✅ Schemaless (can evolve)

#### TOML for Reviews

Review manifests use TOML for better readability:

```toml
# reviews/kpqxywon.../manifest.toml
change_id = "kpqxywon"
author = "alice@example.com"
reviewers = ["@bob", "@charlie"]
status = "Pending"
created_at = "2025-11-23T10:00:00Z"

[[requested_reviewers]]
name = "@bob"
status = "Pending"

[[requested_reviewers]]
name = "@charlie"
status = "Approved"
approved_at = "2025-11-23T14:00:00Z"
```

## Transaction Model

### Atomic Updates

jjj uses a simple transaction model:

```rust
store.with_metadata("Create task T-1", || {
    // 1. Perform operations
    let task = Task::new(...);
    store.save_task(&task)?;

    // 2. All operations succeed or all fail
    Ok(())
})?;
// 3. Metadata committed atomically
```

This translates to:

```bash
# In metadata workspace
jj new -m "Create task T-1"
# ... write files ...
jj bookmark set jjj/meta
```

### Conflict Resolution

If two users modify metadata simultaneously:

```
User A                          User B
──────                          ──────
jjj task new "Fix bug"          jjj feature new "New feature"
  ↓                               ↓
Creates T-42                    Creates F-5
  ↓                               ↓
jj git push                     jj git push
  ↓                               ↓
  └─────── CONFLICT! ───────────┘
```

Resolution:

```bash
# Pull and resolve
jj git fetch
jj bookmark track jjj/meta@origin

# jj automatically merges file-based changes
# If both created different files → no conflict!

# If same file modified → use jjj resolve
jjj resolve T-42 --pick "User A"
```

## Sync Model

### Push

```bash
# Push metadata bookmark
jj git push --bookmark jjj/meta

# Or push all bookmarks
jj git push --all
```

What gets pushed:
- All metadata commits
- Shadow graph history
- Configuration changes

### Pull

```bash
# Fetch metadata
jj git fetch

# Track remote bookmark
jj bookmark track jjj/meta@origin

# Metadata automatically merged
```

### Working Offline

jjj is designed for offline-first workflows:

```bash
# Create tasks offline
jjj task new "Fix login" --feature F-1
jjj task new "Add tests" --feature F-1

# Work on tasks
jjj task attach T-1
# ... make changes ...

# Later, when online
jj git push --all
```

All metadata is local until you push!

## Performance

### ID Generation

IDs are sequential within each type:

```rust
pub fn next_task_id(&self) -> Result<String> {
    let tasks = self.list_tasks()?;
    let max_id = tasks
        .iter()
        .filter_map(|t| t.id.strip_prefix("T-").and_then(|s| s.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    Ok(format!("T-{}", max_id + 1))
}
```

Time complexity: O(n) where n = number of tasks

For large projects (1000s of tasks), this is still fast (~1ms).

### File System Layout

Each work item is a separate file:

✅ Benefits:
- Parallel access
- Minimal conflicts
- Easy to inspect/edit manually

❌ Trade-offs:
- More files = slower directory listing
- Mitigated by using separate directories per type

### Caching Strategy

Currently, jjj reloads from disk on every command.

Future optimization: In-memory cache with file watchers.

## Backup and Recovery

### Export Metadata

```bash
# Full backup
jj git bundle create jjj-backup.bundle -r jjj/meta

# Or use plain git
cd .jj/jjj-meta
git bundle create ~/jjj-backup.bundle --all
```

### Restore Metadata

```bash
# Restore from bundle
jj git bundle unbundle jjj-backup.bundle
jj bookmark set jjj/meta -r <restored-commit>
```

### Reset Metadata

If metadata gets corrupted:

```bash
# Option 1: Reset to earlier state
jj bookmark set jjj/meta -r <earlier-commit>

# Option 2: Delete and reinitialize
jj bookmark delete jjj/meta
jjj init
```

**Your project code is never affected!** The shadow graph is completely separate.

## Advantages

### vs. Git Notes

Git notes have problems:
- Not pushed by default
- Easy to lose
- No history
- Awkward APIs

jjj's shadow graph:
- ✅ Pushed with `git push --all`
- ✅ Full commit history
- ✅ Standard jj operations

### vs. GitHub Issues / JIRA

External tools require:
- ❌ Internet connection
- ❌ Account/authentication
- ❌ Separate data store
- ❌ API rate limits

jjj:
- ✅ Works offline
- ✅ Lives in your repo
- ✅ No external dependencies
- ✅ Infinite scalability

### vs. Text Files in Repo

Storing `.md` files in project:
- ❌ Pollutes history
- ❌ Merge conflicts with code
- ❌ Clutters working directory

Shadow graph:
- ✅ Clean project history
- ✅ Independent merge conflicts
- ✅ Hidden from code directory

## Future Enhancements

### Planned Improvements

1. **Compression**: Use zstd for large datasets
2. **Indexing**: SQLite index for fast queries
3. **Partial clone**: Fetch only recent metadata
4. **Garbage collection**: Prune old review data

### Compatibility

The storage format is designed to evolve:

- JSON allows schema evolution
- Version field for migration
- Unknown fields ignored

This means old jjj versions can read newer data (graceful degradation).

## Summary

jjj's storage layer uses a **shadow graph** to achieve:

- ✅ Clean separation of metadata and code
- ✅ Atomic operations
- ✅ Offline-first workflow
- ✅ Standard git push/pull
- ✅ Easy backup and recovery

This is only possible because of Jujutsu's flexible commit graph and workspace model!

## See Also

- [Design Philosophy](design-philosophy.md) - Why these choices were made
- [Change ID Tracking](change-tracking.md) - How change IDs enable robust metadata
- [CLI Reference](../reference/cli.md) - Using the storage layer
