# Problem Solving

This guide covers the core of working with jjj: identifying problems, decomposing them, prioritizing work, and reaching resolution.

## When to Create a Problem

A problem in jjj represents something that needs to be addressed. You should create a problem when you encounter:

**Observed defects** -- Something is broken or behaving incorrectly.

```bash
jjj problem new "Login fails when email contains a plus sign" --priority high
```

**Feature requests** -- A capability is missing that users need.

```bash
jjj problem new "Users cannot export reports as PDF"
```

**Performance issues** -- The system works but not well enough.

```bash
jjj problem new "Search queries take over 5 seconds on large datasets" --priority high
```

**Technical debt** -- Internal quality issues that slow down development.

```bash
jjj problem new "Authentication module has no test coverage"
```

The key question is: "Is there a gap between how things are and how they should be?" If yes, create a problem.

## Problem Decomposition

Large problems are hard to solve directly. jjj supports decomposing problems into sub-problems using the `--parent` flag, forming a hierarchy (a directed acyclic graph).

### When to decompose

Decompose a problem when:

- It is too broad for a single solution (e.g., "improve performance" is vague; "reduce query time for user search" is actionable)
- Different parts require different expertise or can be worked on in parallel
- You want to track progress at a finer grain

### How to decompose

```bash
# Create the root problem
jjj problem new "Authentication system is unreliable"
# Created P-1

# Break it down into specific sub-problems
jjj problem new "Token refresh fails silently" --parent P-1
# Created P-2

jjj problem new "Session state lost after network interruption" --parent P-1
# Created P-3

jjj problem new "No retry logic for auth API calls" --parent P-1
# Created P-4
```

You can view the hierarchy as a tree:

```bash
jjj problem tree
```

Output:

```
P-1 Authentication system is unreliable [open, P2/medium]
  P-2 Token refresh fails silently [open, P2/medium]
  P-3 Session state lost after network interruption [open, P2/medium]
  P-4 No retry logic for auth API calls [open, P2/medium]
```

### Depth guidelines

- **1 level deep** is typical: a root problem with a few sub-problems.
- **2 levels** is occasionally useful for large initiatives.
- If you need 3+ levels, reconsider whether you are modeling a problem or an organizational structure. Milestones may be a better fit for grouping work at the top level.

## Priority Guidelines

Every problem has a priority level. The default is P2/medium, which is appropriate for most work. Adjust the priority to reflect urgency and impact.

### P0 / Critical

The system is down, data is being lost, or there is a security vulnerability. Drop everything and work on this.

```bash
jjj problem new "Database credentials exposed in public log" --priority critical
```

Examples:
- Production system is completely unavailable
- Active data corruption or data loss
- Security breach or vulnerability being exploited
- Regulatory compliance violation

### P1 / High

A major feature is broken or there is significant performance degradation. This should be addressed in the current work cycle.

```bash
jjj problem new "Payment processing fails for international cards" --priority high
```

Examples:
- Major feature broken for a significant portion of users
- Performance degradation making a feature unusable
- Blocking issue for an upcoming release
- Data integrity issue that is not yet causing loss

### P2 / Medium (default)

Normal work items. Bugs that have workarounds, enhancements, and planned improvements. This is the priority for most day-to-day work.

```bash
jjj problem new "Add dark mode support for settings page"
# Priority defaults to P2/medium
```

Examples:
- Minor bugs with known workarounds
- New features and enhancements
- Refactoring and code quality improvements
- Documentation gaps

### P3 / Low

Nice-to-have improvements. Cosmetic issues, minor optimizations, and polish items. Work on these when higher-priority problems are resolved.

```bash
jjj problem new "Align button spacing on mobile nav" --priority low
```

Examples:
- Cosmetic or visual polish
- Minor performance optimizations
- Edge cases that rarely occur
- Developer experience improvements

### Adjusting priority

Priorities are not permanent. Reassess when context changes:

```bash
jjj problem edit P-5 --priority high
```

A P3 cosmetic issue becomes P1 if your CEO is demo-ing the product tomorrow. A P1 bug becomes P3 if a workaround is found and the affected feature is being replaced.

## Resolving Problems

Problems end in one of two ways: they are **solved** or they are **dissolved**.

### Solving a problem

A problem is solved when an accepted solution addresses it and there are no remaining open sub-problems. The typical flow:

1. Create one or more solutions for the problem
2. Solutions face critique and testing
3. A solution is accepted (all critiques resolved, reviews passed)
4. Mark the problem as solved

```bash
jjj problem solve P-1
```

If the problem has open sub-problems or no accepted solution, jjj will warn you. You can still proceed, but the warning exists to prevent premature closure.

### Dissolving a problem

Sometimes a problem turns out not to be a real problem. It was based on a false premise, it is a duplicate of another problem, or it was specific to an environment that no longer applies. In these cases, dissolve the problem rather than solving it.

Always provide a reason so future readers understand why:

```bash
# False premise
jjj problem dissolve P-7 --reason "The data was correct; our validation rule was wrong"

# Duplicate
jjj problem dissolve P-12 --reason "Duplicate of P-3, which already has solutions in progress"

# Environment-specific
jjj problem dissolve P-15 --reason "Only reproducible on the old CI image; resolved by infrastructure upgrade"
```

The `--reason` flag is optional but strongly recommended. A dissolved problem without a reason is a mystery for anyone who encounters it later.

### When to dissolve vs. solve

- **Dissolve** when the problem itself was wrong -- it did not actually exist, or the premise was false.
- **Solve** when the problem was real and a solution addressed it.
- If you are unsure, ask: "Did we build or change something to make this go away?" If yes, solve. If no, dissolve.

## Problems and Milestones

Milestones group problems for release planning and delivery tracking. Assigning a problem to a milestone signals that it should be addressed within that milestone's timeframe.

```bash
# Assign during creation
jjj problem new "Implement SSO" --milestone M-2 --priority high

# Or assign later
jjj milestone add-problem M-2 P-10
```

View all problems in a milestone:

```bash
jjj milestone show M-2
```

Use milestones to answer questions like:
- "What problems must be solved before we can release v2.0?"
- "Are we on track for the Q1 deadline?"
- "Which high-priority problems are not yet assigned to a milestone?"

## Putting It Together

Here is a typical problem-solving workflow from start to finish:

```bash
# 1. Identify the problem
jjj problem new "Search results include deleted items" --priority high

# 2. Investigate and decompose if needed
jjj problem new "Soft-deleted records not filtered in search index" --parent P-20
jjj problem new "Cache not invalidated on delete" --parent P-20

# 3. Propose solutions for the sub-problems
jjj solution new "Add deleted_at filter to search query" --problem P-21
jjj solution new "Invalidate search cache entry on soft delete" --problem P-22

# 4. Work on solutions (attach changes, face critique, iterate)
jjj solution resume S-5
# ... implement fix, attach change ...
jjj submit

# 5. Once sub-problems are solved, solve the parent
jjj problem solve P-21
jjj problem solve P-22
jjj problem solve P-20

# 6. Assign to milestone for release tracking
jjj milestone add-problem M-3 P-20
```

## Next Steps

- [Critique Guidelines](critique-guidelines.md) -- Learn how critiques evaluate solutions
- [Code Review](code-review.md) -- Understand the review flow for solutions
- [Board and Dashboard](board-dashboard.md) -- Visualize your work
