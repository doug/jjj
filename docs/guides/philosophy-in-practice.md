---
title: Philosophy in Practice
description: How Popperian epistemology improves software engineering through jjj.
---

`jjj` is built on **Critical Rationalism**, a philosophy of knowledge developed by Karl Popper. The central idea: knowledge grows not by accumulating certainties, but by formulating bold conjectures and systematically attempting to refute them. This guide explains how these principles translate into better software engineering.

## 1. All Work is Problem-Solving

In traditional management, work arrives as "Features to Build." In `jjj`, we reframe everything as **Problems to Solve**.

> *   **Traditional**: "Add OAuth support."
> *   **Popperian**: "Users cannot authenticate with external providers (Problem: Authentication)."

This reframing matters. Starting with the problem forces clarity about *why* the work is needed, what success looks like, and when the problem is actually solved. A feature request can be completed without solving anything; a problem has clear resolution criteria.

It also enables decomposition: large problems can be split into sub-problems, each with its own solutions and critiques, forming a hierarchy that maps naturally to your codebase.

## 2. Solutions are Conjectures

When you write code, you are making a guess — a *conjecture* — that this code will solve the problem. In `jjj`, we make this explicit by calling it a **Solution**.

Because we acknowledge solutions might be wrong, we encourage proposing multiple competing approaches:

*   **Solution A**: Use Auth0 (delegated authentication)
*   **Solution B**: Implement a custom OAuth provider

Rather than picking one upfront in a design meeting, we can prototype both and let criticism determine which survives. The solutions exist as jj changes in your working tree; switching between them is `jjj solution resume`.

## 3. Criticism is Progress

The most important part of the process is **Critique**. In most code review systems, criticism is experienced as a blocker or an obstacle. In `jjj`, criticism is the engine of improvement — the mechanism by which errors get eliminated.

If someone critiques your solution, they are not attacking you; they are helping you find a flaw before it ships.

*   **Critique on Solution A**: "Auth0 increases vendor lock-in and adds a runtime dependency."
*   **Critique on Solution B**: "Custom OAuth increases maintenance burden and surface area for security bugs."

Each critique has a severity and must be explicitly resolved. A solution cannot be approved while open critiques remain. This enforces intellectual honesty: you cannot silently sweep concerns under the rug.

The solution that survives the most rigorous criticism is the one worth approving.

## 4. Refutation is a First-Class Outcome

In most tools, a rejected PR is a failure. In `jjj`, a **Withdrawn** solution is a valuable outcome.

If Solution A is withdrawn because its critique about vendor lock-in was validated, you have learned something true: this approach has a real flaw. That knowledge — the documented critique and the rationale — is captured in the event log permanently. The next time someone proposes a similar approach, the team has evidence.

"Closing without merging" is not wasted work; it's verified knowledge.

## 5. Knowledge vs. History

Git history tells you *what* changed. `jjj` tells you *why* it changed and *what criticism it survived*.

Over time, your repository becomes a verified body of knowledge, not just a series of snapshots. Every approved solution carries the record of what problems it addressed and what critiques it overcame. Every withdrawn solution carries the reason it failed. This is the difference between code that exists and code that is understood.

## Applying This Day to Day

You don't need to internalize Popper to use `jjj` effectively. The workflow is simple:

1. **Name the problem** before writing any code.
2. **Propose a solution** as a conjecture — you might be wrong.
3. **Critique explicitly** — your own work and others'. Be specific; give severities.
4. **Address or dismiss** each critique honestly. Document your reasoning.
5. **Approve when criticism is exhausted**, or withdraw and document why.

The result is a codebase where every decision has a traceable rationale — and a team that treats disagreement as progress.
