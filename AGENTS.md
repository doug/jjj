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
- **`jjj doctor`** is the first thing to run when something looks wrong, and the
  right thing to attach to a bug report.

## Verifying changes to this repository

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Behaviour that users see should be covered by an executable journey in
`journeys/` — they are markdown, they run as tests, and they double as the
documentation. `RELEASE.md` lists the checks CI cannot make.
