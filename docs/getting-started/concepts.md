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
| **Benchmark / Metric** | **Finding** | Evidence about a problem. Measured, not agreed to. |
| **Roadmap** | **Milestone** | Time-boxed cycles of problems to solve. |
| **Invalid Issue** | **Dissolved** | A problem found to be based on false premises. |
| **Merged** | **Approved** | A solution that has survived all current criticism. |
| **Closed / Won't Fix** | **Withdrawn** | A solution proven to be flawed by criticism. |

---

## The Core Model

### 1. Problems (The Starting Point)

Everything in `jjj` starts with a **Problem**. A problem is not just a bug; it's any situation that needs addressing — a performance gap, a missing capability, an unclear design. Problems can be decomposed into a hierarchy (DAG), where solving all sub-problems solves the parent.

**Problem lifecycle:** `open` → `in_progress` → `solved` / `dissolved`

- `open` — identified, no solution yet
- `in_progress` — at least one solution is proposed or in review
- `solved` — an approved solution resolves it (or all sub-problems are solved)
- `dissolved` — the problem was based on false premises and is no longer applicable

### 2. Solutions (Conjectures)

Instead of "making a fix," you propose a **Solution**. A solution is a conjecture — your best current guess about how to solve the problem. You can propose multiple competing solutions for the same problem. jjj tracks which jj change ID implements each solution, so metadata survives rebases.

**Solution lifecycle:** `proposed` → `submitted` → `approved` / `withdrawn`

- `proposed` — created, work in progress
- `submitted` — submitted for review; critiques may be raised
- `approved` — survived all criticism; solution is verified knowledge
- `withdrawn` — a critique proved it won't work; this is a valuable outcome, not a failure

### 3. Critiques (Error Elimination)

A **Critique** is how solutions improve. Criticism is not an attack; it's the mechanism of progress. Critiques block a solution from being approved until every one is resolved.

**Critique lifecycle:** `open` → `addressed` / `valid` / `dismissed`

- `addressed` — the solution was updated to handle the critique
- `valid` — the critique is correct; the solution should be withdrawn (use `jjj critique validate`)
- `dismissed` — the critique is incorrect or no longer relevant

**Severities:** `low`, `medium`, `high`, `critical` — affect how urgently `jjj status` surfaces the issue.

### 4. Findings (Evidence)

A **Finding** is something you measured about a problem. It is the one kind with
no approval path, and that is deliberate: a measurement is not something a
reviewer agrees to. A finding is either **current** or **superseded** by a better
measurement.

**Finding lifecycle:** `current` → `superseded`

- `current` — the best evidence available on this problem
- `superseded` — a later measurement replaced it (`jjj finding supersede <old> --by <new>`).
  The old finding is not deleted; what it said remains readable, which is how you
  see a number move.

Record *when the measurement was taken* with `--measured-at`, separately from
when it was written down — two findings filed in the same minute may describe
states a week apart. Solutions and critiques cite findings with `--cites`, which
is what distinguishes a conjecture grounded in evidence from one that is merely
plausible.

### 5. Milestones

**Milestones** are time-boxed goals that group problems together. They give you a roadmap view: which problems are targeted for which release, and how much progress has been made.

### 6. Knowledge Growth

The repository accumulates **verified knowledge**. An approved solution is not just merged code; it's documented evidence that a conjecture survived rigorous criticism. The event log (`jjj events`) records every state change and rationale, so you always know *why* a decision was made, not just *what* changed.
