# Manual Ordering with Quadratic Voting

## Goal

Replace Glicko-2 pairwise ranking with direct manual ordering + quadratic voting. Each user maintains a personal ordering of problems per milestone. Orderings are aggregated via Borda count, boosted by quadratic votes, to produce a shared global ranking. All UX lives inside the existing tree view.

## Problem

Pairwise ranking (Glicko-2) requires many comparison sessions to converge, is slow for large lists, and feels disconnected from the tree view. Users want to directly arrange problems in priority order with minimal friction, while still supporting multi-user consensus.

## Design

### Data Model

**Per-user ordering file** at `rankings/{milestone_id}/{user_slug}.json`:

```json
{
  "order": ["problem_id_1", "problem_id_2", "problem_id_3"],
  "votes": {"problem_id_1": 2, "problem_id_3": 1},
  "updated_at": "2026-03-22T10:30:00Z"
}
```

- `order`: problem IDs in the user's preferred priority order (index 0 = highest)
- `votes`: quadratic vote allocations per problem (number of votes, not cost)
- `updated_at`: timestamp of last modification

**Aggregation** (computed on load, not persisted):

1. **Borda count**: For each user's ordering of N problems, rank 1 gets N points, rank 2 gets N-1, ..., rank N gets 1 point. Problems absent from a user's ordering get 0 points from that user.
2. **Owner weighting**: Milestone assignee's Borda scores are multiplied by 2x.
3. **QV boost**: Each vote K allocated to a problem adds K to its aggregated score. The cost to allocate K votes is K^2 from the user's budget.
4. **Budget**: Each user gets max(100, 2N) credits per milestone, where N = number of problems in the milestone.
5. **Final score**: Sum of weighted Borda scores + QV boosts across all users. Ties broken by earliest creation date.

### Quadratic Voting Mechanics

The marginal cost of each additional vote follows quadratic scaling:

| Votes on item | Total cost | Marginal cost of next |
|---------------|------------|-----------------------|
| 0             | 0          | 1                     |
| 1             | 1          | 3                     |
| 2             | 4          | 5                     |
| 3             | 9          | 7                     |
| K             | K^2        | 2K+1                  |

Removing a vote refunds the marginal cost: removing the Kth vote refunds 2K-1 credits.

### TUI Keybindings

All operate in Normal input mode within the existing tree view:

| Key | Action | Context |
|-----|--------|---------|
| `Shift+Up` | Move selected problem up in personal ordering | On a problem under a milestone |
| `Shift+Down` | Move selected problem down in personal ordering | On a problem under a milestone |
| `Shift+Right` | Drill into the tier the cursor is in (zoom to ~1/3) | On a problem under a milestone |
| `Shift+Left` | Zoom back out to parent tier level | While zoomed into a tier |
| `+` / `=` | Add a quadratic vote to selected problem | On a problem under a milestone |
| `-` | Remove a quadratic vote from selected problem | On a problem with votes > 0 |
| `g` | Toggle between personal and global ordering view | Anywhere |

### Recursive Tier Drilling

The ordered list is implicitly divided into 3 equal tiers (top, mid, bottom third). Shift+Right zooms into the tier containing the cursor, showing only those items. Within the zoomed view, the subset is again implicitly divided into 3 tiers, enabling further drilling.

**Example flow with 27 problems:**

1. **Level 0 (all 27)**: Roughly reorder with Shift+Up/Down. Items split into tiers of 9.
2. **Shift+Right on item in top tier** → **Level 1 (top 9)**: Refine ordering of highest-priority items. Split into tiers of 3.
3. **Shift+Right again** → **Level 2 (top 3)**: Fine-tune the final top-3 ordering.
4. **Shift+Left** → back to Level 1 (top 9).
5. **Shift+Left** → back to Level 0 (all 27).

This enables rapid triage: make O(N) coarse decisions first, then O(N/3) refinements in the tier you care about, recursively. Total decisions for full ordering: ~N instead of N log N comparisons.

**Display when zoomed:**
```
Milestone: v1.0  [Personal | Top > Top | Budget: 95/100]
  1. Fix auth token expiry  ★★
  2. Database performance
  3. XSS vulnerability      ★
  [Shift+← back to Top (9 items)]
```

The breadcrumb trail (e.g., "Top > Top") shows the zoom path. Items shown are a filtered view of the full ordering — reordering within the zoomed view updates positions in the full list.

### Tree View Display

**Personal view** (`[Personal]` in status bar):
```
Milestone: v1.0  [Personal | Budget: 95/100]
  1. Fix auth token expiry  ★★
  2. Database performance
  3. XSS vulnerability      ★
  4. CSS layout fix
```

**Global view** (`[Global]` in status bar):
```
Milestone: v1.0  [Global | 3 voters]
  1. Database performance    (score: 14)
  2. Fix auth token expiry   (score: 12)
  3. XSS vulnerability       (score: 8)
  4. CSS layout fix          (score: 3)
```

- Rank number prefix shows position
- Stars (★) show the current user's vote allocations
- In global view, show aggregated score
- Problems not yet in any ordering appear at the bottom as "Unranked"

### Auto-ordering of New Problems

When a new problem is added to a milestone (via `milestone_id` assignment):
- It is appended to the end of every user's ordering who has an ordering file for that milestone
- It appears as the last ranked item (lowest priority by default)

When a problem is removed from a milestone:
- It is removed from all ordering files for that milestone
- Vote credits allocated to it are refunded

### Storage Location

Reuses existing `rankings/` directory structure:
```
{.jj}/jjj/rankings/
  {milestone_id}/
    alice.json       # Alice's ordering + votes
    bob.json         # Bob's ordering + votes
```

This replaces the current `{user_slug}.jsonl` comparison files.

### What Gets Removed

| Component | File(s) |
|-----------|---------|
| Glicko-2 math | `src/ranking/glicko2.rs` |
| Matchup suggestion | `src/ranking/matchups.rs` |
| Pairwise comparison UI | `InputMode::Ranking` in `src/tui/app/mod.rs` |
| Ranking key handler | `handle_ranking_key()` in `src/tui/app/mod.rs` |
| Ranking renderer | Ranking overlay in `src/tui/ui.rs` |
| `RankingProblem` struct | `src/tui/app/mod.rs` |
| `jjj rank session` CLI | `src/commands/rank.rs` (session subcommand) |
| Comparison JSONL format | `src/ranking/store.rs` (comparison read/write) |

### What Gets Added/Modified

| Component | File(s) | Change |
|-----------|---------|--------|
| Ordering store | `src/ranking/store.rs` | Rewrite: load/save ordering + votes JSON |
| Borda + QV aggregation | `src/ranking/borda.rs` | New: aggregation algorithm |
| Reorder keys | `src/tui/app/mod.rs` | Add Shift+Up/Down handling |
| Tier drilling | `src/tui/app/mod.rs` | Add Shift+Left/Right for zoom in/out, tier breadcrumb state |
| Vote keys | `src/tui/app/mod.rs` | Add +/=/- handling |
| View toggle | `src/tui/app/mod.rs` | Add `g` key, `personal_view: bool` state |
| Tree display | `src/tui/tree.rs` | Rank prefix, vote stars, score display |
| Tree building | `src/tui/tree.rs` | Sort by ordering position (personal or global) |
| `jjj rank show` | `src/commands/rank.rs` | Adapt to show Borda+QV aggregated ranking |
| Help overlay | `src/tui/ui.rs` | Update keybinding help |
| Ranking mod | `src/ranking/mod.rs` | Remove Glicko-2 exports, add Borda exports |

### CLI Changes

- **Remove**: `jjj rank session` (pairwise comparison)
- **Keep**: `jjj rank show` (display aggregated ranking, adapted for Borda+QV)
- **Keep**: `jjj rank history` (could show ordering changes over time, or remove)
- **Add**: `jjj rank set <milestone> <problem_ids...>` (CLI ordering, optional)

## Verification

1. `cargo test` — all tests pass
2. TUI: Shift+Up/Down reorders problems within milestone
3. TUI: Shift+Right/Left drills into/out of tiers correctly
4. TUI: +/- allocates/deallocates votes with correct quadratic cost
5. TUI: `g` toggles between personal and global view
6. Multi-user: two ordering files produce correct Borda+QV aggregated ranking
7. Budget enforcement: cannot exceed max(100, 2N) credits
8. Problems added/removed from milestone update ordering files correctly
9. Tier drilling shows correct breadcrumb trail and filters items appropriately
