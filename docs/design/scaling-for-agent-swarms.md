# Scaling jjj for Agent Swarms

**Status:** Proposed — decisions locked (see "Decisions" below)
**Target regime:** 10–20 concurrent agents on a hard long-running science problem; corpus grows monotonically (never pruned).
**Relationship to the audit:** Forward-looking re-architecture for the agent-swarm scale regime, distinct from `docs/audit/AUDIT_PLAN.md` (correctness bug-fixes). Some P0 sync bugs there are *dissolved* by this redesign; the base-lifecycle ones (0.1/0.5) are *relocated* into a revision-pointer state machine that still needs careful spec + the two-writer test (not eliminated).

---

## Decisions (locked)

**Architecture & scale**
1. **Goal:** ship for a real agent swarm soon. Optimize the critical path to a working swarm; defer pure-elegance work.
2. **Topology:** hybrid — local *pods* (many agents sharing one machine/working copy) + several pods syncing to a remote. Both the local write lock *and* delta-fetch are hot.
3. **Sync model:** **on-demand, agent-as-daemon.** jjj stays a plain CLI; there is **no background daemon** in the tool. `jjj sync` (= delta-fetch + batched push) is a command the agent invokes from its own loop. Cadence policy lives in the SKILL/prompt, not in jjj. **Consequence:** sync is in the agent's synchronous critical path, so **sub-second `jjj sync` for small deltas is a hard requirement** (Pillar 1 is the keystone).
5. **Migration:** **fresh repo.** Adopt the ideal on-disk format directly and version-stamp it. No `jjj migrate`, no dual-format tolerance.
6. **Search (first ship):** **keep FTS5 as-is.** SQLite stays — it is the structured read index + vector store, *not* primarily a search engine. Candle semantic search is a **fast-follow** that will *replace FTS* (semantic + cheap SQL exact-lookup); the decision to drop FTS5 is a separate later step.
7. **Scale target:** **smooth at 25K entities** (sub-second delta-sync and reads), **survives 100K** (graceful, slower, no collapse).

**Swarm coordination semantics**
4. **Coordination:** **diverge & reconcile + soft conventions.** No new hard primitive. Substrate is free overlap (competing conjectures are a feature; real dupes → `problem duplicate --of`). Conventions, documented in the SKILL: soft *domain* specialization per pod via tags or a per-pod milestone; `next --claim` kept as an *advisory* signal, never a lock.
8. **Sync cadence (SKILL convention):** **refined work-item boundaries** — *pull before choosing* next work, *push (batched) after producing* an artifact. `jjj sync` is **pod-debounced** (skip the fetch if a pod-mate synced < ~Ns ago; the batched push only fires when there's new local content) so shared-pod agents don't redundantly hit the remote. `jjj sync --now` forces a barrier at genuine handoffs.
9. **Identity:** **one principal type — a *user*.** Humans and agents are indistinguishable in the data model; both author events, hold claims, rank, and sync under a single user id (already how the code works — `get_current_user` is a string). The id is optionally **namespaced** (`pod-theory/agent-03`, `team/alice`, or bare `alice`) — the namespace is a plain grouping attribute, **not** an agent-vs-human type. Per-user files (event shards `events/{user}.jsonl`, rankings) key uniformly on the id; no agent-specific path or branch anywhere.
10. **Conflict resolution:** **agent always auto-resolves and re-pushes.** Scalar fields already auto-merge by `updated_at`; on a body conflict the pod's agent merges both sides. **Guardrail:** every auto-resolution emits a `ConflictAutoResolved` event recording both sides, so a human can audit/revert a bad merge.
11. **Human role:** **active steerer** — a steering human is just another *user* (decision 9) who injects/reprioritizes problems and sets milestones mid-run on the same write/sync/merge machinery as every other user. Their changes reach other users at the next pull boundary.
12. **Steering authority:** **per-group ranking weight (config, not type).** Ranking influence aggregates by namespace/group with equal influence by default; a steering user's group is simply given a higher weight in config (e.g. **N×**). There is no `if human` branch — agents and humans run the identical code path; only the weight map differs. No absolute pins/vetoes via ranking; any user can still kill a direction via the `problem dissolve` state op.
13. **Observability:** existing TUI tree + `jjj insights` + `jjj events` + the audit trail suffice for the first ship; a dedicated swarm view is deferred until steering proves hard.
14. **Evidence:** **jj Change-ID anchored repo artifacts** — experimental results/data/derivations are committed in the project repo (`data/`, `results/`) and referenced by Change ID, reusing solutions' anchoring (survives rebases, nothing external). *Implication:* critiques can't anchor to a Change ID today; for counter-evidence, recognize change-ids in the critique body for first ship, consider a first-class field later.
15. **Failure/resilience:** **stale-claim expiry + auto-reclaim** — claims carry `claimed_at` + claimant `pod/agent`; after a staleness window an item becomes reclaimable. Claims refresh at the agent's existing sync boundaries (no separate heartbeat); the window must exceed the typical inter-sync interval. In-flight markdown is durable (saved before any crash).

---

## Problem statement

The sync/storage path is **O(total accumulated corpus) on every operation**, not O(delta). For a never-pruned Popperian corpus driven by agents, cost scales with history, not recent activity — backwards. At ~25K entities:

| # | Break point | Where | Cost at 25K |
|---|---|---|---|
| 1 | **Subprocess-per-file fetch** — `jj file show` once per remote file, every fetch, no delta | `fetch.rs:152,197` | ~25K spawns × ~25ms ≈ **~10 min/fetch** |
| 2 | **Monolithic events.jsonl** — full read + dedup + sort + rewrite every fetch; full read on every `list_events` | `events.rs:25`, `merge.rs::merge_events_jsonl` | multi-hundred-MB I/O + O(n log n) per sync |
| 3 | **Full DB rebuild every fetch** — `remove_file(db)` then re-parse + re-index *all* markdown + *all* events | `fetch.rs:242-254`, `db/sync.rs:21` | O(corpus) every fetch |
| 4 | **Single global write lock** — one repo-wide `LOCK_EX` for the whole `with_metadata` section; ×2 from back-ref rewrites | `storage/mod.rs::acquire_write_lock` | throughput ceiling collapses when the section widens |
| 5 | **Remote ref contention** *(distributed; created by the hybrid topology)* — every pod pushes to the single `jjj` bookmark; concurrent pushes reject non-fast-forward and must re-fetch-merge-retry | Pillar 1 push path | sets the real sync-latency floor under many pods |

Breaks 1–4 are the *local incremental* story; **Break 5 is the distributed story the hybrid topology + agent-as-daemon decisions created** — historically the least-specified, highest-risk part of this plan (see Pillar 1 lifecycle + retry, and the Conflict-resolution section). Search is a rounding error next to all of this. The fix is to make **every** path incremental and **every** distributed interaction explicit, and — given decision 3 — make `jjj sync` fast enough to sit inside an agent's loop.

---

## M0 findings (measured 2026-06-19 — see `tools/bench/`)

Probes on jj 0.42 / Apple Silicon / local bare remote; medians.

**Probe 1 — keystone tree-diff: VALIDATED.** `jj diff --from --to --name-only`, two commits differing by 5 files:
- 25K corpus **0.10s** · 100K corpus **0.35s** — both well under the 1s budget. Delta-path discovery scales.
- Flat vs fan-out tree-diff is **identical** (0.358 vs 0.338s @100K) → fan-out is *not* needed for the keystone (claim corrected below).

**Probe 1b — delta content-fetch: forced a Pillar 1 change.** Fetching changed files' content @100K:
- per-file loop (`jj file show` ×K): **15s @ K=50, 60s @ K=200** — a scaled-down Break #1, fatal inline in an agent loop.
- one batched command (multi-path `file show` *or* `jj diff --git`): **~0.32s, flat in K** (47–191× faster).
- ⇒ Pillar 1 **must batch** the content fetch, never loop. With this, the keystone holds.

**Probe 2 — ref contention (Break #5): real, ~quadratic, no data loss.** N pods pushing one `jjj` bookmark at once (WAN latency excluded):
- N=5 → 1.2s · N=10 → 3.2s · N=20 → **14.7s** to drain; mean ~10 / max 19 retries @ N=20; **all writes landed**.
- ⇒ the shared ref serializes writers super-linearly. Fix folded into Pillar 1: **per-pod single-writer bookmarks + fetch-union**.

**Read-your-writes (3rd gate item):** a *policy decision* (writer's own DB upsert stays synchronous — Pillar 2), verified by a unit test when Pillar 2 is built; no standalone probe.

**Verdict:** the delta-fetch keystone is **viable**, conditional on two now-validated refinements — **batched content-fetch** and **per-pod bookmarks**. M0 caught both before they could sink M1. **Cleared to start M1.**

---

## Design pillars

### Pillar 1 — Delta-based fetch + revision-as-merge-base  *(KEYSTONE; kills Break #1 + #5; relocates audit 0.1/0.5 — see lifecycle below)*

- **Track `last_synced_rev`** — the jjj-bookmark commit we last merged — in local-only sync state (`.jj/jjj-meta/.sync_state.json`, never synced).
- **Fetch only the delta — and fetch its content in ONE command (M0-validated + M1-refined).** After `jj git fetch`, resolve each per-pod head (`heads(bookmarks(glob:"jjj*"))`, one subprocess) and, **for each head `H`, diff from the TRUE common ancestor `GCA(last_synced_rev, H) = heads(::last_synced_rev & ::H)` — NOT from `last_synced_rev` directly** (⚠️ correction below). One `jj diff --from <GCA> --to <H> --git --context <huge>` (~0.32s, flat in delta size) returns every added/modified/deleted file's **entire** content on both sides. ⚠️ **Do NOT loop `jj file show` per changed file** — measured **≈94ms/file @25K, ≈300ms/file @100K** (it re-resolves the whole tree per call), so even a 10-file delta blows the sub-second budget at 25K, and the merge needs *two* reads/file (base + remote). **M1 finding — neither M0-listed "batched" option actually returns separable full content:** multi-path `jj file show` **concatenates with no delimiter** (boundaries unrecoverable), and a default `jj diff --git` carries only changed *hunks*, not full files. Reconstruct per file: `base` = ` `/`-` lines, `remote` = ` `/`+` lines (pure parser in `storage/delta.rs`, exhaustively unit-tested). Deletions/adds are explicit in the same diff — drop the separate full-dir `reconcile_remote_deletions` walk.
- **⚠️ M1 keystone correction — the merge base is `GCA(last_synced_rev, H)`, not `last_synced_rev`.** The original "diff from `last_synced_rev`" wording (above, and bullet on per-pod refs below) was **internally inconsistent and the naive reading silently loses data.** It is correct *only* when `last_synced_rev` is a true ancestor of the remote tip — i.e. one shared, fast-forwarded `jjj` line. The moment two pods run on the **parallel per-pod branches that the Break-#5 fix introduces**, `last_synced_rev` (this pod's last-pushed commit) is **not** an ancestor of another pod's head. Diffing straight from it reconstructs a base side that already folds in *our own* unpushed edits; the three-way merge then sees `local == base` and adopts the other pod's stale value — **silently reverting our edit (audit 0.1, exactly relocated).** Empirically reproduced in M1 (jj 0.42): podA sets `status=in_progress`, podB edits the body; naive `diff --from podA --to podB` reverts `in_progress→open`, while `diff --from GCA --to podB` preserves `in_progress` *and* merges podB's body. Fix: per-head `JjClient::merge_base(a,b) = heads(::a & ::b)`; fall back to `root()` (always a safe ancestor — more files diffed, never data lost) when no shared ancestor is reachable. This also unifies cold start: no/unreachable `last_synced_rev` ⇒ base `root()` ⇒ every file shows as added and is adopted.
- **Merge base = a revision, not a `base/` tree.** **Delete the entire `base/` mirror** (`snapshot_base`/`write_base_file`/`read_base_file`); the base is one immutable, jj-reachable revision id (`last_synced_rev`). The merge's *base content* for file F is **reconstructed from the same single full-context diff above** (the `-`/context side) — NOT a separate per-file `file_show`, which would reintroduce the per-file cost. ⚠️ This **relocates** — does not eliminate — the base-lifecycle bug class behind audit 0.1/0.5; see the state machine next.
- **Base-lifecycle state machine (highest-risk area — this is exactly where audit 0.1 lived).** Advancing `last_synced_rev` wrong reintroduces silent local-edit loss. Rules: a `jjj sync` runs **fetch → merge → push** in that order; the three-way merge base is `GCA(`**pre-sync** `last_synced_rev, H)` per remote head (see correction above) — read `last_synced_rev` *before* mutating anything; **only after a successful push** set `last_synced_rev` to the **just-pushed** commit (which already contains merged remote + local). **Never** advance it to a merged-but-unpushed working state. Must be covered by the two-writer test before M1 is "done."
- **Avoid ref contention with per-pod bookmarks (Break #5 — M0-validated).** M0 measured the single shared `jjj` bookmark thundering-herd at ~quadratic cost (20 pods racing = ~14.7s to drain, ~10 mean / 19 max retries, *no data lost*). Fix: **each pod pushes its own single-writer ref `jjj/{pod}`** (never contended); fetch reads all `jjj/*` refs and unions them — the per-user-file sharding philosophy lifted to the ref level, dissolving the bottleneck. Keep a **bounded-exponential-backoff** re-fetch→re-merge→re-push loop as the fallback for any residual race. (Note: per-pod refs make the merge base per-(pod, file) — resolved by the per-head `GCA(last_synced_rev, H)` correction above; this is *why* a single `last_synced_rev` scalar is insufficient as a direct diff base.)
- **Batched push:** one commit per `sync` bundling all local writes since the last push — decouples push cadence from write cadence, no per-write history bloat.
- **Directory fan-out — optional insurance, NOT for tree-diff (M0-corrected).** M0 showed jj tree-diff is the **same** flat vs fan-out (0.358 vs 0.338s @100K), so fan-out is **not** load-bearing for the keystone. It still marginally helps raw enumeration / `file show` and avoids pathological single-directory filesystem behavior at 100K, so adopt `problems/ab/cd/{uuid}.md` cheaply now (free on a fresh repo) as insurance — but don't rely on it for sync latency.
- **Cold start / fallback:** first sync on a fresh clone (no `sync_state`) or an unreachable `last_synced_rev` → full reconcile + `db rebuild`, then re-anchor. This is the **common onboarding path**, not an edge case (see Pillar 2 cold-start lock).
- **Result:** spawn count tracks *activity*, not *history*; sub-second for a small delta **conditional on the M0 validation** of `jj diff` tree-cost at scale and Break #5 contention.

### Pillar 2 — Incremental DB upsert + DB-primary reads  *(kills Break #3)*

DB persists across fetches (ends WAL-sidecar churn). In the Pillar-1 delta loop: changed entity → `sync_to_cache` upsert; deleted → `remove_entity_from_cache`; new events → incremental insert (Pillar 3). Keep `jjj db rebuild` as the explicit full-rebuild escape hatch (the "survives 100K" path — rare, may take a minute, never on a hot path). Add a **sync-generation counter** in the meta row: a crashed mid-fetch bumps a dirty flag; next command sees the mismatch and rebuilds. Markdown stays canonical, so a stale DB is always recoverable.

**Reads go through the DB, uniformly.** Because the delta loop keeps the DB always-current, the DB becomes the **primary read path for every hot read** — `list`/`next`/`status`, the TUI tree, derived back-references (Pillar 4), *and* events/`insights`/`timeline` (Pillar 3) — not just search. The filesystem walk is demoted to **fallback/recovery only** (missing or dirty DB → rebuild, then read DB). This inverts today's model, where reads FS-walk with the DB as an optional accelerator (`storage/mod.rs:146`) and consumers like `insights` read the jsonl directly. Principle: **markdown is the source of truth for *writes*; the DB is the source of truth for *reads*.** A single always-current index serves all queries in O(query), so no read path re-walks or re-parses the corpus.

**Read-your-writes (resolves the Pillar 2 ↔ Pillar 5 tension).** Since reads come from the DB, a writer's *own* DB upsert must be **synchronous within its process** — else an agent that creates an entity then queries it may not see it (create → query → "not found" → duplicate). Rule: the writing process updates the DB for its own write before returning; only *cross-process* visibility may lag (fine — other agents see it at their next sync anyway). This bounds how much of Pillar 5's "upsert outside the lock" is actually safe.

**Cold-start rebuild needs a lock.** A fresh clone (DB lives in unsynced `.jj/`) rebuilds O(corpus) on first command — up to ~2 min at 100K. Several agents hitting a fresh clone at once must not thundering-herd into concurrent rebuilds: the dirty-flag/generation check takes a **rebuild lock** (one builds, others wait, then read).

### Pillar 3 — Per-user event shards + offset index  *(kills Break #2 + append contention)*

Replace single `events.jsonl` with **per-user append-only shards** `events/{user}.jsonl` (mirrors `rankings/{milestone}/{user}.json`); shard key is the user id (decision 9 — namespaced ids like `pod/agent` sanitize to a single shard file) for true single-writer files. Events carry **no correctness weight** (jjj is not event-sourced; markdown is canonical, automation fires on live events) — the log is audit + analytics only. Consequently: **route all readers (`insights`/`timeline`/`events`) through the indexed DB events table, never the jsonl** (today `insights` reads the file via `list_events()` — change it to query the DB). Shards become write-and-sync-only; the offset-indexed incremental load keeps the DB current; sync just adopts new remote shards (LWW union — no read/sort/rewrite).
- **Write:** each agent appends only to its own shard → no cross-writer contention, no whole-file rewrite, no merge.
- **Sync:** per-file last-writer-wins union (reuse `merge_ranking_json`'s class) — no global sort/rewrite.
- **Read:** k-way merge of shards by timestamp; recent-event queries read shard *tails*.
- **Offset index:** track per-shard byte offset already loaded into the DB; incremental load `seek`s and reads only new lines. O(new events).
- Fresh repo → just adopt this format; no migration.

### Pillar 4 — Derive back-references instead of storing them  *(kills write amplification + back-ref merge conflicts; shrinks the lock section → Break #4)*

Today creating a solution rewrites the parent problem's `solution_ids`; a critique rewrites its parent solution's `critique_ids`; etc. — **doubles writes**, serializes hot parents through the global lock, creates a merge-conflict surface (audit 2.10).

Make `solution_ids`, `critique_ids`, `problem_ids` **`#[serde(skip)]` + derived** — the precedent already set for `child_ids` (derived in `list_problems()` from `parent_id`). Forward references (`solution.problem_id`, `critique.solution_id`, `problem.milestone_id`) are the single source of truth; reverse lists are a DB index query (the FS-walk fallback does a one-pass group-by — keep it off hot reads, which now hit the DB per Pillar 2). **Creating an entity now touches exactly one new file** — no parent rewrite, no hot-parent contention, no back-ref conflicts.

Two consequences to handle in the build:
- **Deterministic order:** derived reverse lists must sort canonically (e.g. `created_at` then id) — some consumers rely on today's insertion order.
- **The VS Code extension reads markdown *files* directly** and can't run the DB derive; dropping `solution_ids` from the file removes data a file-only reader depends on. Decide its read path explicitly — query the DB, or do its own O(corpus) group-by — rather than assuming "point it at the derive path."

### Pillar 5 — Lighten the write lock  *(Break #4)*

With Pillar 4 the critical section shrinks to "write one new file." Then: move the *cross-process* part of the cache sync outside the markdown lock — **but keep the writer's own DB upsert synchronous** (Pillar 2 read-your-writes). Keep the global `flock` initially; revisit per-entity locking only if the concurrency benchmark (Pillar 7) shows the global lock is the ceiling after Pillars 3–4. **Caveat:** the lock's real worst-case holder is **`jjj sync`** (it mutates working copy + DB + `sync_state`), not a single write — two pod-mates syncing at once must serialize. The debounce reduces but doesn't remove this; budget the lock around the slow sync op.

### Pillar 6 — Compiled-in candle embeddings  *(FAST-FOLLOW; makes "offline/no-server" true; dissolves audit 1.3)*

Replace the external HTTP embedding client (`src/embeddings.rs` → Ollama) with an **in-process pure-Rust transformer** compiled into the binary.
- **Stack:** `candle-core` + `candle-transformers` (BERT) + `tokenizers`. No C++/ONNX dep; optional `metal`/`accelerate` features for Apple-Silicon accel.
- **Model:** 384-dim small encoder (all-MiniLM-L6-v2 / bge-small-en-v1.5). `safetensors` + `tokenizer.json` via `include_bytes!`. Binary +~90MB (accepted). Store the asset via git LFS.
- **Inference:** BERT forward → mean-pool → L2-normalize. ~10–50ms/doc CPU (faster on Metal).
- **Incremental, off the write path:** a candle forward is ~10–50ms — too slow inside the write lock (would cap writes at ~20/s, below the 100/s target). Embed **asynchronously/batched** after save and on delta-fetch ingest; store vectors in the SQLite `embeddings` table. No batch wipe → fixes 1.3 by construction.
- **Query:** brute-force cosine is sub-10ms at 25K×384. At the **100K survive-target**, 100K×384×4B ≈ 150MB of vectors — keep memory-mapped/resident; add an `hnsw` ANN index only if a 100K bench shows brute-force is too slow.
- **Replaces FTS:** at cutover, search = candle semantic + plain SQL exact-lookup (`LIKE`/indexed columns for id/tag/title/filename); delete the FTS5 virtual table + sanitizer/ranking code. Drop the HTTP client → README "no server / offline-first" becomes literally true.
- **Packaging:** behind a `semantic` cargo feature (default-on for release, off for a lean build).

### Pillar 7 — Scale validation & benchmarks  *(build the generator first)*

- **Synthetic corpus generator** (dev tool): branching problem DAG + solutions/critiques/events, materializing 1K / 5K / 25K / 100K corpora on demand.
- **Multi-process sync rig** — the generator alone can't measure sync. Delta-fetch, **remote-push contention (Break #5)**, and 20-writer throughput need *concurrent processes against a shared bare remote* + multiple clones. This rig — not just the file generator — is the real M0 deliverable; its fidelity to real concurrency gates the value of every number.
- **Benches:** cold fetch, warm delta-fetch (100-file delta), `list`/`next`/`status`, FTS search, write-throughput under 20 concurrent writers, push-contention under N pods, and (fast-follow) semantic query.
- **Acceptance (decision 7):** at **25K** — delta-sync < 1s, reads < 200ms, sustained writes > 100/s under 20-writer contention. At **100K** — no collapse: delta-sync still sub-second, full rebuild < ~2 min (rare), reads < ~1s.

**M0 must-validate gate — RESOLVED (see "M0 findings"):**
1. ✅ `jj diff --name-only` is **0.10s @25K / 0.35s @100K** (sub-second). *Caught:* per-file content fetch was fatal (60s @ 200 files) → **batched fetch required**; fan-out *not* needed.
2. ✅ Ref contention is real (~quadratic, **14.7s @20 pods**, no data lost) → **per-pod bookmarks + backoff**.
3. ⏳ Read-your-writes — a policy (synchronous self-upsert, Pillar 2), to verify by unit test in M1.
**Cleared to start M1.**

---

## First-ship scope vs fast-follow

**First ship (the critical path to a non-collapsing swarm):**
- **M0 — Corpus generator + scale probes + must-validate gate** (Pillar 7) — ✅ **done** (`tools/bench/`): keystone validated, two refinements found (batched content-fetch, per-pod bookmarks), gate cleared. Read-your-writes deferred to a Pillar 2 unit test.
- **M1 — Incremental sync core** (Pillars 1 + 2). The keystone; sub-second `jjj sync`. **Relocates** the base lifecycle into a revision-pointer state machine (highest-risk area — requires the two-writer test) and adds the Break #5 push-retry loop.
- **M2 — Event shards** (Pillar 3). Fresh-format adoption; kills the events bottleneck + append contention.
- **M3 — Derived back-refs + lighter lock** (Pillars 4 + 5). Kills write amplification; makes the hybrid topology's local-pod write path scale.
- **SKILL sync + coordination protocol** (load-bearing for decisions 3/4/8/10/15) — encode: the refined-work-item-boundary cadence (pull-before-choose, push-after-produce; `jjj sync --now` at handoffs); soft domain specialization per pod (tags/milestone); advisory `next --claim` (check after sync; skip live claims, reclaim stale ones); agent conflict auto-resolution on a blocked push; and the evidence convention (commit artifacts to `data/`/`results/`, cite by Change ID).
- **Swarm primitives in the tool** — `jjj sync` (delta-fetch + batched push, pod-debounced, `--now`); a single namespaced **user** identity from config/env (no agent-vs-human type); `claimed_at` + staleness on claims; `ConflictAutoResolved` event; group-weighted ranking aggregation (equal per group by default; per-group weight in config — uniform code path for all users).

Search is **untouched** in the first ship (FTS5 stays).

**Fast-follow:**
- **M4 — Candle embeddings** (Pillar 6), then the FTS5 removal as its own step.
- ANN index, per-entity locking — only if benchmarks demand.

M0 first; M1 is the critical path; M2 and M3 parallelize after M1's delta loop exists (both hook the same ingest path). M4 is fully decoupled and can proceed in parallel by a separate stream.

---

## Conflict resolution (decision 10) — needs its own mechanism

Policy is "agent auto-resolves," but the *mechanism* is unspecified and breaks two invariants if done naively:
- **Machine-readable surface:** `jjj sync` must emit a **structured conflict report** (`--json`: entity id, field/body, both sides, base) — not just `<<<<<<<` markers in a file — and accept a resolution back via an explicit path (`jjj resolve <id>` committing the agent's merged content + the `ConflictAutoResolved` event).
- **Latency:** an agent merge is an LLM call → **unbounded latency inside the otherwise sub-second sync.** Keep it off the hot path: `jjj sync` *surfaces* conflicts and returns; the agent resolves and re-syncs as a separate step. Sub-second is the no-conflict path only.
- **Blast radius:** an agent can be *wrong* on a genuine scientific disagreement; the `ConflictAutoResolved` event audits it, but a bad merge can propagate before a human reviews. Design mitigations: keep both sides recoverable (the event records them); consider routing high-stakes entities (e.g. approved solutions) to human-review-instead-of-auto.

Conflicts should be **rare** by construction (diverge-and-reconcile mostly creates *new* entities; scalars auto-merge by `updated_at`; bodies are append-mostly) — so this path's *correctness* matters more than its speed.

---

## Risks

- **Base-lifecycle correctness (highest risk)** — advancing `last_synced_rev` wrong = silent local-edit loss (audit 0.1 relocated). Spec the state machine (Pillar 1) and prove it with the two-writer test before M1 ships.
- **Remote ref contention (Break #5)** — concurrent pod pushes reject; needs the fetch-merge-push retry loop + backoff, and may set the latency floor. Measure in M0.
- **Read-your-writes** — DB-primary reads (Pillar 2) vs. DB-lag (Pillar 5) conflict; resolved by keeping the writer's own upsert synchronous, but easy to regress.
- **Cold-start thundering herd** — fresh-clone full rebuild needs a rebuild lock so N agents don't rebuild concurrently.
- **Conflict mechanism** — policy without a built interface; see the Conflict-resolution section (structured report + `jjj resolve` + off-hot-path).
- **`last_synced_rev` reachability** — must fall back to full reconcile when the revision is GC'd/rewritten.
- **Derived back-refs reader parity** — VS Code reads markdown files and can't run the DB derive; choose its read path explicitly. Derived lists need a deterministic sort.
- **Persistent DB + concurrent writers** — relies on WAL + `busy_timeout` (audit 1.1). Keep.
- **candle build weight** — 90MB LFS asset, longer compile, larger binary; mitigated by the `semantic` feature flag (fast-follow anyway).
- **Correctness parity** — each milestone ships with its scale bench *and* the two-writer integration test the audit flagged missing (P5): two agents edit the same entity → body divergence → conflict markers (push-blocked by audit 0.4); scalar divergence → `updated_at` resolves; second sync is a no-op (base advanced via `last_synced_rev`).

---

## What this buys

- `jjj sync` and DB maintenance become **O(delta)** — sub-second at any corpus size, fit to sit inside an agent's loop.
- Writes touch **one file** — 20 agents in a pod stop contending on hot parents and a shared event file.
- Coordination needs **no new primitive** — diverge-and-reconcile + soft conventions ride the existing model.
- The fragile `base/` tree disappears, and audit 1.3 is **dissolved by design** (fast-follow). The 0.1/0.5 base bugs are **relocated** into a revision-pointer state machine — simpler, but still the area that needs the two-writer test.
- Semantic search (fast-follow) is **offline, in-process, honest** — no server.

The through-line: **cost scales with recent activity, not accumulated history** — the property the agent-swarm regime requires.
