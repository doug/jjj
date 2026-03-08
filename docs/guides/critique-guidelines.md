---
title: Critique Guidelines
description: Best practices for writing effective critiques with appropriate severity levels
---

# Critique Guidelines

Critiques are the mechanism for error elimination in jjj. Every solution is a conjecture -- a tentative attempt to solve a problem. Critiques subject those conjectures to explicit criticism, which is how we make progress: by discovering and eliminating errors rather than by confirming what we already believe.

This guide covers how to write effective critiques, when to use each severity level, and how to respond to critiques on your own solutions.

## The Philosophical Basis

jjj's critique system is grounded in a Popperian approach to knowledge: all knowledge begins with problems, solutions are conjectures, and progress comes through criticism and error elimination.

A solution cannot be approved until all critiques against it are resolved. This is not bureaucracy -- it is an epistemological requirement. If a criticism stands unaddressed, the solution has a known flaw. Approving a flawed solution when the flaw has been identified would be irrational.

This means:

- Every critique must be resolved (addressed, dismissed, or validated) before a solution can be approved.
- "Resolved" does not mean "agreed with." Dismissing a critique is a legitimate resolution, provided you have a reason.
- The goal is not consensus but error elimination. A single valid critique can refute an otherwise popular solution.

## Severity Levels

When raising a critique, assign a severity that reflects the impact of the issue on the solution's viability.

### Critical

The critique identifies a flaw that definitively invalidates the solution. If this critique stands, the solution should be withdrawn.

```bash
jjj critique new "parameterized queries" "SQL injection in user input handling" --severity critical
```

Examples:
- Security vulnerability that exposes user data
- Data loss or corruption under normal usage
- Crash or undefined behavior in a core path
- Violation of a regulatory or contractual requirement

### High

A significant problem that may invalidate the solution. The solution cannot be accepted without addressing this.

```bash
jjj critique new "concurrent writes" "Race condition in concurrent write path" --severity high
```

Examples:
- Correctness issue under specific but realistic conditions
- Race condition or deadlock potential
- Missing validation at a trust boundary (e.g., API input not sanitized)
- Failure mode that silently corrupts state

### Medium (default)

A legitimate concern that should be addressed but does not necessarily invalidate the approach. Most critiques fall here.

```bash
jjj critique new "error handling" "No tests for the error handling path"
# Severity defaults to medium
```

Examples:
- Design concern (coupling, abstraction level, API ergonomics)
- Missing test coverage for important behavior
- Unclear or misleading naming
- Performance concern that may matter at scale

### Low

A minor observation. Worth noting but not a blocker.

```bash
jjj critique new "naming conventions" "Variable name 'x' could be more descriptive" --severity low
```

Examples:
- Style or formatting issues
- Minor optimization opportunities
- Documentation gaps in non-public code
- Suggestions for alternative approaches that are roughly equivalent

## Responding to Critiques

When you receive a critique on your solution, you have three options. Each one resolves the critique, but with different implications.

### Address the critique

Use this when the critique identifies a real issue and you have modified the solution to fix it. This is the most common response.

```bash
jjj critique address "race condition"
```

What it means: "You were right. I have changed the solution to handle this."

After addressing, the solution can proceed toward approval (assuming no other open critiques remain).

### Dismiss the critique

Use this when the critique does not apply to this solution, is based on a misunderstanding, or identifies something that is not actually a problem.

```bash
jjj critique dismiss "race condition"
```

What it means: "I have considered this criticism and it does not apply. Here is why."

You should explain your reasoning. Use the reply mechanism:

```bash
jjj critique reply "race condition" "This path is only reachable from the admin API, which already validates input upstream. See the solution's approach section for the trust model."
```

Dismissing without explanation is technically valid but makes it harder for others to understand your reasoning.

### Validate the critique

Use this when the critique is correct and the flaw it identifies is fundamental enough that the solution should be withdrawn. This is the honest thing to do when a critique reveals that your approach will not work.

```bash
jjj critique validate "sql injection"
```

What it means: "This criticism is correct. The solution is fundamentally flawed."

After validation, the typical next step is to withdraw the solution and propose a new one (potentially noting what was learned):

```bash
jjj solution withdraw "direct queries"
jjj solution new "Use parameterized queries for all DB access" --problem "db security" --supersedes "direct queries"
```

## Writing Effective Critiques

A good critique is specific, evidence-based, and actionable. It should make the problem clear enough that someone can evaluate whether it is valid.

### Be specific about the flaw

Bad: "This approach seems risky."
Good: "The `update_balance` function reads and writes without a transaction, so concurrent calls can produce incorrect totals."

### Provide evidence or reasoning

Bad: "This will be slow."
Good: "This performs N+1 queries -- one per user in the result set. For the typical page size of 50, that is 51 queries per request. The existing batch endpoint handles this in 2 queries."

### Point to the relevant code when possible

For code-level critiques, reference the specific location:

```bash
jjj critique new "search engine" "Unbounded memory growth from accumulating results" \
  --severity high \
  --file src/search/engine.rs \
  --line 142
```

This makes it easy for the solution author to find and evaluate the concern.

### Suggest a direction (but do not prescribe)

A critique identifies what is wrong. If you have an idea for how to fix it, mention it, but recognize that the solution author may find a better approach:

```bash
jjj critique reply "memory growth" "One approach would be to use a streaming iterator here instead of collecting into a Vec, but there may be other ways to bound the memory usage."
```

### One issue per critique

If you notice three problems, create three critiques. This allows each one to be addressed, dismissed, or validated independently.

## Example Workflow

Here is a complete critique lifecycle, from raising a critique through resolution.

```bash
# Alice proposes a solution
jjj solution new "Cache search results in Redis" --problem "slow search"
# Created 01958a: Cache search results in Redis

# Bob reviews and raises a critique
jjj critique new "Redis" "Cache invalidation not handled on data updates" --severity high
# Created 01958b: Cache invalidation not handled on data updates

# Alice and Bob discuss
jjj critique reply "invalidation" "Good point. What about TTL-based expiration?"
jjj critique reply "invalidation" "TTL alone is insufficient -- stale data is visible for the TTL window. We need event-driven invalidation for writes."

# Alice addresses the critique by modifying the solution
# ... updates the approach to include write-through invalidation ...
jjj critique address "invalidation"

# Carol raises a low-severity critique
jjj critique new "Redis" "Redis client library is unmaintained" --severity low
# Created 01958c: Redis client library is unmaintained

# Alice dismisses with explanation
jjj critique reply "unmaintained" "The library had a release last month and has active maintainers. The GitHub issue that flagged it as unmaintained was from 2023 and has since been closed."
jjj critique dismiss "unmaintained"

# All critiques resolved -- solution can now be accepted
jjj solution approve "Redis caching"
```

## Critiques and Sign-offs: Two Gates to Acceptance

jjj has two mechanisms that gate solution approval, both unified under the solution model:

1. **Critiques** -- Evaluate the solution's approach, design, and correctness. Anyone can raise a critique at any time. All critiques must be resolved (addressed, dismissed, or validated).

2. **Reviewer sign-offs** -- Assigned reviewers must sign off before the solution can be approved. Sign-offs are recorded when a reviewer addresses their review critique.

Both gates must be satisfied for `jjj solution approve` to succeed (unless `--force` is used). The acceptance check runs in order: first critiques, then sign-offs.

Review is per-solution: assign reviewers with `--reviewer` when creating a solution, or add review critiques later. Solutions without assigned reviewers skip the sign-off gate entirely.

```bash
# Assign reviewers at creation
jjj solution new "Add caching" --problem "performance" --reviewer @alice --reviewer @bob

# Or add a review critique later
jjj critique new "caching" "Review requested" --reviewer @alice

# Reviewer signs off by addressing their review critique
jjj critique list --solution "caching" --reviewer @alice
jjj critique reply "review" "LGTM - clean implementation"
jjj critique address "review"
```

See the [Code Review guide](code-review.md) for the full sign-off workflow.

## Next Steps

- [Problem Solving](problem-solving.md) -- When and how to create problems
- [Code Review](code-review.md) -- The reviewer sign-off flow
- [TUI and Status](board-dashboard.md) -- Visualize critiques and solutions
