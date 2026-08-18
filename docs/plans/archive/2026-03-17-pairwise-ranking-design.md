# Pairwise Ranking with Glicko-2

**Date**: 2026-03-17
**Status**: Approved

## Problem

Distributed teams need a way to express and aggregate opinions about which problems matter most, given limited resources. The existing static priority field (Critical/High/Medium/Low) is too coarse and doesn't capture team consensus or allow nuanced ordering.

## Design

### Core Concept

Problem ranking uses pairwise comparisons ("A is more important than B") with Glicko-2 ratings to produce a total ordering of problems within a milestone. Each comparison is attributed to a user and stored as an append-only event. The system suggests optimal matchups based on uncertainty to minimize comparisons needed.

Solution ranking is handled by the existing critique system — critiques ARE the ranking for solutions.

Glicko-2 ratings replace the existing priority tier system entirely.

### Algorithm: Weighted Glicko-2

1. All comparisons from all users for a milestone are pooled
2. Each comparison has a **weight** based on the user's role:
   - Milestone owner: configurable multiplier (default 2x)
   - Other contributors: 1x
   - Recency can optionally factor in (newer comparisons slightly preferred)
3. Standard Glicko-2 rating period processing with weighted outcomes
4. Output: each problem gets a triplet (rating, deviation, volatility)

**Matchup suggestion algorithm** (for guided sessions):
1. Prioritize items with high rating deviation (uncertain placement)
2. Prefer matchups between items with similar ratings (most informative)
3. Avoid pairs this user has already compared recently
4. Present ~5-10 matchups per session (configurable)

**Cold start**: New problems enter at rating 1500, deviation 350. ~3-5 comparisons typically sufficient to place them with reasonable confidence.

**Removed/solved problems**: Excluded from future matchups. Historical comparisons remain. Recalculating without them naturally adjusts remaining ratings.

### Storage

```
rankings/{milestone_id}/{user}.jsonl
```

Each file is an append-only log of comparisons by one user for one milestone:

```jsonl
{"winner":"<problem-id-A>","loser":"<problem-id-B>","ts":"2026-03-17T10:00:00Z"}
{"winner":"<problem-id-C>","loser":"<problem-id-A>","ts":"2026-03-17T10:05:00Z"}
```

**Why per-user JSONL files**:
- Zero merge conflicts (each user edits only their file)
- Append-only (no edits to existing lines)
- Syncs via `jj git push -b jjj`
- Ratings are derived state — recomputed from comparison history on read

### CLI Commands

```
jjj rank [<milestone>]          # Guided matchup session (5-10 pairs)
jjj rank show [<milestone>]     # Display computed ranking with ratings & confidence
jjj rank show --by-user         # Show where team members disagree
jjj rank history [<milestone>]  # Show comparison history
```

**Guided session flow** (`jjj rank`):

```
Ranking problems in milestone "v0.4 release" (8 open problems)

Which is more important to tackle?

  [A] Fix auth token expiry (#01957d)
  [B] Add batch import CLI (#019582)

  Press A, B, or S to skip: _
```

**Rank display** (`jjj rank show`):

```
Ranking for "v0.4 release" (12 comparisons from 3 users)

 #  Problem                      Rating   Conf   Comparisons
 1  Fix auth token expiry        1623     high   8
 2  Migrate to async runtime     1580     med    5
 3  Add batch import CLI         1512     low    2
 4  Refactor storage layer       1498     low    3
```

**Integration with `jjj next`**: TODO bucket sorts by Glicko-2 rating. Items with no comparisons (deviation > 300) are flagged as "unranked".

### TUI Integration

1. **Rating column in problem lists**: Show rank position, rating bar, and confidence indicator when viewing problems in a milestone context.

2. **Quick rank mode**: Press `r` to enter a ranking session inline. Pairs appear in the detail pane, user presses `a`/`b`/`s`.

3. **Sorting**: NextActions and ProjectTree problem lists sort by Glicko-2 rating when milestone context is active.

### Migration

- **Phase 1**: Add ranking system. `priority` field still accepted but ignored by sorting when rankings exist.
- **Phase 2**: Remove `priority` from the model. Existing YAML files with `priority` silently ignored.

No data migration needed — problems without comparisons start at default rating 1500 with high uncertainty.

## Summary Table

| Aspect | Design |
|--------|--------|
| What's ranked | Problems within milestones |
| Input | Pairwise comparisons ("A > B") |
| Algorithm | Glicko-2 (rating + deviation + volatility) |
| Weighting | Milestone owner comparisons weighted higher (configurable) |
| Storage | `rankings/{milestone_id}/{user}.jsonl` — append-only, per-user |
| Matchup selection | Uncertainty-maximizing (high deviation, similar ratings) |
| Cold start | Rating 1500, deviation 350; ~3-5 comparisons to place |
| CLI | `jjj rank` (guided), `jjj rank show`, `jjj rank history` |
| TUI | Rating bar + confidence in lists, `r` for inline ranking |
| Solution ranking | No change — critiques ARE the ranking |
| Priority tiers | Deprecated; Glicko-2 rating replaces them |
| Override | No separate mechanism; owner weight in Glicko-2 instead |
