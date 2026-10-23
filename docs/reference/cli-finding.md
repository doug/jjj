---
title: Finding Commands
description: CLI reference for recording and citing evidence about a problem
---

# Finding Commands

A **finding** is evidence measured about a problem. It is the only entity kind
with no approval path, and that is deliberate: a measurement is not something a
reviewer agrees to. A finding is either `current` or `superseded` by a better
measurement.

Findings are what stop the same investigation being run twice. Solutions and
critiques cite them with `--cites`, which is what separates a conjecture
grounded in evidence from one that is merely plausible.

## `jjj finding new`

```bash
jjj finding new <PROBLEM_ID> <TITLE> [OPTIONS]
```

Record something measured or established about a problem.

| Argument | Description |
|----------|-------------|
| `<PROBLEM_ID>` | Problem this is evidence about (ID, short prefix, or fuzzy title) |
| `<TITLE>` | What was found, **stated as a claim** |

| Option | Description |
|--------|-------------|
| `--body <BODY>` | The evidence itself: numbers, output, what you saw. `-` reads stdin |
| `--method <METHOD>` | How it was measured, so someone else can repeat it |
| `--ref <ID>` | Entities this bears on — solutions, critiques, other findings (repeatable) |
| `--tag <TAG>` | Tags for categorization (**repeatable**) |
| `--measured-at <WHEN>` | When the measurement was taken, if not now (RFC3339) |
| `--json` | Print the created entity as JSON |

**State the title as a claim.** "p99 is 2.3s at 10k documents" is a finding;
"search performance" is a label. The claim is what a later reader compares
against, and what a superseding finding has to contradict.

> **Note the flag spelling.** `finding new` takes `--tag` once per tag, while
> `problem new` and `solution new` take a single comma-separated `--tags`.
> `--tag perf --tag backend` here; `--tags perf,backend` there.

**`--method` is not optional in practice.** A number nobody can reproduce is not
evidence, and the next agent to look will re-measure it anyway — which is the
cost findings exist to avoid.

### `--measured-at` vs `created_at`

`created_at` records when you wrote the finding down; `--measured-at` records
when the thing happened. Two findings filed in the same minute may describe
states a week apart, and without this they look simultaneous.

```bash
jjj finding new "Search is slow" "p99 is 2.3s at 10k docs" \
  --method "ab -n 1000 -c 10 against staging" \
  --measured-at 2026-10-16T14:00:00Z
```

A value that is not RFC3339 is refused before anything is written, with the
format spelled out in the error.

## `jjj finding list`

```bash
jjj finding list [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--problem <PROBLEM>` | Filter to findings on a specific problem |
| `--status <STATUS>` | `current` or `superseded` |
| `--author <AUTHOR>` | Filter by author (substring match) |
| `--mine` | Only findings you recorded |
| `--search <SEARCH>` | Filter by title keyword |
| `--about <ID>` | Findings that reference this entity |
| `--json` | Output as JSON |

> `--search` requires the SQLite cache at `.jj/jjj.db`, because full-text search
> is an index rather than stored data. Run `jjj db rebuild` if it reports no
> index. Every other filter here walks the markdown and needs nothing.

## `jjj finding show`

```bash
jjj finding show <FINDING_ID> [--json]
```

Show a finding in full, with its evidence and method. `<FINDING_ID>` accepts a
full ID, a short prefix, or a fuzzy title match.

## `jjj finding edit`

```bash
jjj finding edit <FINDING_ID> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--title <TITLE>` | New title |
| `--method <METHOD>` | New method description |
| `--body <BODY>` | Replace the evidence body. `-` reads stdin. **Refused if empty** |
| `--clear-body` | Erase the evidence deliberately |
| `--append-body <TEXT>` | Add to the end of the existing body, separated by a blank line. `-` reads stdin |
| `--if-lamport <N>` | Refuse the write unless the entity's Lamport clock is still `N` |

**Why erasing has its own flag.** `--body ""` cannot be distinguished from a
command substitution that produced nothing, so an unset shell variable used to
wipe a body and report success. Clearing is spelled `--clear-body`.

**`--if-lamport` and concurrent edits.** Read the clock from `show --json`,
where a clock of `0` is omitted rather than printed — use a default:

```bash
L=$(jjj finding show 01957d --json | jq '.lamport // 0')
jjj finding edit 01957d --append-body "re-ran at 50k docs: p99 4.1s" --if-lamport "$L"
```

A refused write exits **13**, not 1. The difference matters in a loop: 13 means
nothing was written and the same command may succeed after a re-read, while 1
means the command was wrong and retrying changes nothing.

This guards against overwriting an edit *this clone has already seen*. It cannot
guard against one on a clone that has not synced yet; that collision surfaces at
merge, where the Lamport clocks decide.

## `jjj finding supersede`

```bash
jjj finding supersede <FINDING_ID> --by <ID> [--json]
```

Mark a finding as replaced by a better measurement.

**The old finding is kept, not deleted.** Knowing that a number was once
believed — and what corrected it — is why the same investigation is not run
twice. It is also how you see a number move: the superseded finding and its
replacement read together as a before and after.

```bash
NEW=$(jjj finding new "Search is slow" "p99 is 180ms after the index" \
        --method "same ab invocation" --json | jq -r .id)
jjj finding supersede "p99 is 2.3s" --by "$NEW"
```

## `jjj finding delete`

```bash
jjj finding delete <FINDING_ID>
```

Delete a finding. Prefer `supersede` for anything that was ever true — deleting
destroys the record that it was once believed. Delete is for findings recorded
in error, against the wrong problem.

## Citing findings

Findings are referenced, not approved. The citation lives on the thing that
relies on the evidence:

```bash
jjj solution new "Add a trigram index" --problem "Search is slow" --cites <finding>
jjj critique new "search index" "the benchmark used a warm cache" --cites <finding>
```

`jjj prime` prints what has already been measured, which is the point: an agent
that reads it does not re-measure what the fleet already knows.

## A worked example

The whole evidence lifecycle — measure, supersede, cite:

```bash,test
jjj init
jjj problem new "Search is slow"
jjj finding new "Search is slow" "p99 is 2.3s at 10k docs" --method "ab -n 1000 -c 10" --measured-at 2026-10-16T14:00:00Z
# expect: Method: ab -n 1000 -c 10
jjj finding list --problem "Search is slow"
# expect: p99 is 2.3s at 10k docs
jjj finding new "Search is slow" "p99 is 180ms after the index" --method "same ab invocation"
jjj finding supersede "p99 is 2.3s" --by "p99 is 180ms"
# expect: superseded by
jjj finding list --status superseded
# expect: p99 is 2.3s at 10k docs
jjj solution new "Add a trigram index" --problem "Search is slow" --cites "p99 is 180ms"
# expect: Add a trigram index
```

The superseded finding still lists under `--status superseded`, and the
conjecture that replaced it cites the measurement that justified it. Those two
facts together are the record that stops the question being reopened from
scratch.
