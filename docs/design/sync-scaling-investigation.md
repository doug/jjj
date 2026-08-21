---
title: Sync scaling investigation — corpus-proportional ops in push and fetch
description: Root cause for the O(corpus) instead of O(delta) cost measured by tools/bench/sync_scaling.py
---

# Sync scaling investigation

Decision 3 in `docs/design/scaling-for-agent-swarms.md` requires sub-second
`jjj sync` for a small delta, regardless of corpus size. Below are the two
concrete code paths responsible, found by reading `src/commands/push.rs` and
`src/commands/fetch.rs` against `tools/bench/sync_scaling.py`.

Measured with `tools/bench/sync_scaling.py` (CPU time, 500 vs 5,000 entities,
50-file delta):

| | before | now | |
|---|---|---|---|
| fetch | 2.0x | **1.0x** | O(delta) — done |
| push | 2.8x | **1.5x** | both O(corpus) steps fixed |

**Fetch is fixed.** It used to leave `last_synced_rev` at `None` for any pod
that only ever fetched, so every fetch paid a full cold-start reconcile.

**Push had two O(corpus) steps, and both are now fixed.**

1. *Validation's markdown reload.* A `content_cache` of per-file content hashes
   means only changed files are parsed and re-validated. Nothing is skipped on
   trust: a changed file always misses the cache and is checked in full.
2. *The copy into the sync workspace.* It used to delete and rewrite every
   `.md` file on every push. Now a file is written only when its bytes actually
   differ — compared live, not via an `mtime`/size proxy that a
   timestamp-preserving copy can defeat. Leaving an unchanged file's mtime alone
   is what lets jj's own working-copy snapshot skip rehashing it, which is where
   most of the cost was.

**Is there a third bottleneck?** No. Checked directly, by applying the
copy-step fix and re-measuring on the same machine in the same turn: push went
from 2.18x to 1.10x — within run-to-run noise of 1.0x, not a residual multiple
of it. The `jj` subprocess calls stay flat in corpus size (12 calls), because
`sync_meta_to_bookmark` never asks jj to diff or hash the corpus; jj's
auto-snapshot only has to notice the handful of files jjj actually touched.
That is precisely why removing the unconditional rewrite closes the gap rather
than moving the cost into jj.

The push analysis below is accurate about *where*
the cost is, but the obvious shortcut — skipping `load_from_markdown` when the
SQLite cache is clean — is **wrong and was rejected**: the dirty flag means "a
sync was interrupted", not "the markdown has not changed since the cache was
written". Markdown written outside jjj (a `git merge`, a hand edit) leaves the
cache clean but stale, and validating the stale cache lets a dangling reference
or a conflict-marked body through to every clone. `durability_test.rs::a_dangling_reference_is_still_refused_at_push`
and `push_fetch_test.rs::test_body_conflict_blocks_push` both catch it.

Making push delta-proportional means making *validation itself* incremental —
reload only entity files whose mtime or hash changed since the cache was
written, then validate — not skipping the gate.

## Push: two unconditional full-corpus passes, every push

`src/commands/push.rs::execute` always runs, regardless of how many files
changed:

1. **Full markdown → SQLite reload for validation** (`push.rs:423`):
   ```rust
   db::load_from_markdown(&db, store)?;
   ```
   `load_from_markdown` (`src/db/sync.rs:21`) clears every table and calls
   `store.list_fs::<Problem>()` / `Solution` / `Critique` / `Milestone`
   (`src/db/sync.rs:42,48,54,60`) — each of which walks and parses **every**
   markdown file in the corpus — then rebuilds the FTS index over all of them
   (`rebuild_fts`, `src/db/sync.rs:108`). None of this is scoped to the files
   that changed since the last push.

2. **Full directory copy into the sync workspace** (`sync_meta_to_bookmark`,
   `push.rs:201-226`): for each of `problems/`, `solutions/`, `critiques/`,
   `milestones/`, it `read_dir`s the destination and removes every `.md` file
   it owns, then `read_dir`s the source and copies every `.md` file back
   (`push.rs:211-224`). This is O(corpus) file removals + copies per push, not
   O(delta) — a corpus of 25K entities does ~50K filesystem operations to push
   a single changed file.

Both are unconditional: there is no filtering by "changed since
`last_synced_rev`" anywhere in this path. This matches the profile in
`SWARM.md` exactly — push's jjj-own-time is flat in the delta and linear in
the corpus.

## Fetch: `last_synced_rev` only advances on push, never on fetch

`src/commands/fetch.rs` decides cold-start vs. delta with:

```rust
let state = SyncState::load(&meta_path);
let cold_start = match state.last_synced_rev.as_deref() {
    None => true,
    Some(rev) => jj_client.resolve_commit(rev)?.is_none(),
};
```
(`fetch.rs:353-357`)

`SyncState::advance` is documented as "the ONLY way to move
`last_synced_rev`" (`src/storage/sync_state.rs:136-139`), and it is called
from exactly one place in the whole codebase:

```
$ grep -rn '\.advance(' src/
src/commands/push.rs:479:    state.advance(pushed_commit);
```

**Fetch never advances its own merge-base pointer.** So any pod whose most
recent successful sync operation was a fetch — not a push — has
`last_synced_rev == None` and is permanently treated as a cold start:

- `apply_file_delta`'s base becomes `root()` for every head (`fetch.rs:367-368`),
  so `delta_git` returns full content for every file that has ever existed,
  not just what changed.
- `db::load_from_markdown` runs unconditionally on cold start (`fetch.rs:403`),
  the same full-corpus rebuild described above for push.

This is not a corner case introduced by the benchmark: `tools/bench/sync_scaling.py`
reproduces it directly. Repo `a` pushes (so its `last_synced_rev` advances) and
repo `b` only ever fetches (`measure()` in `sync_scaling.py:99-113`: `b`'s only
operations are `jjj fetch` twice, never `jjj push`). `b`'s `last_synced_rev`
stays `None` forever, so its second fetch — despite a 50-file delta on a
25,000-entity corpus — pays the full cold-start cost. Any real fetch-only or
review-only agent pod (one that pulls work and critiques it but doesn't push
its own metadata bookmark on every cycle) hits the identical trap.

## Why this matches the measured numbers

- **Linear in corpus, flat in delta**: both root causes are full-corpus scans
  (`list_fs::<T>()` over every entity file; `read_dir` + copy over every
  entity file; `root()`-based diff returning every file that ever existed) —
  none of them touch the delta size at all.
- **88% of push is jjj's own work**: the two passes above run entirely inside
  jjj, driving no jj subprocess calls proportional to corpus size — the jj
  calls stay flat (`SWARM.md`'s own count: 12 calls regardless of corpus).
- **Fetch is the worse ratio (7.4x vs. push's 8.8x is close, but structurally
  fetch's cold-start path also re-derives full `delta_git` content, not just
  the DB rebuild)**: consistent with a permanently-cold-start pod paying both
  penalties every single fetch.

## Where this leaves the two "reduce time" problems

- **Push** needs the validation reload and the directory copy scoped to what
  changed since `last_synced_rev` — likely by reusing the same delta
  machinery fetch already has for the file-copy side, and either an
  incremental validate or a change-scoped DB upsert for the reload side.
- **Fetch** needs `last_synced_rev` (or an equivalent per-pod watermark) to
  advance on a successful fetch too, not only on push — otherwise the delta
  path added for Pillar 1 (see `scaling-for-agent-swarms.md`, Break #1/#3) is
  unreachable for any pod that doesn't push every cycle.

No code path changes in this solution — this is the investigation problem;
`Make jjj sync cost proportional to the delta, not the corpus` and its two
child "reduce ... time" problems are where the fixes belong.
