---
title: Cookbook
description: Step-by-step recipes for common jjj and Jujutsu workflows.
---

# Cookbook

This guide provides practical "recipes" for common scenarios you'll encounter while using `jjj` with Jujutsu.

## 1. Handling Merge Conflicts
When two solutions modify the same lines, Jujutsu will create a conflict. Here's how to handle it in a `jjj` context.

1.  **Identify the conflict**: `jj status` will show conflicted files.
2.  **Fix the code**: Edit the files and resolve the conflicts using standard `jj` patterns.
3.  **Update the solution**: If the fix was for a solution like "Use connection pooling", your change is already attached.
4.  **Critique the collision**: If the conflict revealed a deeper architectural issue, add a **Critique** to one or both solutions.
    ```bash
    jjj critique new "connection pooling" "This approach conflicts with the caching solution's logic." --severity medium
    ```

## 2. Decomposing a Large Feature
Large features should be broken down into manageable problems.

1.  **Create the parent problem**:
    ```bash
    jjj problem new "Build new search infrastructure"
    ```
2.  **Decompose into sub-problems**:
    ```bash
    jjj problem new "Implement elasticsearch indexing" --parent "search infrastructure"
    jjj problem new "Create search API endpoint" --parent "search infrastructure"
    ```
3.  **Solve independently**: Team members can now propose solutions for "elasticsearch indexing" and "search API endpoint" in parallel.

## 3. Self-Review and Draft Critiques
You can use `jjj` to document your own thought process before presenting code for review.

1.  **Propose your solution**: `jjj solution new "Optimistic UI update" --problem "flaky network"`
2.  **Critique your own work**: Document edge cases you're worried about.
    ```bash
    jjj critique new "Optimistic UI" "What happens if the network is flaky?" --severity low
    ```
3.  **Address it**: Link your subsequent commits to addressing this critique.
4.  **Signal readiness**: When you request a review, the reviewer can see what you've already considered.

## 4. Switching Between Competing Solutions
If you're trying two different approaches for the same problem:

1.  **Propose both**:
    ```bash
    jjj solution new "Approach A: GraphQL" --problem "API redesign"
    jjj solution new "Approach B: REST" --problem "API redesign"
    ```
2.  **Toggle with `jjj resume`**:
    ```bash
    jjj solution resume "GraphQL"  # Switches your jj workspace to this solution's change
    # ... work on GraphQL approach ...
    jjj solution resume "REST"     # Switches your jj workspace to this solution's change
    ```
3.  **Withdraw the loser**: Once one approach is proven better, withdraw the other with a rationale.
    ```bash
    jjj solution withdraw "GraphQL" --rationale "GraphQL introduced too much complexity for this use case."
    ```

## 5. Preparing for Code Review

Before requesting a review, use jjj to document your thinking so reviewers have context:

1.  **Submit for review** to signal it's ready for criticism:
    ```bash
    jjj solution submit "Add search index"
    ```
2.  **Add self-critiques** for known concerns you haven't fully resolved:
    ```bash
    jjj critique new "Add search index" "Index rebuild time on large datasets unknown" --severity medium
    ```
3.  **Assign a reviewer** via a review critique:
    ```bash
    jjj critique new "Add search index" "Review requested" --reviewer @alice
    ```
4.  **Ask Alice to review**: she runs `jjj status` and sees your solution in the REVIEW queue. When she raises a critique, you'll see it in BLOCKED. When all critiques are resolved, she signs off with `jjj solution lgtm "Add search index"`, and you can approve.

## 6. Tracking a Milestone

When planning a release, use milestones to group problems and track progress:

1.  **Create the milestone**:
    ```bash
    jjj milestone new "v1.0 Launch" --date 2025-09-01
    ```
2.  **Tag problems to the milestone**:
    ```bash
    jjj problem edit "Search is slow" --milestone "v1.0"
    jjj problem edit "Auth missing" --milestone "v1.0"
    ```
3.  **Check progress**:
    ```bash
    jjj milestone roadmap
    ```
    This shows which problems are open, in-progress, and solved for each milestone, so you can assess scope and schedule risk.

## 7. Handling an Abandoned Solution

If a team member leaves or a solution goes stale, you can cleanly hand it off or close it:

1.  **Check the current state**:
    ```bash
    jjj solution show "Old approach"
    ```
2.  **Reassign to yourself** to pick it up:
    ```bash
    jjj solution assign "Old approach"
    jjj solution resume "Old approach"  # Switch your workspace to this change
    ```
3.  **Or withdraw it** with a rationale if the approach is no longer viable:
    ```bash
    jjj solution withdraw "Old approach" --rationale "Superseded by the new caching architecture."
    ```

## 8. Detecting File Overlaps Early

When multiple solutions modify the same files, merge conflicts are likely. Detect them before they happen:

```bash
# Check for overlapping files
jjj overlaps
```

If overlaps exist, you have several options:
1. **Coordinate**: Talk to the other solution's author and agree on an approach.
2. **Sequence**: Withdraw one solution, approve the other first, then rebase.
3. **Critique**: Raise a critique on one of the solutions noting the conflict risk.
   ```bash
   jjj critique new "connection pooling" "Overlaps with caching solution on src/db/pool.rs" --severity medium
   ```

The `jjj status` command also shows overlap warnings automatically.

## 9. Claiming Work Items

Use `jjj next --claim` to atomically find the highest-priority item and assign it to yourself:

```bash
# Grab the top item
jjj next --claim

# See what you claimed in JSON
jjj next --claim --json
```

This is useful in team settings where multiple people may be looking for work at the same time. The claim is idempotent — running it again on an already-assigned item is a no-op.

## 10. Reviewing Project Health

Use `jjj insights` to get aggregate statistics about your project:

```bash
jjj insights
```

This shows approval rates, average time to solve problems, critique resolution times, and top contributors. Use `--json` for structured output that can feed into dashboards or reports.

## 11. Using Search to Find Context

Use full-text or semantic search to find related work before starting something new:

```bash
# Full-text search across all entities
jjj search "authentication"

# Search finds related entities even without exact match
jjj search "login security"

# Filter by type
jjj search "token" --type solution
```

This helps avoid duplicating work and surfaces critiques from previous related efforts.

## 12. Orienting at the Start of a Session

Before reading any listing, get the whole state in one call:

```bash
jjj prime
```

It prints what to work on, what is already proposed (so you do not build the
same thing twice), what has already been measured (so you do not re-measure
it), and anything blocked on a person. Its default output does not grow with the
corpus — the same handful of lines at fifty entities and at twenty-five
thousand — so it is cheap to run every time. `--full` lifts the per-section cap,
`--json` parses.

This replaces the habit of running `status`, then `problem list`, then
`solution list`, then `finding list`.

## 13. Recording Evidence So Nobody Measures It Twice

The result of an investigation is a **finding**, not a solution:

```bash
# What you measured, stated as a claim, with how to reproduce it
jjj finding new "Search is slow" "p99 is 2.3s at 10k docs" \
  --method "ab -n 1000 -c 10 against staging"

# The measurement happened before you wrote it down
jjj finding new "Search is slow" "p99 is 2.3s at 10k docs" \
  --method "ab -n 1000 -c 10" --measured-at 2026-10-16T14:00:00Z

# The conjecture that rests on it cites it
jjj solution new "Add a trigram index" --problem "Search is slow" --cites <finding>
```

When a better measurement arrives, supersede rather than edit:

```bash
jjj finding supersede <old> --by <new>
```

The old finding stays readable. Knowing a number was once believed, and what
corrected it, is what stops the same investigation being repeated — and the pair
read together is how you show that a change actually worked.

## 14. Filtering Without Full-Text Search

`jjj query` is a filter across all five kinds and needs no cache, unlike
`jjj search`:

```bash
# Open perf work, most recently edited first
jjj query "status:open tag:perf sort:edit limit:10"

# My submitted solutions
jjj query "type:solution assignee:mine status:submitted"

# How much of what, in one call instead of one listing per value
jjj query "type:problem" --count-by status

# Parents held back by unfinished sub-problems, and what blocks each
jjj query waiting
```

Use `sort:edit` rather than `sort:updated`: it orders by the causal clock, which
survives a clone whose system time is wrong.

## 15. Creating a Whole Graph in One Call

For a body of work with structure — sub-problems, the evidence behind them, the
conjectures that cite it — describe it once and apply it:

```bash
jjj plan apply plan.json --dry-run    # validate, write nothing
jjj plan apply plan.json --json       # apply, print key → id
```

Nodes reference each other by symbolic `key`, and any reference may instead name
an entity that already exists, so a plan can extend the graph rather than only
seed it. Nothing is written unless the whole plan validates. See
[Orientation & Planning](/reference/cli-orientation#jjj-plan-apply) for the
schema.

## 16. Editing Without Clobbering a Concurrent Change

Read the clock, then make the write conditional on it:

```bash
L=$(jjj problem show 01957d --json | jq '.lamport // 0')
jjj problem edit 01957d --append-body "re-ran at 50k docs: p99 4.1s" --if-lamport "$L"
```

A clock of `0` is omitted from JSON, hence the `// 0` default. A refused write
**exits 13**, which means "nothing was written, re-read and try again" — as
distinct from exit 1, which means the command itself was wrong and retrying will
not help. Use `--append-body` rather than `--body` when you are adding to a body
rather than replacing it; it reads the existing text inside the write
transaction, so there is no window in which an edit can be lost.

## 17. Asking for a Human

```bash
# Only for what a person must do: a dead credential, a destructive step
jjj escalate "the staging credential is dead" --about 01957d

# Answer without closing, so the next agent does not hit the same wall
jjj escalate --respond 01957d "rotated; new key is in the vault"

# Close it once it is actually fixed
jjj escalate --clear 01957d
```

`--respond` and `--clear` take an **id or prefix**, not the reason text. Keep
retries and routine progress out of this channel — one that carries them is one
nobody reads. `jjj prime` leads with open escalations and prints any reply.
