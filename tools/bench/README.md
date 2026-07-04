# M0 — scale probes & bench harness

Validation tooling for the agent-swarm scaling work (see
`docs/design/scaling-for-agent-swarms.md`). M0's job: **get numbers on the
design's riskiest assumptions before writing M1.**

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
