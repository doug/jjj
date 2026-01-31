# Code Review User Journey

This guide walks through a complete code review scenario with two developers: Alice (author) and Bob (reviewer).

## The Scenario

Alice is implementing a feature to add user authentication. She wants Bob to review her work before it's accepted.

## Prerequisites

Both Alice and Bob have:
- A clone of the same repository with jj and jjj initialized
- Their git identity configured (`git config user.name`)

## Step 1: Alice Creates the Problem

Alice identifies what needs to be built and creates a problem to track it.

```bash
# Alice's terminal
$ jjj problem new "Add user authentication"
Created problem p1 (Add user authentication)
```

## Step 2: Alice Creates a Solution and Requests Bob's Review

Alice proposes a solution and requests Bob's review in one command.

```bash
# Alice's terminal
$ jjj solution new "JWT-based authentication" --problem p1 --reviewer bob
Created solution s1 (JWT-based authentication)
  Addresses: p1 - Add user authentication
  Awaiting review: @bob
```

This automatically creates an "Awaiting review from @bob" critique (c1) that blocks acceptance until Bob addresses it.

Alice can verify what was created:

```bash
$ jjj critique list --solution s1
ID       STATUS       SEVERITY   SOLUTION   TITLE
--------------------------------------------------------------------------------
c1       open         low        s1         Awaiting review from @bob
```

## Step 3: Bob Checks His Review Queue

Bob runs `jjj status` to see what needs his attention.

```bash
# Bob's terminal
$ jjj status
Next actions:

1. [REVIEW] c1: Awaiting review from @bob -- Review requested on s1
   -> jjj critique show c1
```

Bob sees he has a review request. He examines the solution:

```bash
$ jjj solution show s1
# Solution s1: JWT-based authentication

Status: proposed
Problem: p1 (Add user authentication)
Assignee: alice

## Approach

(Alice's description of the approach would appear here)

## Open Critiques

- c1: Awaiting review from @bob [low]
```

## Step 4: Bob Reviews and Finds an Issue

Bob examines Alice's code and finds a security concern. He creates a critique:

```bash
# Bob's terminal
$ jjj critique new s1 "Token expiration is too long" --severity high
Created critique c2 (Token expiration is too long) on solution s1
  Severity: high
```

Bob keeps his review critique (c1) open because he's not done reviewing yet - he wants to see the fix before signing off.

## Step 5: Alice Sees the Critique and Responds

Alice checks her status:

```bash
# Alice's terminal
$ jjj status
Next actions:

1. [BLOCKED] s1: JWT-based authentication -- 2 open critique(s)
   c1: Awaiting review from @bob [low]
   c2: Token expiration is too long [high]
   -> jjj critique show c2
```

She views the critique details:

```bash
$ jjj critique show c2
# Critique c2: Token expiration is too long

Solution: s1 (JWT-based authentication)
Status: open
Severity: high
Author: bob

## Argument

(Bob's critique text would appear here)
```

Alice fixes the code and marks the critique as addressed:

```bash
# Alice fixes the token expiration in her code, then:
$ jjj critique address c2
Critique c2 marked as addressed
```

## Step 6: Bob Verifies the Fix

Bob's status now shows he needs to verify Alice's fix:

```bash
# Bob's terminal
$ jjj status
Next actions:

1. [REVIEW] c1: Awaiting review from @bob -- Review requested on s1
   -> jjj critique show c1

2. [VERIFY] c2: Token expiration is too long -- was addressed
   -> jjj critique show c2
```

Bob checks that Alice's fix is good. If satisfied, he can dismiss his original critique (c2 was addressed by Alice, but Bob raised it so he might want to validate):

```bash
# Bob checks the fix looks good
$ jjj critique show c2
# Shows the addressed critique

# Bob is satisfied with the fix
```

## Step 7: Bob Completes His Review (LGTM)

Bob is now satisfied with the solution. He dismisses his review critique to sign off:

```bash
# Bob's terminal
$ jjj critique dismiss c1
Critique c1 dismissed (shown to be incorrect or irrelevant)
```

Dismissing the "Awaiting review" critique is Bob's sign-off. It means "I've looked at this and have no concerns."

## Step 8: Alice Accepts the Solution

Alice checks her status:

```bash
# Alice's terminal
$ jjj status
Next actions:

1. [READY] s1: JWT-based authentication -- All critiques resolved
   -> jjj solution accept s1
```

All critiques are resolved:
- c1 (review request): dismissed by Bob (LGTM)
- c2 (token expiration): addressed by Alice

Alice accepts the solution:

```bash
$ jjj solution accept s1
Solution s1 accepted
Solution accepted. Mark problem p1 as solved? [y/N] y
Problem p1 marked as solved
```

## Summary: The Complete Flow

```
Alice                              Bob
─────                              ───
1. problem new "Auth"
2. solution new --reviewer bob
   → creates s1, c1 (review req)
                                   3. status → sees review request
                                   4. critique new s1 "Issue"
                                      → creates c2
5. status → sees critiques
6. (fixes code)
7. critique address c2
                                   8. status → verify fix
                                   9. (checks fix is good)
                                   10. critique dismiss c1 (LGTM)
11. status → ready
12. solution accept s1
```

## Key Concepts

### Review Requests are Critiques

When you use `--reviewer bob`, jjj creates an "Awaiting review from @bob" critique. This critique:
- Has `reviewer: bob` set
- Blocks solution acceptance until resolved
- Is resolved when Bob dismisses it (LGTM) or addresses it

### Multiple Reviewers

You can request multiple reviewers:

```bash
jjj solution new "Feature" --problem p1 --reviewer alice --reviewer bob --reviewer carol
```

Each reviewer gets their own review critique. All must be resolved before acceptance.

### Reviewer Severity

Specify severity for review requests:

```bash
jjj solution new "Critical fix" --problem p1 --reviewer bob:critical
```

This creates a critical-severity review critique, useful for signaling urgency.

### Multi-Round Review

Bob can raise multiple critiques during his review while keeping his review critique (c1) open. Only when fully satisfied does he dismiss c1. This prevents the "forgot to sign off" problem.

### Viewing Your Queue

```bash
# What needs my attention?
jjj status

# Reviews assigned to me
jjj critique list --reviewer @me --status open

# Critiques I raised that are addressed (need verification)
jjj critique list --author @me --status addressed
```

## Next Steps

- [Code Review Workflow](code-review.md) - Full workflow documentation
- [Critique Guidelines](critique-guidelines.md) - How to write effective critiques
