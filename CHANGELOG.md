# Changelog

## Unreleased

### Added

- **Claims are leases (design decision 15).** `jjj next --claim` now records
  `claimed_at`, and `jjj next` no longer offers work another agent is actively
  holding. Two problems this solves, both measured in the swarm trial:
  - Every agent was shown the same top item, so a fleet starting together all
    claimed one problem — four of four, on the first run.
  - A claim was permanent, so an agent that died mid-task held its work forever.
    Over a long run that is the expected failure, not a rare one.

  A lapsed lease returns the item to the pool and the reclaim is reported. Your
  own claim refreshes each time you re-claim, so an agent that keeps working
  keeps its work. An explicit `jjj problem assign` has **no** lease and never
  expires — handing work to a person is a decision, not a claim. Default one
  hour; set `claim_ttl_minutes` under `[settings]`.

### Fixed

- **Per-pod push bookmarks are `jjj-{pod}`, not `jjj/{pod}`** (breaking, for
  anyone whose pod pushes were working — which is nobody, see below). A git ref
  is a path: `refs/heads/jjj` is a file, so `refs/heads/jjj/{pod}` requires the
  same path to also be a directory and git rejects it with
  `cannot lock ref ... 'refs/heads/jjj' exists`. Since a plain `jjj push` with no
  pod creates the bare bookmark, **every real repository had it**, so per-pod
  push failed everywhere it mattered and Break #5's fix — the entire remedy for
  the ~quadratic ref contention measured in M0 — was inoperative. The GitHub PR
  branch (`jjj/s-{id}` -> `jjj-s-{id}`) had the identical defect. Both names
  still match the `jjj*` glob used for tracking and head discovery, so fetch is
  unchanged. No test caught this because none pushed from a pod to a real
  remote; found by the swarm trial in `tools/swarm/` on its first run, and now
  covered by `pod_and_bare_bookmarks_coexist_on_a_remote`.

- **Linux release binaries are now statically linked (musl).** The 0.5.1
  `*-unknown-linux-gnu` artefacts were built on `ubuntu-latest` and link against
  that runner's glibc 2.39, so they refuse to start on Debian 12, Ubuntu 22.04,
  RHEL 9 or Amazon Linux with `version 'GLIBC_2.39' not found`. Linux targets are
  now `*-unknown-linux-musl` built via `cross`, and the release verifies the
  result is not dynamically linked. `install.sh` fetches the musl assets.
  Found while building the swarm-trial container image against Debian bookworm.

## 0.5.1 — 2026-08-19

**Security release.** Upgrade if you share a repository with anyone.

### Automation rules are machine-local (CVE-class: remote code execution)

`config.toml` syncs through the shared `jjj` bookmark, and `jjj fetch` applied
the remote copy wholesale. Because `[[automation]]` rules with
`action = "shell"` were read from that file, anyone who could push the bookmark
could hand every clone an arbitrary shell command, which then ran on the next
routine operation (`problem new`, `solution submit`, …). No prompt, no warning,
and `jjj push` reported "✓ All checks passed".

The 0.5.0 hardening pass made rule *values* safe (untrusted titles travel via
the environment, so `$(...)` in a title is inert). This release closes the
remaining half — the rule *itself*:

- Rules now live in `.jj/jjj-meta/automation.toml`, which push never copies and
  fetch never writes.
- Rules found in `config.toml` are reported once and **ignored**, whether they
  are a legacy local config or arrived from a remote.
- `jjj push` strips the `automation` key from the shared copy, preserving every
  other key.
- `jjj fetch` announces config changes and warns when a fetched config carried
  rules.
- New `jjj automation list` (what is live, what was ignored, and from where) and
  `jjj automation migrate --force` (move legacy rules to the local file, showing
  them for review first).

**Affected:** all releases up to and including 0.5.0. **Action:** upgrade, then
run `jjj automation list` to confirm nothing unexpected is active.

### Dependencies

- **`serde_yml` → `serde_norway`.** The YAML parser that reads every entity file
  was archived upstream and carries RUSTSEC-2025-0068 (unsound, no safe
  upgrade). It parses metadata written by other people and delivered through the
  shared bookmark, so it is squarely inside the threat model above.
- **ratatui 0.29 → 0.30** (with crossterm 0.28 → 0.29), which also drops the
  unmaintained `paste`. The dependency audit now passes with no ignored
  advisories.
- **MSRV is now 1.88** (was 1.82), required by ratatui 0.30. CI builds at the
  declared MSRV so the number cannot quietly become a fiction again.

### For agents

- **`skills/jjj/SKILL.md`** — a checked-in skill describing how to use and
  collaborate through jjj: the machine interface, the single-agent loop, and the
  multi-agent patterns (claim loop, rival conjectures, adversarial review,
  collision avoidance, distributed prioritization, pods). `tests/skill_test.rs`
  checks every command it teaches against the real CLI, so it cannot drift into
  describing a jjj that does not exist — which is what happened to the
  hand-maintained version it replaces.
- **`AGENTS.md`** at the repository root, for harnesses that read it.
- **`--json` on `problem new`, `solution new` and `critique new`**, so an agent
  can capture the created id directly instead of parsing "Created ...".

### New commands

- `jjj doctor [--json]` — one read-only pass over the environment and repository:
  jjj and jj versions, resolved identity, metadata counts, cache health, lock
  state, unresolved conflicts, active automation, and sync state. Each warning
  carries the command that fixes it.
- `jjj automation list [--json]` — what is active and where from, plus anything
  ignored in `config.toml`.
- `jjj automation migrate [--force]` — relocate legacy rules, showing them for
  review first.

### Per-agent identity (behaviour change)

jjj resolves an actor from `JJJ_USER` → pod → jj `user.name` so that several
agents can share one checkout and remain distinct writers. That resolution was
only wired into event authorship; assignment and attribution read the raw jj
identity instead. On a machine running a fleet, every agent therefore appeared
as the same person:

- `jjj next --claim` assigned work to the jj user, so agents overwrote each
  other's claims and none could tell which items were theirs.
- `jjj critique new` credited every critique to whoever configured the repo.
- The TUI's personal ordering was keyed by the machine, not the agent.

All identity-bearing paths now use the resolved actor. Identity comparison moved
into one place (`identity::actor_matches`), which also accepts the
`Name <email>` form written before 0.5.1, so existing assignees keep matching
after an upgrade. The three previous ad-hoc comparisons included a substring
test that made `bo` match `bob` and an empty identity match everyone.

**`jjj next --mine` now means what it says.** It restricts the queue to work
this actor owns — the assignee of a problem or solution, or the reviewer of a
critique. Previously the flag only toggled the review section, and toggled it
the wrong way: `--mine` *hid* the review requests addressed to you.

### Testing and release infrastructure

- **The TUI has tests.** 6,654 lines of interactive code had none. `App::open_at`
  and a public `handle_key` make it drivable; 16 tests exercise ordering
  (nudge, fling, the full gap cycle, undo), navigation, scroll clamping, narrow
  renders, and that the TUI and `jjj rank show` agree on the same file.
- **GitHub sync runs in CI.** The e2e suite pointed at a live personal repository
  and gated on `gh auth status`, so it silently skipped and reported green. A
  stub `gh` fixture makes it hermetic; it also records its argv, which is how the
  `--base` flag is now verified to actually reach `gh`. The live suite remains
  behind `JJJ_LIVE_GITHUB=1`.
- **Durability is verified by breaking things**: a writer killed mid-save, six
  concurrent writers, a dead lock holder, a truncated cache, an unparseable
  entity file.
- **Format compatibility**: corpora generated by building and running 0.3.3 and
  0.4.1, asserting an upgrade still reads them.
- **Four new journeys** cover ranking, coordination, a two-clone review across
  machines, and triage — the commands that previously had no end-to-end test.
  Journeys now run one test per file, in parallel.
- **CI** additionally lints test code, verifies the declared MSRV, builds the
  `semantic` feature, audits dependencies, runs a benchmark tripwire, builds the
  documentation site, and builds and tests the VS Code extension.
- **Releases are automated.** A `v*` tag builds macOS and Linux binaries for arm64
  and x86_64 with SHA-256 sums, publishes a GitHub release from the changelog
  section, and publishes to crates.io. `install.sh` fetches and verifies a
  prebuilt binary, falling back to a source build. See `RELEASE.md`.

### Fixed

- The VS Code extension's `npm test` ran stale compiled output, so 13 tests for a
  deleted source file kept passing. The build now cleans first.
- `install.sh` documented itself as `curl … | sh` while being a bash script.
- The documentation site had not built since 2026-05-30: the audit findings log
  was picked up as a content page without the frontmatter Starlight requires,
  failing the whole build. `audit/` and `plans/` are now excluded as the internal
  working documents they are, and the docs build runs on pull requests instead of
  only after merge — which is why nobody noticed for three months.

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
