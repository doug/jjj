---
title: Concepts & Terminology
description: Mapping traditional project management terms to jjj's Popperian concepts.
---

# Concepts & Terminology

`jjj` uses terms from **Critical Rationalism** (Popperian epistemology). If you're coming from traditional project management or Git workflows, this guide maps familiar concepts to jjj's vocabulary.

## Terminology Mapping

| Traditional Term | `jjj` Term | Philosophical Context |
| :--- | :--- | :--- |
| **Issue / Bug** | **Problem** | Work begins with a problem. Problems are fundamental. |
| **Branch / PR** | **Solution** | A solution is a *conjecture*—a tentative attempt that may be wrong. |
| **Review Comment** | **Critique** | Error elimination through explicit criticism. |
| **Roadmap** | **Milestone** | Time-boxed cycles of problems to solve. |
| **Invalid Issue** | **Dissolved** | A problem found to be based on false premises. |
| **Merged** | **Accepted** | A solution that has survived all current criticism. |
| **Closed / Won't Fix** | **Refuted** | A solution proven to be flawed by criticism. |

---

## The Core Model

### 1. Problems (The Starting Point)

Everything in `jjj` starts with a **Problem**. A problem is not just a bug; it's any situation that needs addressing — a performance gap, a missing capability, an unclear design. Problems can be decomposed into a hierarchy (DAG), where solving all sub-problems solves the parent.

**Problem lifecycle:** `open` → `in_progress` → `solved` / `dissolved`

- `open` — identified, no solution yet
- `in_progress` — at least one solution is proposed or in review
- `solved` — an accepted solution resolves it (or all sub-problems are solved)
- `dissolved` — the problem was based on false premises and is no longer applicable

### 2. Solutions (Conjectures)

Instead of "making a fix," you propose a **Solution**. A solution is a conjecture — your best current guess about how to solve the problem. You can propose multiple competing solutions for the same problem. jjj tracks which jj change ID implements each solution, so metadata survives rebases.

**Solution lifecycle:** `proposed` → `review` → `accepted` / `refuted`

- `proposed` — created, work in progress
- `review` — submitted for review; critiques may be raised
- `accepted` — survived all criticism; solution is verified knowledge
- `refuted` — a critique proved it won't work; this is a valuable outcome, not a failure

### 3. Critiques (Error Elimination)

A **Critique** is how solutions improve. Criticism is not an attack; it's the mechanism of progress. Critiques block a solution from being accepted until every one is resolved.

**Critique lifecycle:** `open` → `addressed` / `valid` / `dismissed`

- `addressed` — the solution was updated to handle the critique
- `valid` — the critique is correct; the solution should be refuted (use `jjj critique validate`)
- `dismissed` — the critique is incorrect or no longer relevant

**Severities:** `low`, `medium`, `high`, `critical` — affect how urgently `jjj status` surfaces the issue.

### 4. Milestones

**Milestones** are time-boxed goals that group problems together. They give you a roadmap view: which problems are targeted for which release, and how much progress has been made.

### 5. Knowledge Growth

The repository accumulates **verified knowledge**. An accepted solution is not just merged code; it's documented evidence that a conjecture survived rigorous criticism. The event log (`jjj events`) records every state change and rationale, so you always know *why* a decision was made, not just *what* changed.
