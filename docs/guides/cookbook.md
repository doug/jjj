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
3.  **Refute the loser**: Once one approach is proven better, refute the other with a rationale.
    ```bash
    jjj solution refute "GraphQL" --rationale "GraphQL introduced too much complexity for this use case."
    ```

## 5. Preparing for Code Review

Before requesting a review, use jjj to document your thinking so reviewers have context:

1.  **Submit for review** to signal it's ready for criticism:
    ```bash
    jjj solution review "Add search index"
    ```
2.  **Add self-critiques** for known concerns you haven't fully resolved:
    ```bash
    jjj critique new "Add search index" "Index rebuild time on large datasets unknown" --severity medium
    ```
3.  **Assign a reviewer**:
    ```bash
    jjj solution assign "Add search index" --reviewer @alice
    ```
4.  **Ask Alice to review**: she runs `jjj status` and sees your solution in the REVIEW queue. When she raises a critique, you'll see it in BLOCKED. When all critiques are resolved and she signs off, you can `jjj submit`.

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
    jjj solution assign "Old approach" --me
    jjj solution resume "Old approach"  # Switch your workspace to this change
    ```
3.  **Or refute it** with a rationale if the approach is no longer viable:
    ```bash
    jjj solution refute "Old approach" --rationale "Superseded by the new caching architecture."
    ```

## 8. Using Search to Find Context

Use full-text or semantic search to find related work before starting something new:

```bash
# Full-text search across all entities
jjj search "authentication"

# Semantic search (finds conceptually related even without exact match)
jjj search "login security" --semantic

# Filter by type
jjj search "token" --type solution
```

This helps avoid duplicating work and surfaces critiques from previous related efforts.
