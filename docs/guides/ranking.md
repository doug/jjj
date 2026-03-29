---
title: Ranking & Group Decision Making
description: How to prioritize problems using tertiary sort, quadratic voting, and multi-user aggregation
---

# Ranking & Group Decision Making

jjj includes a built-in ranking system for prioritizing problems within milestones. It's designed for teams that need to reach consensus on what to work on next, without requiring meetings or centralized planning tools.

## How It Works

Each user maintains their own **personal ordering** of problems per milestone. These orderings are aggregated into a **global ranking** using Borda count with quadratic voting boost. The result is a prioritized list that reflects the team's collective judgment.

### The Three Layers

1. **Tertiary sort** -- Quickly triage items into Top / Mid / Bottom tiers
2. **Quadratic voting** -- Spend vote budget to emphasize specific items
3. **Borda aggregation** -- Combine all users' orderings into one ranking

## Tertiary Sort (TUI)

The primary way to rank problems is through the TUI's tier-based sorting:

```
jjj ui
```

Navigate to a milestone's expanded problem list. You'll see tier separators dividing problems into thirds:

```
▼ Sprint 1
  ── Top ──
  #1 Critical auth bug
  #2 Payment validation
  ── Mid ──
  #3 Improve error messages
  #4 Add CSV export
  ── Bottom ──
  #5 Update dependencies
  #6 Refactor logging
```

### Keybindings

| Key | Action |
|-----|--------|
| `Shift+K` / `Shift+Up` | Assign to **top** tier |
| `Shift+J` / `Shift+Down` | Assign to **bottom** tier |
| `Shift+L` / `Shift+Right` | **Drill into** the tier containing the cursor |
| `Shift+H` / `Shift+Left` | **Drill out** one level |
| `+` / `=` | Add a vote to the selected problem |
| `-` | Remove a vote |
| `r` | Toggle between personal and global view |

### The Sorting Workflow

1. **First pass** -- Scan all items. Press `Shift+K` for things that matter most, `Shift+J` for things that can wait. Items you skip stay in the middle.

2. **Drill in** -- Move cursor to a top-tier item and press `Shift+L`. Now you see only the top third, split again into Top/Mid/Bottom. Repeat the triage.

3. **Drill out** -- Press `Shift+H` to zoom back out.

4. **Add votes** -- For items you feel strongly about, press `+` to add emphasis. Votes cost quadratically (1 vote = 1 cost, 2 votes = 4 cost, 3 votes = 9 cost), so you can't dump all your budget on one item.

This recursive process gives you O(N log N) sorting with O(N) key presses per pass.

## Quadratic Voting

Each user has a vote budget per milestone: `max(100, 2 * problem_count)`. Votes can be positive (boost) or negative (suppress).

The cost of K votes on one problem is K². This means:

- 1 vote costs 1
- 2 votes cost 4
- 3 votes cost 9
- 10 votes cost 100 (the entire default budget)

This forces you to spread your influence across multiple problems rather than concentrating it all on one.

Votes appear as arrows in the tree view:

```
  #3 Improve error messages ▲▲
  #5 Update dependencies ▼
```

## Global Aggregation

When multiple users have orderings for the same milestone, jjj aggregates them using **Borda count + QV boost**:

1. **Borda points**: Each user's ordering awards N points to their #1, N-1 to #2, etc.
2. **QV boost**: Each vote adds its value directly to the score (positive or negative).
3. **Harmonic weighting**: Borda points are weighted by `1/rank` for each user, giving top-ranked items proportionally more influence.

Toggle between views with `r`:
- **Personal view** -- Your ordering, your votes, your tiers
- **Global view** -- Aggregated ranking across all users

## CLI Commands

### View rankings

```bash
# Show aggregated rankings for the first active milestone
jjj rank show

# Show rankings for a specific milestone
jjj rank show "Sprint 1"

# Per-user breakdown
jjj rank show --by-user

# JSON output for scripting
jjj rank show --json
```

### Example output

```
Rankings for: Sprint 1 (6 problems, 2 voters)

  Rank  Problem                                    Score  Voters
  ─────────────────────────────────────────────────────────────
  #1    Critical auth bug                          12.5   2
  #2    Payment validation                          9.3   2
  #3    Improve error messages                      7.1   1
  #4    Add CSV export                              4.0   2
  #5    Update dependencies                         2.5   1
  #6    Refactor logging                            1.2   1
```

## Storage

Personal orderings are stored as JSON files in the jjj metadata branch:

```
rankings/{milestone_id}/{user-slug}.json
```

Each file contains the ordered problem list and vote allocations. They sync with `jj git push` like all other jjj metadata.

## Tips

- **Start rough, refine later.** The first pass through tertiary sort takes 30 seconds for 20 items. Drill in only where precision matters.
- **Negative votes are useful.** If something keeps bubbling up that you think is low priority, vote it down.
- **Check global view after sorting.** Press `r` to see how your ordering combines with the team's. You might discover disagreements worth discussing.
- **Re-sort after scope changes.** When problems are added or solved, your ordering updates automatically (new items go to the end).
