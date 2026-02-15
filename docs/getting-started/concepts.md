---
title: Concepts & Terminology
description: Mapping traditional project management terms to jjj's Popperian concepts.
---

# Concepts & Terminology

`jjj` uses terms from **Critical Rationalism** (Popperian epistemology). If you're coming from traditional project management or Git workflows, this guide will help you translate those concepts.

## Terminology Mapping

| Traditional Term | `jjj` Term | Philosophical Context |
| :--- | :--- | :--- |
| **Issue / Bug** | [Problem](file:///docs/reference/cli-problem.md) | Work begins with a problem. Problems are fundamental. |
| **Branch / PR** | [Solution](file:///docs/reference/cli-solution.md) | A solution is a *conjecture*—a tentative attempt that may be wrong. |
| **Review Comment** | [Critique](file:///docs/reference/cli-critique.md) | Error elimination through explicit criticism. |
| **Roadmap** | [Milestone](file:///docs/reference/cli-milestone.md) | Time-boxed cycles of problems to solve. |
| **Invalid Issue** | **Dissolved** | A problem found to be based on false premises. |
| **Merged** | **Accepted** | A solution that has survived all current criticism. |
| **Closed / Won't Fix** | **Refuted** | A solution proven to be flawed by criticism. |

---

## The Core Model

### 1. Problems (The Starting Point)
Everything in `jjj` starts with a **Problem**. A problem is not just a bug; it's any "problematic state" that needs addressing. Problems can be decomposed into a hiearchy (DAG), where solving all sub-problems solves the parent.

### 2. Solutions (Conjectures)
Instead of "making a fix," you propose a **Solution**. A solution is a conjecture. You might propose multiple competing solutions for the same problem. This encourages experimentation and parallel thinking.

### 3. Critiques (Error Elimination)
A **Critique** is how we improve solutions. Criticism is not an attack; it's the mechanism of progress. In `jjj`, critiques block a solution from being accepted until they are addressed (by modifying the code) or dismissed (by explaining why the critique is invalid).

### 4. Knowledge Growth
The repository grows as **Knowledge**. An "Accepted" solution is not just code that was merged; it is a piece of documented knowledge that has survived our best attempts to prove it wrong.
