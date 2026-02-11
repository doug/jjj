# Embedding Search Design

## Overview

Add semantic embedding support to jjj's search functionality, enabling similarity-based discovery alongside existing full-text search (FTS). Embeddings are computed via OpenAI-compatible APIs (defaulting to local Ollama) and stored in the local SQLite cache.

## Goals

1. **Semantic similarity** - Find problems/solutions similar to a given entity
2. **Smart recommendations** - Surface related items when viewing entities
3. **Hybrid search** - Combine FTS keyword matching with semantic relevance for better ranking

## Non-Goals (This Phase)

- Syncing embeddings across machines (they're local cache, recomputed per machine)
- Built-in embedding model (uses external API)
- Approximate nearest neighbor indexing (brute force sufficient for expected scale)

## Design Decisions

### Embedding Service

**OpenAI-compatible API** with Ollama as default:
- Default URL: `http://localhost:11434/v1/embeddings`
- Works with Ollama, OpenAI, Together, Voyage, local vLLM, etc.
- Single implementation covers all providers

### Configuration

**Local config file: `.jj/jjj.toml`** (never synced, like `.jj/jjj.db`)

```toml
[embeddings]
enabled = true
base_url = "http://localhost:11434/v1"
model = "qwen3-embedding:8b"
dimensions = 4096
api_key = ""  # optional, for remote APIs
```

**Environment variable overrides:**
- `JJJ_EMBEDDINGS_ENABLED`
- `JJJ_EMBEDDINGS_BASE_URL`
- `JJJ_EMBEDDINGS_MODEL`
- `JJJ_EMBEDDINGS_DIMENSIONS`
- `JJJ_EMBEDDINGS_API_KEY`

Env vars take precedence over file values.

### Graceful Degradation

**No config / not explicitly enabled:**
- Silently try default Ollama URL
- If available → use embeddings
- If unavailable → silently disable, FTS-only (no warning)

**Explicitly enabled (`enabled = true`):**
- Try configured URL
- If available → use embeddings
- If unavailable → warn once per session, fall back to FTS-only

### Storage

**SQLite only** - embeddings are local cache, recomputed per machine.

```sql
CREATE TABLE embeddings (
    entity_type TEXT NOT NULL,  -- "problem", "solution", "critique", "milestone"
    entity_id TEXT NOT NULL,
    model TEXT NOT NULL,        -- e.g., "qwen3-embedding:8b"
    dimensions INTEGER NOT NULL,
    embedding BLOB NOT NULL,    -- f32 array as bytes
    created_at TEXT NOT NULL,   -- RFC3339 timestamp
    PRIMARY KEY (entity_type, entity_id)
);

CREATE INDEX idx_embeddings_model ON embeddings(model);
```

Storing model name allows detection of stale embeddings when model changes.

### Computation Timing

**Hybrid approach:**
- Compute embedding on entity create/update (incremental)
- Recompute all during `jjj db rebuild` (batch)

Batch rebuild uses batched API calls (~20-50 texts per request) for efficiency.

### Text Preparation

Content concatenated for embedding per entity type:

| Entity | Fields |
|--------|--------|
| Problem | `{title}\n\n{description}\n\n{context}` |
| Solution | `{title}\n\n{approach}\n\n{tradeoffs}` |
| Critique | `{title}\n\n{body}` |
| Milestone | `{title}\n\n{description}` |

### Similarity Computation

**Cosine similarity in Rust** (no SQLite extension):
- Load candidate embeddings from SQLite
- Compute similarity scores in Rust
- Sort and return top-k results

For jjj's expected scale (hundreds to low thousands of entities), brute force O(n) is sufficient. Can extend to ANN with sqlite-vec later if needed for large scale.

## CLI Changes

### `jjj search` (enhanced)

Smart detection of query type:

```bash
# Hybrid text search (FTS + semantic when available)
jjj search "auth bug"
jjj search "auth bug" --type problem

# Similarity search (entity reference detected)
jjj search p/01957d
jjj search s/01958a --type critique

# Force text-only (skip semantic)
jjj search "auth bug" --text-only
```

**Hybrid ranking** uses Reciprocal Rank Fusion (RRF):
```
RRF score = Σ 1/(k + rank) across FTS and semantic results
```

### `jjj db rebuild` (extended)

Rebuilds FTS index and embeddings (if service available). No new flags needed.

### `jjj db status` (extended)

```
$ jjj db status
Database: .jj/jjj.db
Schema version: v2
Entities: 47 problems, 23 solutions, 15 critiques, 3 milestones
FTS index: 88 documents
Embeddings: 88/88 (model: qwen3-embedding:8b)
Sync status: clean
```

## Implementation

### New Files

| File | Purpose |
|------|---------|
| `src/embeddings.rs` | EmbeddingClient for API calls |
| `src/config.rs` | LocalConfig loader for `.jj/jjj.toml` |

### Modified Files

| File | Changes |
|------|---------|
| `src/db/schema.sql` | Add embeddings table |
| `src/db/schema.rs` | Bump schema version, create embeddings table |
| `src/db/entities.rs` | Integrate embedding computation on upsert |
| `src/db/sync.rs` | Add `rebuild_embeddings()` to rebuild flow |
| `src/commands/search.rs` | Hybrid search, entity reference detection |
| `src/commands/db.rs` | Extended status output |

### EmbeddingClient Interface

```rust
pub struct EmbeddingClient {
    base_url: String,
    model: String,
    dimensions: usize,
    api_key: Option<String>,
    http_client: reqwest::blocking::Client,
}

impl EmbeddingClient {
    /// Returns None if service unavailable
    pub fn from_config(config: &LocalConfig, warn_on_error: bool) -> Option<Self>;

    /// Compute embedding for single text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Batch embed multiple texts (single API call)
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
}
```

### Search Flow

```rust
pub fn search(query: &str, entity_type: Option<&str>) -> Result<Vec<SearchResult>> {
    // 1. Detect entity reference (e.g., "p/01957d")
    if let Some((etype, id)) = parse_entity_reference(query) {
        return similarity_search(etype, id, entity_type);
    }

    // 2. Hybrid text search
    let fts_results = fts_search(query, entity_type)?;

    // 3. If embeddings available, combine with semantic
    if let Some(client) = get_embedding_client() {
        let semantic_results = semantic_search(client, query, entity_type)?;
        return merge_with_rrf(fts_results, semantic_results, k=60);
    }

    Ok(fts_results)
}
```

## Phasing

### Phase 1: Core Infrastructure
- Embedding configuration and client
- SQLite storage for embeddings
- Compute on create/update + rebuild
- Smart `jjj search` with hybrid ranking and similarity detection

### Phase 2: Recommendations
- `jjj show` includes "Related" section
- TUI sidebar shows related items when viewing an entity

### Phase 3: Proactive Detection
- Duplicate/similar detection on `jjj problem create`
- Configurable similarity threshold for warnings

---

## Phase 2: Recommendations (Detailed Design)

### `jjj show` Related Section

When viewing any entity, show semantically similar items at the bottom:

```bash
$ jjj show p/01957d

Problem: Authentication timeout causes user logout
ID: 01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a
Status: Open
Priority: High
...

## Description
Users are being logged out unexpectedly when...

## Context
This started happening after the session...

────────────────────────────────────────────────────

Related:
  p/01958a  [0.89]  "Session expiration not refreshing tokens"
  p/01823e  [0.84]  "Login flow drops auth state on redirect"
  s/01234b  [0.82]  "Implement token refresh middleware"
  c/01567c  [0.78]  "Critique: Session timeout too aggressive"
```

**Implementation:**

```rust
// src/commands/show.rs

fn show_entity(entity_type: &str, entity_id: &str) -> Result<()> {
    // ... existing show logic ...

    // Add related section if embeddings available
    if let Some(related) = find_related(conn, entity_type, entity_id, top_k=5)? {
        println!("\n{}\n", "─".repeat(40));
        println!("Related:");
        for item in related {
            println!("  {}/{}  [{:.2}]  \"{}\"",
                item.entity_type.chars().next().unwrap(),
                &item.entity_id[..6],
                item.similarity,
                truncate(&item.title, 50)
            );
        }
    }

    Ok(())
}
```

**Related items query:**
- Retrieves embedding for source entity
- Computes cosine similarity against all other entities
- Returns top-k results above minimum threshold (0.5)
- Excludes the source entity itself
- Can filter by entity type if specified

### TUI Related Panel

When viewing an entity in the TUI, show related items in a dedicated panel.

**Layout option 1: Bottom panel**
```
┌─────────────────────────────────────────────────────────┐
│ Problem: Authentication timeout causes user logout      │
│ Status: Open  Priority: High                            │
│                                                         │
│ ## Description                                          │
│ Users are being logged out unexpectedly when...         │
│                                                         │
├─────────────────────────────────────────────────────────┤
│ Related (r to toggle)                                   │
│ > p/01958a [0.89] Session expiration not refreshing...  │
│   s/01234b [0.82] Implement token refresh middleware    │
│   c/01567c [0.78] Session timeout too aggressive        │
└─────────────────────────────────────────────────────────┘
```

**Layout option 2: Right sidebar (when terminal wide enough)**
```
┌────────────────────────────────────┬────────────────────┐
│ Problem: Auth timeout causes...    │ Related            │
│ Status: Open  Priority: High       │                    │
│                                    │ p/01958a [0.89]    │
│ ## Description                     │ Session expir...   │
│ Users are being logged out...      │                    │
│                                    │ s/01234b [0.82]    │
│                                    │ Token refresh...   │
└────────────────────────────────────┴────────────────────┘
```

**TUI keybindings:**
- `r` - Toggle related panel visibility
- `Enter` on related item - Navigate to that entity
- `j/k` - Scroll related list when focused

**Implementation:**

```rust
// src/tui/app.rs

pub struct App {
    // ... existing fields ...
    show_related: bool,
    related_items: Vec<RelatedItem>,
    related_selected: usize,
}

impl App {
    fn load_related(&mut self, entity_type: &str, entity_id: &str) {
        if let Some(client) = &self.embedding_client {
            self.related_items = find_related(&self.conn, entity_type, entity_id, 10)
                .unwrap_or_default();
        }
    }
}
```

**Lazy loading:** Related items are computed when first viewing an entity, then cached for that viewing session. Moving to a different entity clears and recomputes.

### Phase 2 Files

| File | Changes |
|------|---------|
| `src/commands/show.rs` | Add related section to output |
| `src/tui/app.rs` | Add related panel state and keybindings |
| `src/tui/ui.rs` | Render related panel |
| `src/db/search.rs` | Add `find_related()` function |

---

## Phase 3: Proactive Detection (Detailed Design)

### Duplicate Detection on Create

When creating a new problem, check for semantically similar existing problems and warn the user:

```bash
$ jjj problem create "Fix authentication timeout bug"

Similar existing problems found:

  p/01957d  [0.91]  "Authentication timeout causes user logout"
    Status: Open, Priority: High

  p/01823e  [0.85]  "Login session timeout issue"
    Status: InProgress, Priority: Medium

Create anyway? [y/N/v]
```

**Options:**
- `y` - Create the new problem anyway
- `N` - Cancel (default)
- `v` - View details of a similar problem before deciding

### Configuration

Add threshold setting to `.jj/jjj.toml`:

```toml
[embeddings]
enabled = true
base_url = "http://localhost:11434/v1"
model = "qwen3-embedding:8b"
dimensions = 4096

# Phase 3 settings
duplicate_threshold = 0.85      # warn if similarity exceeds this
duplicate_check_enabled = true  # can disable duplicate checking
```

### Implementation

```rust
// src/commands/problem.rs

fn create_problem(title: &str, description: &str, ...) -> Result<()> {
    let config = load_local_config()?;

    // Check for duplicates if embeddings available and enabled
    if config.embeddings.duplicate_check_enabled {
        if let Some(client) = EmbeddingClient::from_config(&config, false) {
            let text = format!("{}\n\n{}", title, description);
            let embedding = client.embed(&text)?;

            let similar = find_similar_problems(
                &conn,
                &embedding,
                config.embeddings.duplicate_threshold,
                top_k=5
            )?;

            if !similar.is_empty() {
                print_similar_warning(&similar);

                match prompt_user()? {
                    Response::Yes => { /* continue */ }
                    Response::No => return Ok(()),
                    Response::View(id) => {
                        show_problem(&id)?;
                        return create_problem(title, description, ...); // recurse
                    }
                }
            }
        }
    }

    // ... existing create logic ...
}
```

### Batch Detection for Solutions

Also check when creating solutions - warn if a similar solution already exists for the same problem:

```bash
$ jjj solution create p/01957d "Add token refresh mechanism"

Similar solutions for this problem:

  s/01234b  [0.88]  "Implement token refresh middleware"
    Status: Proposed

Create anyway? [y/N/v]
```

### Skip Flag

Add `--force` flag to bypass duplicate checking:

```bash
jjj problem create --force "Fix authentication timeout bug"
```

### Phase 3 Files

| File | Changes |
|------|---------|
| `src/commands/problem.rs` | Add duplicate check before create |
| `src/commands/solution.rs` | Add duplicate check before create |
| `src/config.rs` | Add duplicate_threshold, duplicate_check_enabled |
| `src/cli.rs` | Add --force flag to create commands |

---

## Future Extensibility

- **ANN indexing**: Can add sqlite-vec for approximate nearest neighbor if scale requires it
- **Multiple models**: Could support different models per entity type
- **Embedding versioning**: Model field in schema supports detecting stale embeddings

## Testing

- Unit tests for EmbeddingClient (mock HTTP responses)
- Integration tests for hybrid search ranking
- Tests for entity reference detection parsing
- Tests for graceful degradation when service unavailable
