---
title: Ranking & Group Decision Making
description: How to prioritize problems with ordered lists, sized gaps, and multi-user aggregation
---

# Ranking & Group Decision Making

jjj includes a built-in ranking system for prioritizing problems within milestones. It's designed for teams that need to reach consensus on what to work on next, without meetings or a centralized planning tool.

## How It Works

Each user maintains their own **personal ordering** of problems per milestone: a sorted list in which you can mark a **sized gap** below any item to say "there's a big priority cliff right here." Order expresses *direction* (what beats what); gaps express *intensity* (by how much). These per-user orderings are aggregated into a **global ranking** that reflects the team's collective judgment.

Direction and intensity are a single authored signal — there are no separate "votes." (Earlier versions used quadratic votes and Top/Mid/Bottom tiers; those were replaced by gaps. Any vote data in older ranking files is ignored on load.)

## Ranking in the TUI

```
jjj ui
```

Expand a milestone's problem list. The number beside each problem is its rank; its color marks which third it falls in — green (top), amber (mid), red (bottom).

### Keybindings

| Key | Action |
|-----|--------|
| `Shift+K` / `Shift+↑` | Nudge selection **up** one slot (double-tap within 400 ms: fling to top) |
| `Shift+J` / `Shift+↓` | Nudge selection **down** one slot (double-tap: fling to bottom) |
| `p` | Cycle the sized **gap** below the selected item: none → S → M → L → XL → none |
| `r` | Toggle between **personal** and **global** view |
| `Ctrl+Z` | Undo the last ordering change |

### The Workflow

1. **Triage fast.** Press `Shift+K`/`Shift+J` to nudge a problem up or down one slot. Double-tap to fling it straight to the top or bottom. Get a rough order in seconds.

2. **Mark the cliffs.** Where priority drops sharply between two items, put the cursor on the upper one and press `p` to insert a gap (cycling S → M → L → XL). A `▾XL` marker means "everything below this is a different league." A big gap just above the bottom item is the "**must not win**" signal.

3. **Compare with the team.** Press `r` to see how your ordering combines into the global ranking — a good way to spot disagreements worth discussing.

Gaps and order are saved automatically and sync with the rest of your jjj metadata.

## Sized Gaps

A gap sits *below* an item and stretches the distance to the next one. The implicit (unmarked) gap is one unit; sized gaps grow geometrically:

| Gap | Depth it adds |
|-----|---------------|
| (none) | 1 |
| S | 2 |
| M | 4 |
| L | 8 |
| XL | 16 |

An item's scoring weight is `1 / (1 + cumulative depth above it)`. With **no** gaps marked, the weights are exactly the harmonic sequence `1, ½, ⅓, …` — so an un-annotated list ranks identically to a plain ordered list. Marking gaps simply stretches the cliffs you care about; everything stays backward-compatible.

## Global Aggregation

When multiple users have orderings for the same milestone, jjj combines them:

1. **Budget-normalized influence.** Each user's gap-weighted points are scaled so they sum to a fixed budget `B = max(100, 2 × problem_count)`. A user who ranks 3 items has the same total influence as one who ranks 30 — sorting is the baseline activity and everyone's counts equally. The harmonic shape means getting the **top** of your list right matters far more than the tail.
2. **Gaps create real cliffs.** A large gap (for example an `XL` above the bottom item) drives that item well below the pack — the "anything but this" signal, with no need for a separate negative-vote channel.
3. **Ties** are broken by problem ID (lexicographic) for determinism.

Toggle the view with `r`:
- **Personal view** — your ordering and your gaps.
- **Global view** — the aggregated ranking across all users.

## CLI Commands

```bash
# Aggregated rankings for the first active milestone
jjj rank show

# A specific milestone
jjj rank show "Sprint 1"

# Per-user breakdown (shows each user's ordering and gaps)
jjj rank show --by-user

# JSON output for scripting
jjj rank show --json
```

### Example output

```
Rankings for milestone: Sprint 1

  Rank  Problem                                        Score  Voters
  ──────────────────────────────────────────────────────────────────
  #1    Critical auth bug                               48.1   2
  #2    Payment validation                              22.7   2
  #3    Improve error messages                          14.0   1
  #4    Add CSV export                                   9.6   2
```

(`Voters` is the number of users who ranked the problem.)

## Storage

Personal orderings are stored as JSON in the jjj metadata branch:

```
rankings/{milestone_id}/{user-slug}.json
```

Each file holds the ordered problem list and a per-item map of sized gaps. Each file is owned by exactly one user, so on `jj git fetch` they merge per file (last writer wins per file) — no three-way merge needed. They sync with `jj git push` like all other jjj metadata.

## Tips

- **Start rough, refine later.** Nudge items into a rough order first; mark gaps only where the priority cliffs actually matter.
- **A big bottom gap means "must not win."** Use it for things that should stay decisively last.
- **Check the global view after ordering.** Press `r` to see how your judgment combines with the team's.
- **Scope changes are handled for you.** New problems are appended to your ordering automatically; solved or removed ones are pruned.
