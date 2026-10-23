# Agent instructions

This repository is **jjj**, a project tracker built on Jujutsu. It is also
managed *with* jjj — problems, solutions, and critiques live in the `jjj`
bookmark alongside the code.

## Before you write anything

Set an identity. Without it every agent shares the repository owner's identity,
so work claims overwrite each other and per-agent queues do not work:

```bash
export JJJ_USER=<your agent name>
jjj whoami --json          # confirm it took effect
```

## Read this first

**`skills/jjj/SKILL.md`** — how to use and collaborate through jjj: the machine
interface, the single-agent loop, and the multi-agent patterns (work claiming,
rival conjectures, adversarial review, collision avoidance, pods). It is checked
against the real CLI by `tests/skill_test.rs`, so it does not drift.

For working on jjj's own code, `CLAUDE.md` covers the architecture, storage
model, and build commands.

## House rules

- **Use ids, not titles, in anything scripted.** Fuzzy title matching resolves to
  whatever uniquely matches today. Read ids from `--json`.
- **Never `--force` past a critique.** `jjj solution approve --force` bypasses the
  one mechanism that makes review real.
- **Automation rules are machine-local** (`.jj/jjj-meta/automation.toml`) and are
  deliberately never synced. Do not move them into `config.toml`; that file
  travels through the shared bookmark and a rule there would execute on every
  collaborator's machine.
- **`jjj prime` is the first call of a turn.** It gives you what to work on,
  what is already proposed, what has already been measured, and what is blocked
  on a person — in one call with output that does not grow with the corpus.
  Re-deriving that with a dozen listings is the single largest waste in a swarm
  run.
- **Guard every write you based on a read.** Pass `--if-lamport <N>` with the
  clock from `show --json` (`jq '.lamport // 0'` — a clock of 0 is omitted). A
  refused write **exits 13**; treat that as "re-read and retry", and only exit 1
  as "the command was wrong".
- **Record evidence as a finding, not in prose.** `jjj finding new` with
  `--method` is what stops the next agent re-measuring what you already
  measured, and `--measured-at` keeps two findings a week apart from looking
  simultaneous.
- **`jjj escalate` is only for what a human must do** — a dead credential, a
  destructive step, a blocker the playbook does not cover. Not retries, not
  routine progress. A channel carrying those is a channel nobody reads.
- **`jjj doctor`** is the first thing to run when something looks wrong, and the
  right thing to attach to a bug report.

## Verifying changes to this repository

```bash
./scripts/gate.sh          # fmt, invariants, clippy, tests — verdict in GATE_EXIT
```

`gate.sh` runs the stages below and reports which one failed. Note that it
deliberately does **not** run `cargo fmt` before `cargo fmt --check`: a stage
must not repair what it measures.

```bash
cargo fmt --check
./scripts/check-invariants.sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Before committing a branch, check that **every** commit compiles, not just the
tip — a rebase that moves a caller ahead of the function it calls breaks
`git bisect` without touching a line of code:

```bash
./scripts/verify-commits.sh origin/main
```

Behaviour that users see should be covered by an executable journey in
`journeys/` — they are markdown, they run as tests, and they double as the
documentation. `RELEASE.md` lists the checks CI cannot make.
