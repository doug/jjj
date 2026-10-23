# Changelog

## Unreleased

### Writes refuse to destroy work the caller never saw

- **`--if-lamport <N>` on every edit** refuses the write unless the entity's
  logical clock is still `N` (read it from `show --json`). It guards the
  decisions too — `solve`, `dissolve`, `assign`, `submit`, `approve` are as
  destructive as an edit and were previously unconditional.

- **A refused write exits 13**, not 1. The difference is the point: 13 means
  nothing was written and the same command may succeed after a re-read, while 1
  means the command was wrong and retrying changes nothing. Without it
  `--if-lamport` is useless in a loop, because a caller has to tell a stale read
  from a malformed argument by matching on prose.

- **An empty value is no longer an instruction to erase.** `--body ""` from an
  unset shell variable used to wipe a body and report success. It is refused;
  erasing is spelled `--clear-body`.

- **`--append-body`** adds to a body without rewriting it, read inside the write
  transaction so there is no window to lose an edit in. Two clones appending
  concurrently still contend, and the tests say so.

- **`assign` refuses a live claim** unless `--force`, and records
  `claim_contested` when it takes one. Handing on work that was assigned rather
  than claimed is still allowed — that is not a contest.

### Merges that produced impossible entities are refused

- **A field can no longer outlive the status that gave it meaning.** One clone
  dissolving a problem while another solved it merged to `status: solved` with
  `dissolved_reason` set — simultaneously Solved and Dissolved, with the reason
  printed to the user. The same shape covered a finding's `superseded_by`.
  Nothing is lost: the event log records the detail and merges by line union.

- **A conflicted body is never wrapped twice.** Re-merging added another marker
  set every single time. That is the mechanism behind a swarm run whose conflict
  markers went from 4,371 at single depth to 14,215 nested — each fetch
  re-wrapping a body that already held both hunks.

- **`updated_at` is compared as an instant, not a string.** chrono drops
  trailing subsecond zeros, so RFC3339 is not fixed-width here and a write
  landing on an exact millisecond sorted as though it were earlier than one 4µs
  before it.

- **A hand-off is not reverted by a surviving lease.** Earliest-wins applies
  only between two leases; a bare `assignee` with no `claimed_at` is not one.

### A diagnostic that finds a fault fails

- **`jjj doctor` exits non-zero** when a check fails. It used to print
  everything that was wrong and exit 0, so anything branching on the code — CI,
  a wrapper, an agent deciding whether the repo is usable — was told the
  repository was healthy. `tools/bench/bench.py` had the same shape: a skipped
  measurement counted as a pass, so a release gate could go green over a
  benchmark that never ran.

- **Fixes say when running them is a decision, not a repair.** Four of them
  destroy work, and the difference was invisible in the output. An unattended
  agent clearing `doctor` to green would run `jjj resolve --theirs` and discard a
  colleague's edit, because nothing said that command picks a loser. The hazard
  is named per check, and carried on `--json` as `fix_needs_judgement`.

- **Two new checks**: frontmatter keys no client recognises (the `extra` map that
  preserves them is invisible by design, and an unread key is usually a typo or a
  removed client), and the cache checked against the markdown that owns the
  truth rather than assumed to agree with it.

### Finding work

- **`jjj next --tag <lane>`** restricts the queue to one lane so several agents
  can share it without colliding. The lane resolves upwards: a critique of a
  solution to problem P is in P's lane.

- **Work a person is already needed on is not offered.** An escalation naming an
  entity suppresses it, resolved up the same chain. A *global* escalation does
  not halt the queue — "somebody look at the release" is not a reason to stop all
  work — and clearing one returns the work.

- **A refuted attempt is reported, not erased.** A problem whose only solution
  was withdrawn looked identical to one nobody had tried, so the next agent
  rediscovered the dead end.

- **`query waiting` names the blockers.** Each row carries `blocked_by`, so the
  one call that answers "what is stuck" no longer needs one more call per row to
  find out why.

- **A listing capped by `limit:` says so**, on both paths. On `--json` an agent
  could not tell a truncated answer from a complete one; the notice goes to
  stderr so stdout stays parseable.

- **`query --count-by <field>`** answers "how much of what" without listing
  anything. Whether the groups overlap is computed from the numbers, not guessed
  from the field name.

### One call, one graph

**`jjj plan apply FILE|-`** creates a whole graph of work from one JSON
document. Nodes name each other by symbolic `key`, because the ids do not exist
yet, and the result maps each key to the id it became — without which the caller
has created work it cannot name. Unknown fields are refused rather than ignored:
a plan is written by something that cannot see a warning, and a misspelled
`prioirty` would otherwise create the work at the wrong priority and report
success. The plan is validated before anything is written. `--dry-run` reports
the outcome without writing.

A finding in a plan takes **`measured_at`**, and findings and critiques a plan
creates now carry an **author**. A plan is the one case where measurement time
and recording time differ by a lot — it is written when the numbers are taken
and applied whenever someone gets to it — so the entity kind whose whole purpose
is *when was this measured* was the one that could not say. A malformed
timestamp is refused during validation, before anything is written, by the same
parser `finding new --measured-at` uses. Without the author, a fifty-problem
plan produced findings and critiques that no author filter and no `--mine` could
see.

### Escalations have a reply half

**`jjj escalate --respond <id> "what I found"`** answers without closing, and
records `escalation_answered`. The answer used to live in whatever channel the
human happened to use, so the next agent rediscovered the same wall. `jjj prime`
prints the reply — for the next agent that is the most useful line in the whole
briefing. Clearing stays separate, because "here is the answer" and "this no
longer needs a person" are different claims that often arrive at different times.

### Added

- **`finding --measured-at <RFC3339>`** records when the measurement was taken,
  as distinct from when it was written down. Two findings a week apart in the
  thing they measured used to look simultaneous.

- **`problem show --json` returns what the text path returns** — solutions,
  evidence, what it is waiting on, children. It stopped at the frontmatter, so
  an agent had to make three more calls to rebuild what `show` already had in
  hand.

- **`jjj prime`** orients an agent in one call, and its default output is
  constant in corpus size: 7 lines at 50 entities, at 2,000 and at 25,000.

### The cache stops being load-bearing for correctness

- **`jjj push` validates the markdown, not the cache.** Its eight reference and
  cycle checks ran against SQLite, so publishing was gated on a derived index
  rather than on the files being published. The checks now walk the entity files
  directly, and `push` opens no database at all — it neither requires a cache
  nor creates one.

  Two traps worth recording, because both are general. The first version read
  through the ordinary listings, which serve straight from a clean cache; an
  entity written to markdown but not yet indexed was therefore invisible, and
  validation reported "All checks passed" over a corpus containing a dangling
  reference. It has to be the authoritative filesystem walk. Separately, `push`
  used to clear the cache's dirty flag — true while validation loaded every
  entity, false once it stopped, and six readers consult that flag before
  trusting a cached read. Removing a cache *write* can break a cache *read*
  several files away.

- **The TUI's related pane no longer rebuilds the cache it finds missing.** It
  opened the database with a call that *creates* what is absent, so in a
  repository with no cache a background thread silently rebuilt the whole thing
  — unobservably, from a thread, while the user was looking at a list. Opening a
  cache opportunistically now refuses in one place for all three reasons it
  should: the bypass flag is set, the file is absent, or the file will not open.

### Fixed

- **`jjj prime` caps the escalation section like every other one.** Every
  sibling section listed at most five items with an "and N more" line; the
  escalation section printed all of them, in both the text and JSON paths. A
  repository with ninety open escalations buried the rest of the briefing — the
  one section most likely to be long was the one without a limit. `--full`
  still shows everything, and the count has always been in the heading.

### Testing

- **A format corpus per release, not just the oldest.** A corpus guards only the
  keys it contains, so the newest fields were the least protected. Nothing
  carried a `lamport` key before `corpus-0.5`, so a release that stopped reading
  the field deciding which concurrent edit wins would have passed the whole
  compatibility suite with every listing still looking correct.

- **`scripts/check-invariants.sh`** runs as its own CI job. Each rule names the
  incident it came from, and because it is a text check over `src/` it reports a
  broken fence in seconds rather than after a compile.

- **`scripts/verify-commits.sh`** compiles every commit on a branch, not just
  its tip. A branch whose tip builds can still have commits in the middle that
  do not, which makes `git bisect` useless exactly when it is needed; a rebase
  that reorders a caller ahead of the function it calls produces this without
  touching a line of code.

## 0.5.2 — 2026-08-22

### Sync is delta-proportional

A swarm of agents working on jjj itself found and fixed the two paths that made
sync cost scale with how much history exists rather than how much changed.

- **Fetch no longer pays a cold start every time.** `last_synced_rev` advanced
  only on push, so a clone that only ever fetches — a review-only agent, or any
  pod between pushes — stayed at `None` forever and re-derived the whole corpus
  on every fetch. Measured on a 25,000-entity corpus: `warm_delta_fetch` for 100
  changed files goes from 6.9s to 0.88s.

- **Push validates and copies incrementally.** Validation reloaded every entity
  file into SQLite and the sync workspace rewrote every `.md`, on every push,
  however small the delta. Both now do work proportional to what changed:
  `delta_push` goes from 12.5s to 3.6s. A first push is faster too, 18.3s to
  13.6s, because it routes through the bulk path rather than building the search
  index a row at a time.

  The tempting shortcut here is wrong and was rejected twice: skipping
  validation when the cache looks clean, or keying the skip on `(mtime, size)`,
  both let a stale cache publish a dangling reference or a conflict-marked body
  to every clone. Timestamp-preserving copies (`rsync -t`, `cp -p`, `tar`)
  defeat the second by design. The cache is keyed on content.

### Review works between clones

- **`jjj solution submit` publishes the code.** Attaching recorded a change id
  but never shared the commit, so a reviewer who fetched saw a solution pointing
  at a revision that did not exist for them. Submit now pushes a
  `review-s-<id>` branch, and reviewers can read the actual diff.

- **`jjj solution lgtm --approve`** finishes a review in one step. Signing off
  did not approve, and a reviewer who stopped there left the work stranded —
  eight reviewed solutions and nothing merged, in one trial. It still refuses
  while any critique is open.

### Every entity body is writable without an editor

`problem`, `solution`, `critique` and `milestone` creation all take `--body`
(and `--body -` to read stdin). The only way to write a description, an
approach or an argument used to be an interactive `$EDITOR`, which a scripted
or headless caller does not have — so arguments ended up in titles.

### Fixed

- **Review branches no longer collide with metadata bookmarks.** They were named
  `jjj-s-<id>`, which matches the `jjj*` glob that finds metadata bookmarks, so
  a solution's *code* commit was merged into the *metadata* commit: the sync
  commit picked up the whole source tree, conflicted, and jj refused to push it.
  84% of metadata pushes failed this way in one trial, while the remote stayed
  perfectly clean — the contamination existed only in the commit being built.

- **State transitions are scriptable.** `critique address|validate|dismiss` and
  `solution lgtm` accept `--json` like `new`, `list` and `show` already did, and
  emit the updated entity so a caller need not follow up with `show`.

- **`problem dissolve` accepts `--rationale`**, the spelling every other command
  that records a decision uses.

- **The MSRV job now tests the MSRV.** `rust-toolchain.toml` overrides whatever
  rustup is told to default to, so the job installed the declared MSRV and then
  built with something else. It has proved nothing for as long as it has
  existed; the declared 1.88 is, as it happens, true.


### Added

- **`jjj solution lgtm` works for pull-based review.** It previously required a
  review critique *already assigned* to you, which only fits a push model — an
  author names a reviewer, the reviewer signs off. Teams and fleets that pull
  from a queue instead were stuck: 49 of 92 sign-offs failed in one trial, and
  the error advised filing a critique against work the reviewer believed
  **correct**, filling the objection record with fake objections. Signing off on
  self-selected work now records the review in one step, with `--rationale`
  carrying the evidence.

  Two guards come with it: only *submitted* work can be signed off, and **an
  author can never sign off their own solution** — making self-selected review
  easy must not make self-approval easy, since that gate is what stops a fleet
  approving its own homework.

- **CLI consistency: `--mine`, `events --user`, `lgtm --rationale`.** A
  nine-agent trial produced 274 failing invocations, and roughly 150 were agents
  calling flags that did not exist but plausibly should have — `solution list
  --mine` 76 times, because `next --mine` and `critique list --mine` both do.
  Agents probe an API far more systematically than people, and they had inferred
  the consistent surface jjj lacked. Now:
  - `--mine` on `problem list`, `solution list` and `events` (it already existed
    on `next`, `status` and `critique list`)
  - `events --user <actor>` — "what did agent-03 do?" is the first question
    anyone asks of a swarm's log, and it had no answer
  - `solution lgtm --rationale "..."` — a sign-off asserts the work is correct,
    so it should carry the evidence, as `approve` already does

- **`jjj next` spreads a fleet instead of stampeding it.** Category, priority and
  age are identical for every agent, so a strictly-ordered queue handed all of
  them the same head of the list and they all took it — measured at nine agents,
  twenty-three claim attempts, three distinct items out of eight. Within a
  priority band, work is now ordered by a hash of the actor and the entity id:
  each actor gets a stable but different order, so they spread out without
  coordinating, and an agent does not reshuffle between turns. Priority still
  decides what matters; strict oldest-first *within* a band is deliberately given
  up, which for one worker is cosmetic and for a fleet is the difference between
  parallel and serial.

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

- **A not-yet-fetched reference no longer breaks the cache.** The SQLite cache
  enforced foreign keys, but it is a *derived* view of an eventually-consistent
  distributed store: a fetch legitimately delivers a critique before the solution
  it references, or a solution whose problem is still on another pod's unmerged
  bookmark. That is convergence, not corruption — and enforcing it made
  convergence fatal. One such reference failed the whole `db rebuild` with
  `FOREIGN KEY constraint failed`, which also fails `push`, so a clone could
  neither heal its cache nor publish its work — and rebuild was the very thing
  that would have healed it. A one-hour swarm hit this 140 times. Genuinely
  dangling references are still refused, in the right place and as a report
  rather than a brick: `jjj db validate`, which `push` runs before publishing.

- **An empty entity reference no longer resolves to an arbitrary entity.**
  Resolution falls back to a case-insensitive substring match on the title, and
  every title contains the empty string — so `""` matched *every* entity, and in
  a repository holding exactly one it matched that one **successfully and
  silently**. Since the usual source of an empty argument is an unset shell
  variable, `jjj solution approve "$SID"` would approve something arbitrary, and
  `problem dissolve ""` would dissolve something. Empty references are now
  rejected with a message naming the likely cause. (The deferred audit item 4.2,
  in its pathological form.) Found when a swarm agent called `solution attach`
  with no argument twenty times in an hour.

- **`solution new` now says what to do about a duplicate.** Its only advice was
  "use --force to create anyway", which is precisely wrong in the common case:
  when two people race onto the same work, forcing creates the duplicate that was
  just detected. It now distinguishes the two situations, since they call for
  opposite actions — take different work, or give a genuinely rival approach a
  title that says how it differs.

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
