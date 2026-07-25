# Changelog

## 0.5.0 — 2026-07-25

The "scale and consolidate" release: full-codebase audit remediation, the
agent-swarm scaling milestones (M1–M4), a coordination layer for multi-pod
operation, and the perf fixes surfaced by the new benchmark harness. At a
25K-entity corpus, every DB-backed read (list/status/next/search/events) now
completes in under 200ms.

### Scaling (agent swarms)

- **Incremental sync (M1):** fetch resolves each per-pod head and applies one
  batched delta diff from the true common ancestor instead of enumerating
  every remote file; each pod pushes its own single-writer `jjj/{pod}`
  bookmark, dissolving push contention. Reads are DB-primary with a
  markdown-canonical fallback, and a dirty cache heals itself on the next
  read.
- **Per-pod event shards (M2):** events append to `events/{pod}.jsonl`
  (no contention, no merge conflicts); a local byte-offset index makes DB
  ingest incremental. `events`, `insights`, and `timeline` now read from the
  SQLite events table (O(query)) with an O(delta) tail top-up.
- **Derived back-references (M3):** `solution_ids`/`critique_ids`/
  `problem_ids` are derived at read time from forward references — nothing to
  keep consistent on write. Critiques are cache-faithful (schema v10), so all
  four entity types serve from the cache.
- **In-process semantic embeddings (M4, `semantic` feature):** the Ollama
  HTTP client is gone. Embeddings run in-process via candle BERT
  (mean-pool + L2-normalize), loading a model from a runtime path
  (default `~/.cache/jjj/models/all-MiniLM-L6-v2`). Fully offline, no
  server. Default builds are lean and run FTS-only search.

### Coordination

- Unified actor identity (`JJJ_USER` > pod > jj user) and `JJJ_POD` process
  override; `jjj whoami [--json]` shows the resolved actor/pod/push bookmark.
- Structured conflict resolution: `jjj conflicts [--json]` lists entities
  with unresolved markers; `jjj resolve <id> --ours|--theirs [--rationale]`
  collapses one deterministically and logs a `conflict_resolved` event.

### Ranking

- Quadratic votes replaced by **sized gaps**: order plus per-slot gap
  (S/M/L/XL) is the single authored signal. Global ranking is a
  budget-normalized, gap-weighted harmonic aggregation — equal influence per
  user, top-of-list dominant, un-annotated lists backward compatible.
  TUI: nudge/fling (Shift+J/K, double-tap), `p` cycles gaps, Ctrl+Z undo.

### Performance

- Unique-prefix display computation rewritten from O(n²) to O(n log n)
  (`problem list` at 25K: 115s → 0.1s).
- `jjj search` no longer rebuilds the whole DB per query (25K: ~10s →
  0.08s); synchronous per-save FTS upserts keep results fresh.
- Embedding requests batched; TUI lookups indexed.

### Reliability (audit remediation)

- Fetch performs a true three-way merge per file with per-file base
  advancement; remote deletions reconcile; conflict markers are blocked at
  push. Rankings sync with per-file last-writer-wins union.
- Automation shell actions pass entity values via environment variables —
  untrusted titles can no longer inject shell commands.
- SQLite opens with WAL + busy timeout; a re-entrant repo-wide write lock
  serializes concurrent writers; atomic writes everywhere.
- The DB→markdown dump path was removed entirely: markdown is the sole
  source of truth, so an interrupted cache can never overwrite canonical
  files.
- Event-type parsing is exhaustive and derived from the enum — unknown-type
  corruption (including all GitHub event types) fixed.

### Tooling

- `tools/bench/bench.py`: repeatable release-gate benchmark (reads, rebuild,
  search, events, concurrent-writer throughput, push/fetch/delta-sync) with
  a recorded 25K baseline in `tools/bench/README.md`.
- CI installs jj and sets `JJJ_REQUIRE_JJ=1` so jj-backed tests can't
  silently skip.

### VS Code extension (0.2.0)

- New commands: Sync Metadata (with post-fetch conflict detection), Resolve
  Conflicts, Show Identity (whoami), Claim Next Work Item.
- Test suite repaired after the body-field migration; 286 tests passing.

### Known gaps

- Sync latency at 25K (delta push ~12s, delta fetch ~7s) is above the
  design's sub-second target — tracked as the top open perf item.
- FTS5 removal (search = semantic + SQL exact-lookup) is deliberately a
  separate future step.

## 0.4.1 and earlier

Pre-changelog releases; see git history.
