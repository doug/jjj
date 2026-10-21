# Plan: the swarm core

**Written 2026-10-19.** Base `401ec92`. Design rationale lives out of band in
`~/src/memory/jjj/swarm-core-design.md` — read §2 (the two founding insights), §3
(git-bug verification), §5 (the human interface) and §6 (why not voting) before taking
any unit in this plan. This file is the executable half; it does not re-argue the design.

Target: 40,759 LOC in `src/` → 8–10K, three nouns, seven verbs, an op-log substrate, and
one human surface that is not a duplicate model.

**Not started. Not approved. No unit below is authorised to delete anything until its
gate passes.**

---

## 0. Working agreements

These are not style preferences. Every one of them cost real time in the session that
produced this plan.

1. **Every commit must compile on its own.** Run `scripts/verify-commits.sh` (see §6)
   before pushing a branch. A unit whose commits only compile in aggregate is rejected.
2. **Never pipe a gate step through `rg`.** `cargo clippy ... | rg '^error'` makes the
   pipeline status ripgrep's, so a green run can sit on top of real errors. This happened.
   **Already fixed** — `scripts/gate.sh` has `set -uo pipefail` and runs each stage through
   a pipe-free `run` helper, and its header comment records the incident. Verified
   2026-10-19. The rule exists so it is not reintroduced, in `gate.sh` or anywhere else.
   **A second defect of the same family was found and fixed on 2026-10-19:** `gate.sh` ran
   `cargo fmt --all` unconditionally immediately before `run fmt cargo fmt --all --check`,
   so the fmt stage formatted the tree and then verified its own output. A check that cannot
   fail is not a check, and it reported OK over work it had silently changed. The auto-fix is
   gone; run `cargo fmt --all` yourself. The general rule: **a gate stage must not be able to
   repair what it is measuring.**
3. **`skills/jjj/SKILL.md` is `include_str!`'d.** Every edit forces a full crate rebuild
   plus 56 test binaries, and there is a drift test. Batch all SKILL.md work into H1 at
   the end. Do not touch it incidentally.
4. **Never hand-write patch context.** Generate patches from verbatim `git show
   HEAD:<path>` output. A hand-typed context line with box-drawing characters will be
   rejected by `git apply` with a useless message.
5. **Hunk indices renumber after every staging and commit.** Re-list before each take.
6. **Exit 13 is reserved for `JjjError::Precondition` only.** Everything else is 1.
   beads uses 13 for the same meaning; do not broaden it.
7. **`gh` is unauthenticated in this environment** — 60 requests/hour, IP-based. Do not
   poll `gh run watch`. Make single `gh api .../jobs` calls, spaced.
8. **Commit messages**: imperative mood, concise subject, body only when the "why" is
   not obvious. No AI attribution of any kind, no co-author lines, no session trailers.
9. **Do not commit unless the user asked.** Do not tag. Do not publish to crates.io.
   `Cargo.toml` is at 0.5.3 with the tag deliberately held.
10. **A freshly linked binary stalling at `_dyld_start` with zero CPU is not your bug.**
    Under several agents running corpus benchmarks, page-in starvation can hold a new binary
    for 3–8 minutes before `main`. It looks exactly like a deadlocked test. `/usr/bin/sample`
    will show the stall is pre-`main`, and `cp`ing the identical binary runs instantly. Wait
    it out. This cost one agent ~25 minutes of debugging its own correct code.
11. **A unit is done when its gate passes, not when the code is written.** `scripts/gate.sh`
    must report `GATE_EXIT=0` across fmt, invariants, clippy and tests.

### The compatibility licence

Only two repos anywhere have `.jj/jjj-meta`, and both are months stale (newest entity
2026-06-01 and 2026-03-08). There is effectively no installed base to break. **This is
the reason a breaking rewrite is affordable** — treat it as a one-time licence and spend
it deliberately in Phase C and Phase E rather than preserving compatibility by reflex.
A migration path is still required (C6, E1, E2), because "recoverable" and "nobody will
notice" are different standards.

---

## 1. Lanes

Assign with `jjj next --lane <lane>`. One agent per unit; lanes exist so that two agents
do not contend on the same files.

| lane | owns | files |
|---|---|---|
| `measure` | Phase A | `tools/bench/`, telemetry, throwaway spikes |
| `substrate` | Phases B, C | `src/db/`, `src/storage/` |
| `verbs` | Phase D | `src/commands/`, `src/cli.rs` |
| `model` | Phase E | `src/models/`, migrations |
| `host` | Phase F | `src/mcp/` (new), `ui/` (new) |
| `prune` | Phase G | deletions only |
| `docs` | Phase H | `skills/`, `CHANGELOG.md`, `scripts/` |

---

## 2. Phase A — Measure. Gates everything.

Nothing is deleted and nothing is built until these four land. The cut list in the
design note is an argument; this phase turns it into a measurement or kills it.

| id | unit | depends | parallel |
|---|---|---|---|
| A1 | Opt-in local command telemetry | — | yes |
| A2 | Cache-off benchmark of `prime`/`query` | — | yes |
| A3 | **Op-log replay cost spike** | — | yes |
| A4 | Seven-verb swarm trial | A1 | after A1 |

**A1 — telemetry.** Append `{ts, command, subcommand, ms}` to a machine-local file under
`.jj/jjj-meta/` (never synced, same class as `.lamport`). Off unless
`JJJ_TELEMETRY=1`. No entity content, no titles, no paths. *Acceptance:* a week of
`jjj` use produces a histogram of which of the ~102 leaf operations were actually
invoked.

**A2 — cache-off benchmark.** Run `prime`, `query` and `next` with `src/db/` bypassed,
against corpora of 50, 500, 5,000 and 25,000 entities (the bench harness in
`tools/bench/` already has a 25K baseline). *Acceptance:* a table of p50/p99 latency per
command per corpus size, cache on vs off. *Kill criterion:* if cache-off `prime` exceeds
500ms at 5,000 entities, B1 does not proceed and the plan keeps a rebuildable index.

**A3 — op-log replay cost spike. This is the single largest technical risk in the plan.**
The design note assumes replay is cheap because the entity count is small. That is the
wrong variable: **replay cost grows with operation count, not entity count.** A problem
edited 500 times has 500 operations to replay before its current state is known, and
nothing bounds that. Build a throwaway prototype (not on a branch that will merge) that
synthesises packs and measures replay time against 10/100/1,000/10,000 ops per entity.
*Acceptance:* a curve, plus a stated answer to "at what op count does an entity need a
snapshot?" *Kill criterion:* if replay at 1,000 ops/entity exceeds 50ms, Phase C must
include a snapshot/compaction unit (C9) before C8 deletes `merge.rs`.

### A2 result (2026-10-19) — ran; criterion did not fire, by 11%

M2 Mac, release, 100 iterations/cell, p99 by nearest rank. Cache-off = `JJJ_NO_DB=1` with
`.jj/jjj.db` removed. Corpus generated fresh with long-tailed bodies, a parent forest to
depth 4, all five kinds, and **UUID7-structured ids** — the agent caught that a
counter-derived id scheme would share leading hex digits and inflate `prime`'s
shortest-unique-prefix computation adversarially.

| command | 50 | 500 | 5,000 | 25,000 |
|---|---|---|---|---|
| `prime` on | 38.8 ms | 41.7 ms | 87.0 ms | 856.3 ms |
| `prime` **off** | 40.8 ms | 76.8 ms | **451.4 ms** (p99 604) | 2884 ms |
| `query` off/on | 1.2× | 3.1× | 12.3× | 18.8× |

**The kill criterion did not fire** — 451 ms p50 against the 500 ms line, reproducible to
1 ms over a 200-iteration re-run. Three things belong beside that verdict:

- **The margin is 11%, and the p99 of that same cell is 604 ms — already past the budget.**
  The criterion was written on p50, so it passes; on p99 it would have fired.
- **The knee is exactly at the gate.** ~82 µs/entity at 5,000 and ~114 µs at 25,000 after
  subtracting a ~41 ms process floor, so a 500 ms budget lands at 4,500–5,500 entities. The
  threshold I picked sat on the inflection by coincidence, not by insight.
- **It is not comfortably fast at 25,000** — 2.88 s.

**What actually strengthens the deletion is a different number: at 25,000 entities `prime`
misses 500 ms *with* the cache too, at 856 ms.** The cache does not make `prime` fast at
scale, it makes it 3.4× less slow; below 500 entities it is worth under 40 ms. So the
cache's entire value window is a corpus of roughly 2,000–25,000 entities, and the observed
corpus is 52. Deleting it costs nothing that exists and forfeits no 25K capability, because
there is no 25K capability.

**The decomposition matters more than the verdict.** Re-running with the event shards
emptied (75,004 events → 4) moved `prime` by 2% at 5,000 and 0% at 25,000. **The event log
is not the cost; per-entity `parse_frontmatter` is.** So if `prime` ever needs to be fast
again, the lever is parsing — lazy or partial frontmatter decode, a per-file hash skip —
**not an index over events.**

### The two results compose, and together they settle B1

A3 found that deserialisation, not the fold, is the op-log's cost: 0.84 µs/op. A2 found that
per-entity frontmatter parsing, not the event log, is today's cost: ~82 µs/entity at 5,000.
These are the same finding from two directions — **the cost is turning bytes into structs,
and both a snapshot and a derived index must therefore store *materialised state*, not a
memoised fold or an event index.** Neither agent could see that; it only appears when the
two reports are laid side by side.

Composing the numbers gives the prediction the plan was missing:

| ops/entity after Phase C | added per-entity cost | predicted cache-off `prime` @ 5,000 |
|---:|---:|---:|
| 1 (observed today) | ~1 µs | ~455 ms — passes |
| 100 | ~84 µs | **~870 ms — fails** |
| 1,000 | ~840 µs | **~4.2 s — fails badly** |

So cache-off `prime` clears its own gate today and fails it after Phase C at any realistic
operation count. **A2's green light is real and does not survive Phase C**, exactly as A3
argued. B1-as-reduction is settled, not a judgement call.

### A1 result (2026-10-19) — done, gate green, verified

`.jj/jjj-meta/.telemetry.jsonl`, one JSON object per line: `{ts, command, subcommand, ms}`.
Enabled only by `JJJ_TELEMETRY=1`; disabled costs **one `getenv`, 83 ns**, with no stat and
no file creation. Enabled costs ~120 µs, about 1% of a 26 ms invocation. 16 concurrent
`jjj` processes each append one intact line via a single `write_all` to an `O_APPEND` file —
no lock, so concurrent agents never serialise on it.

Independently verified rather than taken on report: `push.rs` copies a hard allowlist into a
separate `.jj/jjj-sync` workspace (the same mechanism that keeps `automation.toml` local),
`fetch.rs::classify` is an allowlist ending `_ => Target::Skip`, and `telemetry.rs` uses only
`ArgMatches::subcommand` — no `get_one`, `value_of` or `values_of` anywhere — so a title,
body, id or path cannot reach the file even in principle.

**One limitation that lands directly on A4:** an invocation clap *rejects* (misspelled
subcommand, missing argument) is never recorded, because clap prints and exits before `main`.
A command that parses and then fails *is* recorded. For A1's own purpose — which of ~102
leaf operations get used — that is harmless. **For A4 it is the opposite of harmless: a
rejected command is exactly the "what did the agent reach for" signal A4 exists to capture.**
A4 must collect those from its own harness, not from this file.

**A4 — seven-verb trial.** Run a swarm trial (see the August 2026 trial notes in project
memory for harness traps that cost whole runs) with agents told that only `prime`,
`next`, `query`, `create`, `edit`, `escalate`, `doctor` exist. *Acceptance:* a log of
every time an agent reached for something outside the seven, and what it wanted. Each
distinct reach becomes a Problem, not an automatic addition to the surface.

Two additions from A1 and A3, both mandatory:
- **Capture clap-rejected invocations from the harness**, not from `.telemetry.jsonl` —
  telemetry structurally cannot see them, and they are the most interesting rows.
- **Record operations per entity.** A3 produced the replay curve but nothing says where on
  it a swarm sits; observed usage is ~1 op/entity and neither live corpus has a `lamport`
  field at all. This single number decides whether C9's threshold is ever approached and
  whether B1's derived index is sized right. It is the largest remaining unknown in the plan.

### A3 result (2026-10-19) — ran; kill criterion did not fire; plan changed anyway

Apple M1 Ultra, release build, `tools/spike-oplog/`. Op mix 60% body edits, 3 ops/pack,
10% concurrent packs, 1,200 B mean body (swept 128 B and 8,000 B; conclusion unchanged).

| ops/entity | parse+apply p99 | +one object read per pack | on disk (compressed) |
|---:|---:|---:|---:|
| 10 | 0.007 ms | 0.08 ms | 3.2 KiB |
| 100 | 0.092 ms | 0.99 ms | 36 KiB |
| 1,000 | **0.92 ms** | **8.5 ms** | 354 KiB |
| 10,000 | 8.95 ms | 85.5 ms | 3.4 MiB |

**The kill criterion did not fire** — 9.1 ms at 1,000 ops against the 50 ms line, with
margin at every body size tried. C9 is therefore not required *by A3's stated criterion*.

Four findings that matter more than the verdict:

1. **"Replay is cheap" is true and irrelevant.** The fold is 77 ns/op; deserialising
   bytes into structs is 0.84 µs/op — 11× more. Consequence: **a snapshot must store
   materialised state, not a memoised fold**, or it saves nothing.
2. **Growth is exactly linear with no knee**, so there is no threshold to discover; it has
   to come from a budget. What degrades first is not latency but **size**: at 1,000 ops an
   entity's log is 354 KiB compressed against a 0.9 KiB final state — 1,090× amplification,
   which is 1.7 GB at 5,000 entities.
3. **The dominant cost is a C3 layout decision, not a C9 compaction question.** One git
   object per pack costs ~21 µs of open/read/close, which is 8× the parse cost. Reading
   packs from one object per entity, or a packfile, moves the 50 ms crossing from ~4,700
   ops to ~17,000. **Free if decided before C3 is written, expensive after.**
4. **The operating point is unmeasured and that is the real gap.** Observed usage is ~1
   op/entity — `minerva_old` has 46 events across 45 problems and neither live corpus has
   a `lamport` field at all. A3 gives the curve; nothing yet says where on it a swarm sits.
   **A4 must record ops/entity**; it is the number that converts this curve into a decision.

### Gate A — closed 2026-10-19. Three of four units ran; A4 still open.

| unit | kill criterion | result | fired? |
|---|---|---|---|
| A1 telemetry | disabled cost must be unmeasurable | 83 ns, one `getenv` | no |
| A2 cache-off | `prime` p50 < 500 ms @ 5,000 | 451 ms (p99 **604 ms**) | no |
| A3 op-log replay | parse+fold p99 < 50 ms @ 1,000 ops | 0.92 ms | no |
| A4 verb trial | — | **blocked**: needs a container fleet | — |

**No criterion fired, and the verdicts were the least useful thing the phase produced.**
Read only the three "no"s and B–H proceed as written; read the findings and four things
change. That asymmetry is a defect in how I wrote the gates, recorded here so Gate C is
written better:

- **A3's threshold measured the wrong budget.** 50 ms per entity is not the binding
  constraint; `prime`'s per-entity share is, and A2 puts that at ~100 µs — 500× tighter.
  A3 passed a line that was never the real one.
- **A2's threshold landed on the knee by coincidence.** ~82 µs/entity at 5,000 means the
  500 ms budget is crossed at 4,500–5,500 entities. The gate sat on the inflection by
  luck, not by insight, and the p99 in that same cell is already past it.

**The four changes Gate A forces on the later phases:**

1. **B1 becomes a reduction, not a deletion** (§3). Composing A2's ~82 µs/entity with
   A3's 0.84 µs/op predicts cache-off `prime` @ 5,000 at ~455 ms today, ~870 ms at 100
   ops/entity and ~4.2 s at 1,000. A2's green light does not survive Phase C.
2. **C3's layout decision is promoted ahead of C9's compaction.** One git object per pack
   costs ~21 µs open/read/close — 8× the parse — and moves the 50 ms crossing from
   ~17,000 ops to ~4,700. It is free to decide now and expensive later.
3. **A snapshot must store materialised state, not a memoised fold.** A2 and A3 found the
   same cost from opposite ends: replay is 77 ns/op, deserialisation is 0.84 µs/op, and
   emptying 75,004 event shards moved `prime` 2%. The cost is turning bytes into structs.
4. **Six DB dependencies enter Phase B's scope** (§3), two of them ordering bugs in this
   plan: `jjj push` runs `db validate` as a correctness gate, and `src/tui/app/related.rs`
   opens the DB directly while its removal is gated behind Gate D — leaving the related
   pane broken from B1 to G2.

**Two defects found by the units, outside their briefs, both fixed before the gate closed:**

- `scripts/gate.sh` ran `cargo fmt --all` immediately before `cargo fmt --all --check`, so
  the fmt stage repaired what it then measured. Same family as the piped-clippy incident
  the script exists to prevent. Hence §0 rule 2's new clause.
- `prime`'s escalation section was uncapped in both the JSON and text paths while every
  sibling used `.take(limit)` — a direct violation of D1's constant-size requirement, in
  code this plan's author shipped the previous session.
- `jjj plan apply` could create findings but had no `measured_at` field, and set no
  `author` on any finding or critique. Found by trying to carry out this gate's own
  closing instruction. A plan is the case where measurement time and recording time
  differ *most* — it is applied days after the numbers are taken — so the one entity
  kind whose whole point is "when was this measured" was the one that could not say.
  Fixed: the field is accepted and validated before any write, the parser is now shared
  with `finding new` so there is one error message, and both kinds carry an author.

**Before any later phase starts:** publish the three results as Findings with
`measured_at` — they are already written as `findings` in the companion
`2026-10-19-swarm-core.plan.json`, so applying that plan publishes them. **A4 remains the largest unknown** — it must record ops/entity (§2 A3
note 4) and capture clap-*rejected* invocations, which A1's telemetry cannot see.

---

## 3. Phase B — Cut the cache first

**Counter-intuitive but load-bearing ordering.** `CACHE_FAITHFUL` on `Persist` means
every model field change costs six coordinated steps: DDL, `SCHEMA_VERSION` bump,
`requires_rebuild` migration, upsert, positional decoder, `DbEntity::COLUMNS`. Phase C
rewrites the storage model wholesale. Building the op-log while the cache still mirrors
every field doubles the work and multiplies the chance of a silent decode drift. So the
cache goes **before** the substrate, not after.

> **Corrected 2026-10-19 after A3. Read this before taking B1.**
>
> B1 and Phase C are **not independent**, and the first draft of this plan treated them
> as if they were. An op-log makes a single entity read **88× more expensive at 1,000
> ops** than today's state-based read, crossing over at only ~10 ops/entity. A2 measures
> cache-off `prime` against *today's* markdown storage, so **A2's green light does not
> survive Phase C** — it describes a storage layer that Phase C removes.
>
> Worse, `prime` must read *every* entity to compute the ready front even though its
> output is constant in corpus size. Deriving a per-entity budget from A2's own kill line
> (500 ms ÷ 5,000 entities = 100 µs/entity) allows only **~34 ops/entity**. No plausible
> swarm operating point sits below 34 edits per problem. So per-entity snapshots (C9) do
> not make `prime` safe; `prime` needs a **corpus-level materialised read path**.
>
> That is a cache. Confirmed independently: a Rust reimplementation of git-bug *without*
> a cache layer lists 48,889 issues in ~5 s against **~400 ms for git-bug itself with its
> cache**, and its author attributes the gap to exactly that missing layer. **git-bug
> needs a cache because it has an op-log.** The design note recommended adopting the
> op-log *and* deleting the cache; those two recommendations undercut each other, and the
> evidence was inside the project being used as the model.
>
> **So B1 is no longer a deletion.** It is a reduction: strip `src/db/` to a minimal
> derived index — id, status, parent, lamport, assignee and a materialised body — and
> delete the rest (FTS, semantic search, the 16-column mirror, the migration ladder).
> Most of the LOC saving survives and so does most of the `CACHE_FAITHFUL` relief, since
> far fewer fields are mirrored. The index must be declared **derived and rebuildable**,
> never a source of truth, and Phase C owns keeping it in step.
>
> The effort argument for doing B before C still holds: a smaller mirror makes the Phase C
> rewrite cheaper. What changed is that the cache cannot be *deleted*, so the headline
> 8–10K estimate needs revisiting once B1 is scoped.

| id | unit | depends | parallel |
|---|---|---|---|
| B1 | **Reduce** `src/db/` to a minimal derived index (not delete) | Gate A | no |
| B2 | Remove `src/embeddings.rs` (495) | B1 | yes |
| B3 | Fold `src/commands/search.rs` (222) into `query` | B1 | yes |

*Acceptance for each:* `scripts/gate.sh` → `GATE_EXIT=0`; the per-release corpus test
added in `80ea24c` still loads every historical fixture; `jjj doctor` no longer reports
cache state and does not mention a database.

**Dependencies on the DB that this plan did not account for** (found by A2, all verified
against the code). B1 must own every one of them:

1. **`jjj search` has no fallback and silently becomes O(corpus) per invocation.** With no
   usable cache it calls `load_from_markdown` — a full rebuild — on *every* query. Measured
   521 ms at 2,000 entities; `search.rs`'s own comment puts it near 10 s at 25,000. It does
   not fail, it just goes quietly quadratic in a swarm. The plan named B3 only as "fold
   `search.rs` into `query`"; this is the real content of that unit.
2. **`critique list --search` and `finding list --search`** have the same hard dependency and
   were named nowhere in this plan.
3. **Semantic similarity (`jjj search p/<id>`) needs the `embeddings` table.** B2 removes
   `embeddings.rs` but nothing said this entry point goes with it.
4. **`jjj push` opens the DB to run `db validate`** (`commands/push.rs:456`) — the
   pre-publish dangling-reference check, which `schema.rs` calls the right place to catch
   them. **B1 removes a correctness gate from `push`**, and the plan did not mention `push`
   having a DB dependency at all. Either the check moves to the filesystem path or `push`
   loses it knowingly; it must not be lost by accident.
5. **`src/tui/app/related.rs` opens the DB directly for `find_similar`.** The TUI is deleted
   in G2, which is gated on Gate D — many phases after B1. **Between B1 and G2 the related
   pane is broken.** A dependency-ordering gap, not a design problem: either G2 moves ahead
   of B1 or the pane is stubbed in B1.
6. **These paths recreate the database even with `JJJ_NO_DB=1` set** — confirmed
   empirically: `problem list --search` with no `.jj/jjj.db` and the bypass set created the
   file. So "cache off" is not reachable by an env var alone, and **B1 has to remove the
   call sites**, not merely gate them.

Working cache-off with filesystem fallbacks, verified fast (32–121 ms at 2,000): `events`,
`insights`, `timeline`, `contention`, `status`, `conflicts`, and the derived `solution_ids`
back-reference.

*The bypass switch itself* (`JJJ_NO_DB=1`, 36 lines via `db::bypassed()`, with `jjj doctor`
warning when it is set) is **read-path only** — a write taken with it set skips the cache
upsert and leaves an existing `.jj/jjj.db` stale. It is a measurement switch, not a
supported mode. It dies with `src/db/` if B1 proceeds; if B1 is deferred it is what makes
"is the index still earning its keep?" answerable at any time instead of once.

---

## 4. Phase C — The op-log substrate. The keystone.

Modelled on git-bug's verified data model (design note §3). Mostly sequential; C7 is
deliberately assigned to a **different agent** than C1–C5.

| id | unit | depends | parallel |
|---|---|---|---|
| C1 | `Operation` + `OperationPack` types, JSON serialization | B1 | — |
| C2 | Lamport handling + the DAG clock invariant | C1 | with C3 |
| C3 | **Decide pack layout**, then store packs on the `jjj` bookmark | C1 | with C2 |
| C4 | Deterministic replay; tie-break by pack id | C2, C3 | — |
| C5 | Conflicted value as a first-class state | C4 | — |
| C6 | Migration from state-based `.md` files | C4 | with C5 |
| C7 | **Adversarial property tests** | C1 (spec only) | yes — different agent |
| C8 | Delete `src/storage/merge.rs` (1,697) and `pick_side` | C4, C7 | — |
| C9 | Materialised snapshots + compaction | C4 | — |

**C2 — the invariant that jjj currently lacks.** git-bug refuses any commit whose Lamport
clock is not lower than its child's, which makes clock abuse a structural impossibility.
jjj currently caps witnessing at a 1,000,000 jump, which is a heuristic. Implement the
structural check and keep the cap as a second line. *Acceptance:* a test that a pack
claiming a clock out of DAG order is refused, not clamped.

**C3 — decide the layout before writing the code. A3 found this is the largest free
lever in the phase.** One git object per pack puts ~21 µs of open/read/close in front of
every pack, which is 8× the parse cost and moves the 50 ms crossing from ~17,000 ops down
to ~4,700. The write side has the same variable and is unmeasured: 1,000 ops/entity at 3
ops/pack is 333 new git objects per entity — 17,000 across the real 52-entity corpus and
1.7M at 5,000 entities, which is plausibly the same thing as the open sync-delta latency
item in the bench memory. *Acceptance:* a written decision among one-object-per-pack,
one-object-per-entity and packfile, with the read and write cost of each stated, made
**before** any storage code lands.

**C9 — no longer conditional, and reframed.** A3's kill criterion did not fire, so C9 is
not required to unblock C8. It is required for two other reasons A3 surfaced: on-disk
growth is 1,090× at 1,000 ops/entity (1.7 GB at 5,000 entities), and `prime` reads every
entity, which no per-entity threshold fixes. **A snapshot must store materialised state,
not a memoised fold** — the fold is 77 ns/op and free; deserialisation is 0.84 µs/op and
is the entire cost. Recommended per-entity threshold 1,000 ops, which keeps every layer
≥4.7× under the 50 ms line on this hardware and so survives a 2–3× slower CI runner.
C9 and B1's derived index are the same problem seen from two ends; whoever takes either
should read both.

**C5 — the synthesis of both founding insights.** When two clones edit the same field
concurrently, the result is **not** a winner. It is a conflicted value that: appears in
`prime`, is queryable (`query 'conflicted:true'`), can be adjudicated later by any
principal with an argument, and escalates if nobody adjudicates it. This is what jj does
with trees and what git-bug does with operations. *Acceptance:* a scenario test where two
clones edit one body, both sync, and neither body is lost or silently chosen; the problem
appears on the ready front as needing adjudication.

**C7 — written against the spec, not the implementation, by an agent that has not read
C1–C5's code.** Three properties:
- *Convergence:* for any permutation of merge order over the same operation set, the
  replayed state is identical.
- *No loss:* every operation ever written is recoverable from the log alone, with no
  reference to jj history. This is the property `pick_side` violates today and is the
  whole reason for the phase.
- *Clock abuse:* a pack with a fabricated clock cannot reorder another principal's
  operations.

This unit is the gate for the entire phase. If C7 cannot be made to pass, C8 does not run
and `merge.rs` stays.

**C6 — migration.** Existing `.md` frontmatter becomes a single synthetic "import"
operation per entity, carrying the current state and the existing `lamport` value.
*Acceptance:* every fixture in the release corpus migrates and round-trips; a
`jjj doctor` check reports any entity that failed to migrate rather than skipping it.

**Gate C:** C7 green. `GATE_EXIT=0`. Release corpus round-trips.

---

## 5. Phase D — The seven verbs

Highly parallel — seven agents — but D0 blocks all of them.

| id | unit | depends | parallel |
|---|---|---|---|
| D0 | **Output contract spec** | Gate C | no |
| D1 | `prime` — constant-size output | D0 | yes |
| D2 | `next` — claim, lanes | D0 | yes |
| D3 | `query` — one grammar, three kinds | D0 | yes |
| D4 | `create` — single + `--plan` | D0 | yes |
| D5 | `edit` — `--if-lamport`, exit 13 | D0 | yes |
| D6 | `escalate` — with `--reply` | D0 | yes |
| D7 | `doctor` — read-only, exits nonzero on fault | D0 | yes |
| D8 | Collapse transitions into `edit --status` | D1–D7 | no |

**D0 — the output contract.** One markdown shape per verb that reads well to a human and
parses deterministically, plus `--json` on `prime`, `query`, `next`, `create` only. The
other 49 `--json` flags go. *Acceptance:* a written contract document and a parity test
harness asserting that for any entity, the markdown and the JSON carry the same facts.
This is what prevents a recurrence of the bug fixed in `1405f17`, where `--json`
returned less than the text path.

**D1 — `prime` is the hard one.** Output must be constant in corpus size, because the
binding constraint on an agent is context window. *Acceptance:* `prime` output byte length
varies by less than 2× between a 50-entity and a 25,000-entity corpus.

**One violation already existed and is fixed (2026-10-19).** A2 found that a corpus with
132 open escalations printed all 132. The escalation section was uncapped in *both* paths —
`prime.rs:110` emitted the whole vector in JSON while every other key used `.take(limit)`,
and the text path iterated all of them instead of going through `section()`. Now capped in
both, with the true total in the heading, an `escalations` key added to `counts`, and
`… and N more` matching every other section. `--full` still shows every one. D1 should add
the byte-length test so this cannot regress silently.

**D8 — transition collapse.** `solution submit`/`approve`/`withdraw`, `problem reopen`,
`critique dismiss` and the rest become `edit --status`. The guards and events must come
from the shared domain layer, exactly as `plan.rs`'s `actions` already do — the comment at
`src/commands/plan.rs:65` explains why a bulk path that sets `status` directly is a hole,
and names beads as having exactly that hole. Do not reintroduce it.

**Gate D:** parity harness green across all seven verbs. ~102 leaf operations down to
~20, verified by counting.

---

## 6. Phase E — Three nouns

Data migration. Sequential with C6.

| id | unit | depends | parallel |
|---|---|---|---|
| E1 | Finding → Problem body + `--cites` on any kind | Gate C | no |
| E2 | **Milestone: survives iff it gains a decline list** | E1 | no |
| E3 | *Optional:* rename Solution→Attempt, Critique→Objection | E2 | no |
| E4 | Divergence detection across principals | Gate C | yes |

**E1.** Zero findings exist outside tests. The mechanism worth keeping is `--cites`, which
becomes a frontmatter list available on all kinds. The `measured_at` field added in
`27e44e2` moves with it. *Acceptance:* every finding in the release corpus becomes a cited
body with no content lost; `jjj query 'cites:<id>'` returns what `finding list --problem`
used to.

**First, a framing correction.** An earlier version of this plan split E2 and E4 on
*solo versus a group of humans*. That is the wrong axis. The design splits on
**single-principal versus multi-principal**, and the moment a swarm exists jjj is
multi-principal regardless of the human count — one human plus twelve agents is thirteen
principals with identities, claims, concurrent writes and conflicting opinions. Every
expensive thing (identities, claims, Lamport, lossless merge, conflicted values,
escalation) is on that side and is required either way. The human count does not reach
Phase C at all.

What genuinely changes with more than one human is small, additive and deferrable:
escalation routing ("a human" → "which human", mostly already carried by lanes);
human-versus-human authority deadlock (resolves out of band — document it as deliberately
out of scope rather than discovering it as a bug); and commit signing becoming worth
having, since git-bug signs so that clock claims are attributable. None of it blocks
anything below.

**E2 — Milestone survives iff it gains a decline list.** The earlier cut to a tag was
derived from solo usage data (zero milestones in `minerva_old`, four in `testjjj`), but
that data shows jjj used as a flat list by someone not steering a swarm — it does not
speak to this question either way. Keeping a fifth kind is *not* free: it must migrate
into the op-log (C6), render in the panel (F3), appear in `query`, and be taught in
SKILL.md (H1). "It might be useful" is exactly the reasoning this plan exists to stop,
so the test is not a guess about future headcount:

> The kind survives **if and only if** it becomes a commitment that records what is being
> **declined** — "this window, these problems, and explicitly not those" — which a tag
> cannot carry and which the swarm reads. If it stays what `CLAUDE.md` describes today,
> "time-based goals grouping problems", it is a tag with extra steps and it goes.

The decline list is the Popperian half: a commitment that does not say what it declines is
not falsifiable, and it is the part that actually stops a swarm drifting into work that was
decided against. A solo operator has no colleague to catch that drift, so this is not a
team feature. *Acceptance:* either the kind ships with a decline list that `prime` surfaces
to agents, or it is migrated to a tag — not left as it is.

**E4 — divergence detection, not aggregation. Unconditional.** `rg -i
'variance|divergen|disagree' src/ranking/ src/commands/rank.rs` returns **nothing**: the
ranking system computes a global aggregate and never surfaces where principals differ.
That is backwards. Deciding what to work on is out of band — a meeting, a doc, an argument,
someone's judgement — and the aggregate is the part that pretends to decide. What a tool
can do that the conversation cannot is arrive with the agenda already computed: **which
problems are ranked most differently, by whom.**

This is a multi-*principal* feature, so it is always live. With several humans it names what
they disagree about. With one human and a swarm it names **where the agents disagree with
you**, which is exactly where scarce human attention belongs. Same code, different
population compared.

Per-principal orderings already exist at `rankings/{milestone_id}/{user}.json`. Build a
rank-variance across principals, sorted descending, exposed as `jjj query 'contested:true'`
and a region in the F3 panel. Roughly 50 lines, against the 1,293 that G1 extracts.
*Acceptance:* given three orderings it names what they disagree about most and stays silent
about what they agree on.

**The choice this phase actually faces is aggregation versus divergence, not solo versus
group.** G1 extracts the 1,293 lines either way; E4 builds the 50 either way.

**E3 is churn and is optional.** The names Attempt and Objection describe the semantics
better than Solution and Critique, and Objection in particular makes the veto reading
obvious. But it touches every file, every test and SKILL.md. Take it only if Phases A–D
came in under budget. If skipped, say so in the changelog rather than leaving the design
note and the code disagreeing.

---

## 7. Phase F — MCP server and the App panel

Can start as soon as D0 lands; must not merge before Gate D.

| id | unit | depends | parallel |
|---|---|---|---|
| F1 | `jjj mcp serve` — 7 tools, `visibility`-scoped | D0 | — |
| F2 | Error envelope with a retryable flag | F1 | — |
| F3 | `ui://` panel — four regions | F1 | with F2 |
| F4 | App-only tools | F1, F3 | — |
| F5 | Stateless-per-call invariant + test | F1 | yes |

**F1 — visibility is the point, not the transport.** `visibility: ["model","app"]` on the
five shared verbs; `visibility: ["app"]` on the human-only acts. Per the MCP Apps spec a
host **MUST reject** a model-initiated `tools/call` for a tool lacking `"app"`, which makes
the human-authority asymmetry host-enforced rather than an honour system in jjj's own
code. Do not also implement an `is_human` policy check — that was the superseded design.

**F2.** Exit 13 has no carrier in an MCP result. Carry it as a `retryable: true` field the
panel renders as "someone moved this, reload". Scope the envelope to retryable vs not —
nothing more.

**F3.** One `ui://` resource, mime `text/html;profile=mcp-app`, four regions: ready front,
who is on what, open objections, escalations waiting. Zero external domains, so the
default `default-src 'none'` CSP needs no relaxation. The markdown output from D0 is the
required text fallback. **State persistence is undefined in the MCP Apps MVP** — do not
rely on filters or scroll surviving teardown.

**F4.** Answer an escalation; file an objection; approve/reject an attempt. Only these.
Per the spec, do not put `resourceUri` on a tool with no panel — a UI-linked tool that
returns only text renders an empty, never-resolving loading box.

**F5 — the invariant that keeps Phase B honest.** An MCP server is long-lived; jjj is
one-shot. The server must re-read from disk on every call or it drifts from what other
agents and `jj git fetch` have done. *Acceptance:* a test that an external write between
two `tools/call`s is visible to the second, and a `scripts/check-invariants.sh` rule that
`src/mcp/` holds no cached entity state across calls.

**Also required:** the panel does not render in a terminal. Verified host list is Claude
web and desktop, Goose, VS Code Insiders and ChatGPT; Antigravity has only said it plans
to explore support, and ACP/Zed does not ship App rendering. **The CLI stays** as the
substrate all surfaces drive and the fallback where no panel exists. Any unit that treats
the panel as the only interface is wrong.

---

## 8. Phase G — Deletions

Each independent, each parallel, each gated on Gate D. One commit per unit.

| id | unit | LOC |
|---|---|---|
| G1 | **Extract** `src/ranking/` + `rank.rs` — do not delete | 1,293 |
| G2 | Remove `src/tui/` | 6,790 |
| G3 | Remove `vscode/` | — |
| G4 | Remove `src/sync/` (GitHub) | 1,878 |
| G5 | Remove `src/automation.rs` | 585 |

**G1 is an extraction, not a deletion.** The budget normalisation
(`B = max(100, 2*problem_count)`, so ranking 10 items carries the same influence as
ranking 200 — test at `src/ranking/scoring.rs:182`) is correct work that solves a real
problem, and it becomes right again the moment there are multiple humans. Move it out of
the swarm core, keep `docs/design/latent-preference-ranking.md`, and record in the
changelog that it can return as an optional layer. Design note §6 has the full argument
for why a swarm should not vote: sybil, correlated votes, and the fact that voting is a
justificationist move inside a falsificationist tool.

What is wanted instead is E4's divergence detection: the extraction removes an aggregate
nobody used, and E4 adds the disagreement signal that was never there. This holds however
many humans there are — the aggregate is the part that pretends to decide, and deciding is
out of band in both cases.

**G2 — the justification, in one line:** `rg 'crate::commands::' src/tui/` returns zero
matches while `MetadataStore`/`save_*` appear 8× in `app/actions/edit.rs`, 7× in
`lifecycle.rs`, 7× in `create.rs` and 6× in `app/mod.rs`, and `next_actions.rs:68`
computes its own blocking logic. It is a second implementation of the model, not a view.
F3 replaces the interface part.

**G5 is a security deletion.** Automation rules produced the RCE fixed in 0.5.1. The
swarm's automation is agents polling `next`, which is strictly more capable with no
shell-template surface. Leave `jjj automation migrate` in place for one release so an
existing `.jj/jjj-meta/automation.toml` is reported rather than silently ignored.

---

## 9. Phase H — Consolidate

| id | unit | depends |
|---|---|---|
| H1 | Rewrite `skills/jjj/SKILL.md` for the seven verbs | Gates D, F; all of G |
| H2 | New rules in `scripts/check-invariants.sh` | G2, F5 |
| H3 | `CHANGELOG.md` + version bump to 0.6.0 | H1, H2 |

**H1 is last on purpose** — `include_str!` means every edit rebuilds the crate and 56 test
binaries, and there is a drift test. One pass, at the end.

Queued for H1 by completed units:
- `CLAUDE.md`: add `.telemetry.jsonl` to the machine-local, never-synced list under
  `.jj/jjj-meta/` (which currently names `automation.toml`, `.sync_state.json`,
  `.events_offsets.json`, `.lamport`, `jjj.db`).
- `CHANGELOG.md`: opt-in command telemetry — `JJJ_TELEMETRY=1`, machine-local,
  command/subcommand names only.
- `skills/jjj/SKILL.md`: if A4's agents run with telemetry on, one line saying to export
  `JJJ_TELEMETRY=1` and that the file never syncs.
- Document that a clap-rejected invocation is not counted, so a histogram undercounts
  misspellings.

**H2 — encode the lessons as checks**, so the next swarm cannot repeat them:
- no file outside the op-log layer writes storage directly (the Phase G2 bug class);
- no gate step is piped through `rg`;
- `exit_code()` returns 13 only for `Precondition`;
- `src/mcp/` caches no entity state across calls.

**H3.** This is a breaking release. Say plainly what was removed, that ranking is extracted
rather than gone, and that `--if-lamport`'s guarantee changed: with the op-log a lost race
no longer discards the loser, so the flag's documented limitation from `401ec92` is
narrower. Do not tag. Do not publish.

---

## 10. Critical path

```
Gate A ──▶ B1 ──▶ C1 ──▶ C2 ─┐
                       └ C3 ─┴▶ C4 ──▶ C7 ──▶ C8 ──▶ D0 ──▶ D1..D7 ──▶ D8 ──▶ H1 ──▶ H3
                                                                  └──▶ F1..F5 ─┘
```

Everything in Phase G hangs off Gate D and runs in parallel. Phase E hangs off Gate C.
**C4 → C7 → C8 is the bottleneck**, and C7 is an adversarial unit that may legitimately
refuse to pass — budget for that rather than scheduling around it.

---

## 11. Risk register

| risk | detection | kill criterion / response |
|---|---|---|
| ~~Op-log replay cost grows with op count~~ | A3 | **RESOLVED 2026-10-19: criterion did not fire** (0.92 ms parse+fold at 1,000 ops). Replaced by the three rows below. |
| **Deserialisation, not replay, is the cost** — 0.84 µs/op vs 77 ns/op | A3 | snapshots must hold materialised state, not a memoised fold |
| **`prime` reads every entity; the op-log makes each read op-proportional** | A3 §4.3 | a corpus-level derived index is mandatory, not optional — B1 reduces rather than deletes |
| **On-disk growth is 1,090× at 1,000 ops/entity** (1.7 GB at 5,000) | A3 | C9 promoted from conditional to required |
| Pack layout chosen implicitly by C3 | A3 §4.2 | decide explicitly first; 4,700 vs 17,000 ops of headroom rides on it |
| **A2's result does not transfer across Phase C** | A3 §4.3 | re-gate B1's sizing after Gate C, or commit to the derived index up front (chosen) |
| Cutting `src/db/` makes `prime` too slow | A2 | >500ms at 5,000 entities → B1 does not run; keep a rebuildable index |
| C7 convergence cannot be proven | C7 | C8 does not run; `merge.rs` stays; the phase is a partial win, not a failure |
| Migration loses content | C6, E1 | release corpus round-trip test; `doctor` reports failures rather than skipping |
| The panel is unavailable in the terminal | known now | CLI stays as the primary surface where the user actually works |
| MCP Apps MVP has no state persistence | known now | accept, or store view state server-side — do not discover this late |
| Keeping a kind on "might be useful" grounds | E2 | the decline-list test decides it from the code, not from a headcount guess |
| A swarm builds for usage that does not exist | A4 | each reach outside the seven verbs becomes a Problem, not an automatic feature |
| Commits that only compile in aggregate | `verify-commits.sh` | unit rejected, rebased |

`scripts/verify-commits.sh` (compile every commit on the branch in a throwaway worktree)
**now exists** — A3 reported that §0 rule 1 cited a script that was not in `scripts/`.
Added 2026-10-19; takes an optional base ref, defaults to `origin/main`.

---

## 12. Load this plan into jjj

Dogfood it. `docs/plans/2026-10-19-swarm-core.plan.json` next to this file is a
`jjj plan apply` document that creates the phase problems and their children, so the
swarm takes work from `jjj next --lane <lane>` instead of from this markdown.

```sh
jjj plan apply docs/plans/2026-10-19-swarm-core.plan.json --dry-run   # inspect first
jjj plan apply docs/plans/2026-10-19-swarm-core.plan.json
```

This is also the first real test of `plan apply` (landed in `3219688`) and of the ready
front: the phase problems are parents, so they will not be offered until their children
are done, and `jjj query waiting` should list exactly the phases whose units are open.

**Note:** jjj has never tracked its own development — there is no `jjj` bookmark on this
repository. Running this plan through jjj means creating one, which is itself worth doing
and will surface anything that only breaks on a real corpus.
