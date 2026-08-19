---
title: Diagnostics & Automation Commands
description: CLI reference for jjj doctor and jjj automation
---

# Diagnostics & Automation Commands

## `jjj doctor`

```bash
jjj doctor [--json]
```

One read-only pass over the environment and the repository. jjj's failure modes
are mostly environmental — a stale push lock, a cache that needs rebuilding, a
`jj` whose CLI moved, an automation rule you did not know was active — and each
is easy to see once you know where to look. This gathers them in one place.

Nothing here mutates the repository, so it is always safe to run, and it is the
right thing to paste into a bug report.

**Checks:**

| Check | Reports |
|---|---|
| `jjj` | The running version |
| `jj` | The jujutsu version jjj resolves and can execute |
| `identity` | The actor writes are attributed to (`JJJ_USER` → pod → jj config) |
| `metadata` | Metadata path and entity counts per type |
| `cache` | Whether the SQLite cache exists, opens, and its schema version |
| `push lock` | Whether a `.push.lock` is held, and by which pid |
| `write lock` | Whether the flock file is present (never a fault — the kernel releases it) |
| `conflicts` | Entities left carrying unresolved merge markers |
| `automation` | How many rules are active, from where, and how many run shell commands |
| `sync` | The last synced revision, if this clone has pushed |

Every warning carries the command that fixes it.

**Example:**

```bash
$ jjj doctor
jjj doctor
──────────────────────────────────────────────
✓ jjj                    v0.5.1
✓ jj                     jj 0.44.0
✓ identity               Alice
✓ metadata               .jj/jjj-meta — 12 problems, 5 solutions, 3 critiques, 1 milestones
! cache                  no SQLite cache — reads fall back to walking the filesystem
    → run `jjj db rebuild` (search and fast listings need it)
✓ push lock              free
✓ conflicts              none
✓ automation             2 rule(s) active from automation.toml (1 shell)
✓ sync                   last synced rev qkxrpyms

Usable, with warnings above.
```

`--json` emits one object per check with `check`, `level` (`ok` / `warn` /
`problem`), `detail` and `fix` — for a supervisor asserting a pod is healthy
before it starts writing.

## `jjj automation list`

```bash
jjj automation list [--json]
```

Show which automation rules are active and where they came from, plus any rules
sitting in `config.toml` that are being **ignored**.

Rules live in the machine-local `.jj/jjj-meta/automation.toml`. `config.toml`
travels through the shared `jjj` bookmark, so a rule there would run whatever a
collaborator pushed — those are reported and never executed. See
[Configuration](/reference/configuration/) for the full rationale.

**Example:**

```bash
$ jjj automation list
Active rules (1) — from automation.toml:
  on solution_submitted → GithubPr

Ignored (1) — found in the synced config.toml:
  on problem_created → Shell
      curl https://example.invalid/hook

These do not run. config.toml is shared through the jjj bookmark,
so a rule there would execute whatever a collaborator pushed.
Move them to this machine with:  jjj automation migrate
```

## `jjj automation migrate`

```bash
jjj automation migrate [--force]
```

Move rules from the synced `config.toml` into the machine-local
`automation.toml`, for repositories created before 0.5.1.

Without `--force` the rules are printed for review and nothing changes — a rule
that arrived from a remote runs with your privileges, so read it before adopting
it. With `--force` the rules are appended to `automation.toml` and the
`automation` key is removed from `config.toml` so it stops syncing.
