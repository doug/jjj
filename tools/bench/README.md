# Bench — repeatable harness & M0 scale probes

Validation tooling for the agent-swarm scaling work (see
`docs/design/scaling-for-agent-swarms.md`).

## Repeatable harness (`bench.py`) — the release gate

Times real jjj commands end-to-end against a generated corpus so regressions
in the read/write/sync paths show up before a release:

```bash
cargo build --release            # harness uses target/release/jjj
python3 bench.py                 # quick run: 2K corpus, includes sync benches
python3 bench.py --count 25000 --json baseline-25k.json   # release gate
```

Covers the design's bench matrix: cold list (FS walk), `db rebuild`, warm
list/status/next, FTS search, events listing (cold ingest + warm DB-primary),
write throughput under N concurrent `JJJ_POD` writers, and cold push / cold
fetch / warm 100-file delta-fetch against a local bare remote. Everything runs
in an isolated tempdir with its own jj/git identity. `--json` records results
plus the jjj git rev for comparison across runs.

Before cutting a release: run the 25K gate on a quiet machine and compare
against the baseline below; flag anything that moved >2x.

### Recorded baseline — 25K corpus (2026-07-25, M2 Mac, rev fa71165)

| bench | median |
|---|---|
| cold_list (no DB, FS walk) | 1.06s |
| db_rebuild | 8.1s |
| warm_list (DB) | 0.096s |
| status | 0.136s |
| next_top5 | 0.109s |
| search_fts | 0.078s |
| events_cold_ingest | 1.77s |
| events_warm (DB-primary) | 0.054s |
| write_throughput (10 pods × 5) | 2.07s (~24 ops/s incl. process spawn) |
| cold_push (full corpus) | 18.3s |
| cold_fetch (full corpus) | 8.4s |
| delta_push (100 files) | 12.5s |
| warm_delta_fetch (100 files) | 6.9s |

All DB-backed reads meet the design's <200ms @ 25K acceptance. **Known
gap:** sync (`delta_push` / `warm_delta_fetch`) is far above the design's
sub-second target — the first run of this harness caught two O(n²)/O(n)
read regressions (fixed in eeacd0f, e752095); the sync latencies are the
remaining open finding.

## Cache on vs off (`cache_off.py`) — can `src/db/` be deleted?

Answers one question with numbers: the SQLite cache is documented as a
disposable index over the canonical markdown, so is it still load-bearing?

```bash
cargo build --release
python3 cache_off.py                                  # 50/500/5000/25000, 100 reps
python3 cache_off.py --counts 5000 --reps 200          # just the gate cell
python3 cache_off.py --counts 5000 --events-per-entity 0   # isolate the event log
```

Cache-off is taken with `JJJ_NO_DB=1` (`crate::db::bypassed`) **and** the
`.jj/jjj.db` file removed, so the arm shows a world where `src/db/` does not
exist rather than one where it merely sat idle. The corpus is mixed across all
five kinds with varied body lengths, tag counts and a parent/child forest —
`bench.py`'s flat corpus of identical stubs flatters the markdown walk.

### Recorded result — 2026-10-19, M2 Mac (load avg ~4-5), rev 401ec92, 100 reps

| command | corpus | cache on p50 / p99 | cache off p50 / p99 | off/on |
|---|---|---|---|---|
| `prime` | 50 | 38.8 / 42.7 ms | 40.8 / 43.5 ms | 1.05x |
| `prime` | 500 | 41.7 / 44.7 ms | 76.8 / 81.3 ms | 1.84x |
| `prime` | 5,000 | 87.0 / 94.5 ms | **451.4 / 604.3 ms** | 5.19x |
| `prime` | 25,000 | 856.3 / 931.8 ms | 2884.3 / 3384.7 ms | 3.37x |
| `query` | 50 | 27.8 / 32.2 ms | 33.8 / 36.9 ms | 1.22x |
| `query` | 500 | 29.7 / 32.6 ms | 92.2 / 95.9 ms | 3.10x |
| `query` | 5,000 | 54.3 / 61.5 ms | 668.8 / 726.1 ms | 12.32x |
| `query` | 25,000 | 182.6 / 190.5 ms | 3423.4 / 4041.4 ms | 18.75x |
| `next` | 50 | 38.6 / 43.4 ms | 40.9 / 44.3 ms | 1.06x |
| `next` | 500 | 41.6 / 46.5 ms | 75.2 / 78.5 ms | 1.81x |
| `next` | 5,000 | 85.5 / 90.9 ms | 431.7 / 459.5 ms | 5.05x |
| `next` | 25,000 | 838.7 / 888.6 ms | 2709.9 / 3129.8 ms | 3.23x |

`db rebuild` for reference: 0.06s @50, 0.25s @500, 2.19s @5K, 10.09s @25K.
Full output in `baseline-cache-off.json`.

**The plan's kill criterion (cache-off `prime` over 500ms at 5,000 entities) did
not fire, by 11%** — 451ms, reproduced at 452ms over 200 reps. It is a pass, not
a comfortable one: the p99 of that same cell is 604ms, already past the budget.

**What drives it is the YAML parse, not the event log.** Re-running 5,000 and
25,000 with the event shards emptied (75,004 events → 4) moved `prime` by 2% and
0% respectively. The cost is one `parse_frontmatter` per entity file at roughly
90–115µs, linear in entity count, which puts a 500ms `prime` budget at about
5,000 entities — exactly where the gate was set.

**Two things the ratio column hides.** Below 500 entities the cache is worth
under 40ms, and the real observed corpus is 52. At 25,000 `prime` misses 500ms
*with* the cache (856ms), so the cache is not what makes `prime` fast at that
scale — its whole value window is a corpus between roughly 2,000 and 25,000
entities that nobody has.

**Not measured here, and not fixed:** `search`, `problem list --search` and the
other `--search` flags have no filesystem fallback. With `JJJ_NO_DB=1` they
still create and populate `.jj/jjj.db` (the bypass is read-path only), and
`jjj search` falls into a full `load_from_markdown` per invocation. `push` also
opens the DB to run `db validate` before publishing. Those paths are B3's
problem; this harness only records that they exist.

## Sync scaling (`sync_scaling.py`) — the open finding

Decision 3 makes sub-second `jjj sync` a **hard** requirement, and it is violated
at 25K. Profiling shows the cause is not where it was assumed to be:

    delta_push @25K:  11,898ms total — 1,367ms in jj (12 calls), 10,531ms in jjj

88% of the time is **jjj's own work**. The subprocess count is already near
minimal — 12 calls for a push, 7 for a fetch — so this is not a batching problem.
Holding the delta at 100 files and growing the corpus shows what it is:

| corpus | jjj's own time (push) | jj's time |
|---|---|---|
| 2,000 | 1,182ms | 932ms |
| 8,000 | 3,244ms | 972ms |
| 25,000 | **10,401ms** | 1,395ms |

Linear in *corpus*, flat in *delta*: **O(total) work for an O(delta) operation**.
That is Break #1, which Pillar 1 was meant to eliminate — the jj-side delta work
is correct, jjj's own paths are not. M0 validated jj's primitives; nobody
profiled jjj.

```bash
python3 sync_scaling.py                              # 2K vs 25K
python3 sync_scaling.py --small 1000 --large 4000    # a faster loop
```

**The score is a ratio, deliberately:** `jjj_ms(large) / jjj_ms(small)`, ~8.8x
today, approaching 1.0 as the work becomes delta-proportional. Absolute timings
are meaningless on a machine saturated by a swarm; a ratio of two measurements
taken under the same load survives it.

## M0 probes (historical validation gate)

M0's job was to **get numbers on the design's riskiest assumptions before
writing M1** — these probes measure raw jj primitives, not jjj itself.

## Build

```bash
rustc -O -o gen_corpus gen_corpus.rs     # standalone corpus generator
```

`gen_corpus <out_dir> <count> <flat|fanout>` writes `<count>` realistic problem
entity files (YAML frontmatter + body, ~500B) under `<out_dir>/problems/`.
`fanout` shards as `problems/{ab}/{cd}/{id}.md`. Ids are deterministic.

## Probes (the must-validate gate)

Each probe answers one question that could invalidate the design.

| Probe | Question | Run |
|---|---|---|
| `probe1_treediff.py` | Is `jj diff --from --to --name-only` sub-second between two 25K/100K-file revisions with a small delta? (Pillar 1 keystone) | `python3 probe1_treediff.py [counts...]` |
| `probe1b_fetch.py` | Fetching the delta's *content*: loop `jj file show` per file vs one batched call vs one `diff --git`. (Refines Pillar 1 — the per-file cost Probe 1 surfaced) | `python3 probe1b_fetch.py [count] [K...]` |
| `probe2_push_contention.py` | Under N pods pushing one `jjj` bookmark, how many retries to drain, and wall time? (Break #5) | `python3 probe2_push_contention.py [N...]` |

Probe 1/1b isolate **pure jj tree/content cost**; Probe 2 isolates **ref-race
serialization** on a local bare remote (no WAN latency — that's separately
additive). The third must-validate item — **read-your-writes** (Pillar 2 reads
from the DB vs Pillar 5 lets the DB lag) — is a *policy decision verified by a
unit test when Pillar 2 is built*, not a standalone measurement, so it has no
probe here.

## Results

See `docs/design/scaling-for-agent-swarms.md` "M0 findings" for the recorded
numbers and the design changes they forced.

## Notes

- Probes use isolated `HOME`/`JJ_CONFIG`/git identity so they don't touch your
  config and don't depend on it.
- Generating + committing 100K files in jj takes ~30–40s of setup per case;
  the 100K runs take a few minutes total.
