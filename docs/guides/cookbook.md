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
