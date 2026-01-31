# Unified Critique and Review System

## Goal

Unify the separate "review" and "critique" systems into a single critique-based model. A request for review becomes a critique ("this person with valid insights hasn't looked at the solution"), eliminating the parallel reviewer/sign-off tracking.

## Current State

Two parallel systems that both block solution acceptance:

| Aspect | Critique | Review/Sign-off |
|--------|----------|-----------------|
| Entity | Separate model with lifecycle | Properties on Solution (`reviewers[]`, `sign_offs[]`) |
| Status | Open → Addressed/Valid/Dismissed | Binary: signed off or not |
| Discussion | Replies supported | Optional comment on sign-off only |

Awkward workflow:
1. Reviewer must create critique AND withhold sign-off (two actions)
2. System doesn't link reviewer's sign-off to their critiques
3. Reviewer must manually sign off after their critique is addressed

## Design

### Model Changes

**Critique** - Add `reviewer` field:

```rust
pub struct Critique {
    pub id: String,
    pub solution_id: String,
    pub title: String,
    pub status: CritiqueStatus,
    pub severity: CritiqueSeverity,
    pub author: Option<String>,
    pub reviewer: Option<String>,  // NEW: who should address/review this
    pub argument: String,
    pub evidence: String,
    pub file_path: Option<String>,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub code_context: Vec<String>,
    pub replies: Vec<Reply>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Solution** - Remove review fields:

```rust
pub struct Solution {
    // REMOVE: reviewers: Vec<String>
    // REMOVE: sign_offs: Vec<SignOff>
    // KEEP: force_accepted: bool (still useful for bypassing open critiques)
    // ... rest unchanged
}
```

**Acceptance logic:**

- Before: Block if open critiques OR unsigned reviewers
- After: Block if open critiques (simpler - review requests ARE critiques)

### CLI Changes

**Remove:**
- `jjj solution review <solution_id> <reviewers>`
- `jjj lgtm <solution_id> [--comment]`
- `jjj review @alice @bob` (shorthand)

**Modify `solution new`:**

```bash
# Request review (creates "Awaiting review" critique)
jjj solution new "Title" --problem p1 --reviewer @bob
jjj solution new "Title" --problem p1 --reviewer @bob:high  # with severity
jjj solution new "Title" --problem p1 --reviewer @alice --reviewer @bob:critical
```

Each `--reviewer` creates a critique:
- `title`: "Awaiting review from @bob"
- `author`: solution creator (the requester)
- `reviewer`: bob (who should review)
- `severity`: specified or default low
- `status`: Open

**Modify `critique new`:**

```bash
# Assign reviewer to any critique
jjj critique new s1 "Bug found" --reviewer @carol
```

**Modify `critique list`:**

```bash
jjj critique list --reviewer @me    # Assigned to me (awaiting my review)
jjj critique list --author @me      # I raised (may be blocking others)
```

### Workflow

**Alice creates solution and requests Bob's review:**

```bash
jjj solution new "Add connection pooling" --problem p1 --reviewer @bob
# → s1 created
# → c1 created: "Awaiting review from @bob" [low, reviewer: bob]
```

**Bob reviews and finds issues:**

```bash
jjj critique new s1 "Pool size is hardcoded" --severity medium
# → c2 created [author: bob]
# → c1 still open (Bob keeps his "I'm still looking" hold)
```

**Alice addresses:**

```bash
jjj critique address c2
```

**Bob checks fix, satisfied:**

```bash
jjj critique dismiss c1 --reason "Looks good"
# → Bob's review complete, c1 closed
```

**Alice accepts:**

```bash
jjj solution accept s1
# ✓ No open critiques (c1 dismissed, c2 addressed)
# → s1 accepted
```

**Multi-round review:**

Bob can raise multiple critiques across rounds while keeping c1 open. Only when fully satisfied does he dismiss c1. This prevents the "forgot to sign off" problem.

**Adding reviewers later:**

```bash
jjj critique new s1 "Awaiting review from @carol" --reviewer @carol --severity low
```

### Status Command Integration

Default shows everything relevant to current user, prioritized by actionability:

```bash
jjj status

# Actionable (your turn):
1. [REVIEW] s1 "Add connection pooling"
   c1: "Awaiting review from @you" [low]
   → jjj critique dismiss c1 --reason "No concerns"

2. [VERIFY] s2 "Fix auth bug" - your critique was addressed
   c5: "SQL injection risk" [high] - marked Addressed
   → jjj critique show c5
   → jjj critique dismiss c5 --reason "Fixed"

# Waiting (others' turn):
3. [WAITING] s3 - awaiting review from @carol
4. [WAITING] s4 - @bob hasn't addressed your critique c7
```

**Actionable states** (shown first):
- Critiques with `reviewer: @me` and status Open → I need to review
- Critiques with `author: @me` and status Addressed → I need to verify fix

**Waiting states** (shown after):
- My solution has open critiques from others → waiting for reviewers
- My critique is Open and not Addressed → waiting for author

`--all` flag shows everything in the system, not just current user's items.

### VS Code Extension Updates

**Remove from CLI wrapper (`vscode/src/cli.ts`):**
- `requestReview(solutionId, reviewers)` method
- `lgtm(solutionId, comment)` method

**Update in CLI wrapper:**
- `newSolution()` - add `reviewers` parameter for `--reviewer` flags
- `newCritique()` - add `reviewer` parameter for `--reviewer` flag
- `listCritiques()` - add `reviewer` filter parameter

**Update types (`vscode/src/cli.ts`):**
```typescript
// Solution type - remove:
// reviewers: string[]
// sign_offs: SignOff[]

// Critique type - add:
reviewer?: string;
```

**Update cache (`vscode/src/cache.ts`):**
- Remove `getSolutionsAwaitingReview()` if it exists
- Add `getCritiquesForReviewer(reviewer: string)` - critiques assigned to reviewer
- Add `getCritiquesAwaitingVerification(author: string)` - author's critiques marked Addressed

**Update entity document provider (`vscode/src/documents/`):**
- Solution view: remove reviewers/sign-offs section
- Critique view: show reviewer field if set

**Update tree view (if exists):**
- Remove "Awaiting Sign-off" category
- Update to show critiques assigned to current user

**Update tests:**
- Remove tests for review/lgtm commands
- Update solution fixtures to remove reviewers/sign_offs
- Add tests for new `--reviewer` functionality

### Workflow Command Updates

**`jjj submit`** (`src/commands/workflow.rs`):

Current logic:
1. Check for open critiques
2. Check for unsigned reviewers
3. Block if either fails (unless --force)

New logic:
1. Check for open critiques (this now includes "awaiting review" critiques)
2. Block if any open (unless --force)

The reviewer check is eliminated - it's now just a critique check.

**`jjj solution accept`** (`src/commands/solution.rs`):

Same simplification - remove reviewer sign-off check, just check for open critiques.

## Migration

1. Remove `reviewers` and `sign_offs` fields from Solution model
2. Remove SignOff struct
3. Add `reviewer` field to Critique model
4. Remove review-related CLI commands
5. Add `--reviewer` flag to `solution new` and `critique new`
6. Add `--reviewer` filter to `critique list`
7. Update status command categories
8. Update workflow.rs submit logic
9. Update solution.rs accept logic
10. Update VS Code extension (types, CLI wrapper, cache, views)
11. Update documentation
12. Update all tests

Existing solutions with `reviewers` or `sign_offs` will lose that data. This is acceptable as a breaking change for a pre-1.0 tool.

## Benefits

1. **Single system** - No parallel reviewer/critique tracking
2. **Explicit state** - "Awaiting review" is a visible, addressable critique
3. **Multi-round support** - Reviewer keeps hold until fully satisfied
4. **Simpler acceptance** - Just check for open critiques
5. **Better visibility** - `critique list --reviewer @me` shows pending reviews

## Open Questions

- **Identity:** How do we verify @bob is actually Bob? Currently uses git config (trust-based). Deferred to separate design.
