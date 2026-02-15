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
3.  **Update the solution**: If the fix was for Solution `s1`, your change is already attached.
4.  **Critique the collision**: If the conflict revealed a deeper architectural issue, add a **Critique** to one or both solutions.
    ```bash
    jjj critique new s1 "This approach conflicts with s2's caching logic." --severity medium
    ```

## 2. Decomposing a Large Feature
Large features should be broken down into manageable problems.

1.  **Create the parent problem**:
    ```bash
    jjj problem new "Build new search infrastructure"
    # Note: assume ID is p1
    ```
2.  **Decompose into sub-problems**:
    ```bash
    jjj problem new "Implement elasticsearch indexing" --parent p1  # p2
    jjj problem new "Create search API endpoint" --parent p1       # p3
    ```
3.  **Solve independently**: Team members can now propose solutions for `p2` and `p3` in parallel.

## 3. Self-Review and Draft Critiques
You can use `jjj` to document your own thought process before presenting code for review.

1.  **Propose your solution**: `jjj solution new "Optimistic UI update" --problem p5`
2.  **Critique your own work**: Document edge cases you're worried about.
    ```bash
    jjj critique new s1 "What happens if the network is flaky?" --severity low
    ```
3.  **Address it**: Link your subsequent commits to addressing this critique.
4.  **Signal readiness**: When you request a review, the reviewer can see what you've already considered.

## 4. Switching Between Competing Solutions
If you're trying two different approaches for the same problem:

1.  **Propose both**:
    ```bash
    jjj solution new "Approach A: GraphQL" --problem p1 # s1
    jjj solution new "Approach B: REST" --problem p1    # s2
    ```
2.  **Toggle with `jjj resume`**:
    ```bash
    jjj resume s1  # Switches your jj workspace to s1's change
    # ... work on s1 ...
    jjj resume s2  # Switches your jj workspace to s2's change
    ```
3.  **Refute the loser**: Once one approach is proven better, refute the other with a rationale.
    ```bash
    jjj solution refute s1 --rationale "GraphQL introduced too much complexity for this use case."
    ```
