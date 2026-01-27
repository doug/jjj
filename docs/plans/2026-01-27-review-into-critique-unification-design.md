# Review-into-Critique Unification Design

## Problem

The tool has two separate gatekeeping mechanisms — **critiques** (mandatory, anyone can raise) and **reviews/LGTM** (optional, requested from specific people). They overlap conceptually but differ in implementation, creating 7 friction points:

1. LGTM has no metadata (no timestamp, no comment)
2. Review requirement is an implicit boolean (asymmetric with mandatory critique blocking)
3. LGTM from non-requested reviewers is recorded but ignored
4. No explicit review rejection/blocking mechanism
5. `jjj next` REVIEW category shows misleading "requested by {assignee}" text
6. Force-accepted solutions with open critiques aren't tracked
7. No CLI command to change `requires_review` per-solution

## Design Decision

**Merge reviews into the critique model.** Reviews become a feature of solutions, not a separate system. An LGTM is a structured sign-off. A review request assigns reviewers who must either raise critiques or sign off. This aligns with the Popperian model: everything is error-elimination.

## Data Model Changes

### Solution Model

**Remove:**
- `requires_review: bool`
- `lgtm: bool`
- `lgtm_by: Option<String>`

**Add:**
```rust
pub reviewers: Vec<String>,
pub sign_offs: Vec<SignOff>,
pub force_accepted: bool,
```

**New struct:**
```rust
pub struct SignOff {
    pub reviewer: String,
    pub at: DateTime<Utc>,
    pub comment: Option<String>,
}
```

**Derived logic (no stored fields):**
- "Requires review" = `!reviewers.is_empty()`
- "Review complete" = every entry in `reviewers` appears in `sign_offs` AND has no open critiques
- Non-assigned sign-offs are stored in `sign_offs` but don't affect the gate

### Critique Model

Unchanged. Critiques remain the universal blocking mechanism. There is no explicit rejection — reviewers who disagree raise critiques.

## CLI Command Changes

### Modified Commands

- `jjj solution new "title" --review @alice,@bob` — assign reviewers at creation (optional)
- `jjj solution review <id> @alice @bob` — add assigned reviewers (additive)
- `jjj solution lgtm <id> [--comment "looks good"]` — structured sign-off with timestamp. Works for anyone; gate only checks assigned reviewers.
- `jjj solution show <id>` — displays reviewers section: assigned, signed off (with timestamp/comment), pending

### Removed Commands

- `jjj review request` — replaced by `jjj solution review <id> @reviewer`
- The standalone `review` command namespace goes away entirely

### Unchanged Commands

- All `jjj critique` commands
- `jjj submit` (gate logic updated, command unchanged)

### `jjj next` Changes

- REVIEW category shows solutions where you are an assigned reviewer who hasn't signed off
- Description shows "review requested by {author}" instead of "requested by {assignee}"

## Acceptance Gate (submit logic)

```
fn submit(solution_id):
    1. Check critiques: any open critiques -> block with list
    2. Check reviewers: if reviewers assigned AND not all signed off -> block with pending list
    3. Both clear -> accept solution
    4. Auto-solve: if only solution on problem + no open sub-problems -> solve problem
```

Gates are sequential (critiques first, then reviewers) but conceptually unified: critiques block, reviewers are people who must confirm they have no critiques to raise.

**Force-accept:** `jjj submit --force` bypasses both gates. Records `force_accepted: true` on the solution.

## VS Code Extension Changes

### TypeScript Interfaces (`cli.ts`)

Remove from `Solution`: `requires_review`, `lgtm`, `lgtm_by`

Add to `Solution`:
```typescript
reviewers: string[];
sign_offs: Array<{ reviewer: string; at: string; comment?: string }>;
force_accepted: boolean;
```

### Entity Document Provider

Solution detail view replaces old "Review: LGTM by alice" with:
```
## Reviewers
- alice: signed off (2026-01-27) — "looks good"
- bob: pending
```

Non-assigned sign-offs shown separately: `Also endorsed by: charlie (2026-01-27)`

### Project Tree Provider

Solution node description shows `"2/3 reviewed"` or `"awaiting review"` instead of old LGTM indicator.

### Tests

- Update `entityDocument.test.ts` helpers for new fields
- Add tests for reviewer section rendering (assigned + signed off + pending)
- Add test for non-assigned endorsement display
- Update `cache.test.ts` solution helpers

## Storage & Migration

### New Frontmatter Format

```yaml
reviewers:
  - alice
  - bob
sign_offs:
  - reviewer: alice
    at: 2026-01-27T15:30:00Z
    comment: "looks good"
force_accepted: false
```

### Backwards Compatibility

Lazy migration on load:
- If `lgtm_by` is set: `sign_offs: [{reviewer: lgtm_by, at: updated_at, comment: null}]`
- If `requires_review` is true but no `lgtm_by`: `reviewers: []` (user re-assigns)
- Old fields dropped on next save

No bulk migration tool needed.
