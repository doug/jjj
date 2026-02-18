---
title: Philosophy in Practice
description: How Popperian epistemology improves software engineering through jjj.
---

`jjj` is more than a tool; it's an implementation of **Critical Rationalism**, a philosophy of knowledge developed by Karl Popper. This guide explains how applying these principles to your daily work leads to better software.

## 1. All Work is Problem-Solving
In traditional management, tasks are often handed down as "Features to Build." In `jjj`, we reframe these as **Problems to Solve**.

> **Example**:
> *   **Traditional**: "Add OAuth support."
> *   **Popperian**: "Users currently cannot authenticate with external providers (Problem P1)."

By starting with a problem, you align the entire team on the *reason* for the work, rather than just the *instructions*.

## 2. Solutions are Conjectures
When you write code, you are making a guess (a conjecture) that this code will solve the problem. In `jjj`, we represent this as a **Solution**.

Because we acknowledge that solutions might be wrong, we encourage proposing multiple solutions:
*   **Solution S1**: Use Auth0.
*   **Solution S2**: Use a custom OAuth provider.

We don't "pick one" upfront. We build them (or prototype them) and let criticism decide which is better.

## 3. Criticism is Progress (Error Elimination)
The most important part of the process is **Critique**. In most code review systems, criticism is seen as a delay or a hurdle. In `jjj`, it is the engine of improvement.

If someone critiques your solution, they aren't attacking you; they are helping you eliminate an error.
*   **Critique C1**: "Auth0 increases our vendor lock-in."
*   **Critique C2**: "Custom OAuth increases our maintenance burden."

The solution that survives the most rigorous criticism is the one that becomes **Accepted**.

## 4. The Value of Refutation
In `jjj`, we don't fear "closing without merging." If a solution is **Refuted**, we have learned something valuable: we have eliminated an error. This is a first-class outcome, just as important as accepting a solution.

## 5. Knowledge vs. History
Git history tells you *what* changed. `jjj` tells you *why* it changed and *what criticism it survived*. Your repository becomes a verified body of knowledge, not just a series of snapshots.
