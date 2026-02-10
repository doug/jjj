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
Created problem 01957d: Add user authentication
```

## Step 2: Alice Creates a Solution and Requests Bob's Review

Alice proposes a solution and requests Bob's review in one command.

```bash
# Alice's terminal
$ jjj solution new "JWT-based authentication" --problem "user auth" --reviewer bob
Created solution 01958a: JWT-based authentication
  Addresses: 01957d - Add user authentication
  Awaiting review: @bob
```

This automatically creates an "Awaiting review from @bob" critique that blocks acceptance until Bob addresses it.

Alice can verify what was created:

```bash
$ jjj critique list --solution "JWT"
ID       STATUS       SEVERITY   SOLUTION   TITLE
--------------------------------------------------------------------------------
01958b   open         low        01958a     Awaiting review from @bob
```

## Step 3: Bob Checks His Review Queue

Bob runs `jjj status` to see what needs his attention.

```bash
# Bob's terminal
$ jjj status
Next actions:

1. [REVIEW] c/01958b: Awaiting review from @bob -- Review requested on s/01958a
   -> jjj critique show "review"
```

Bob sees he has a review request. He examines the solution:

```bash
$ jjj solution show "JWT"
# Solution 01958a: JWT-based authentication

Status: proposed
Problem: 01957d (Add user authentication)
Assignee: alice

## Approach

(Alice's description of the approach would appear here)

## Open Critiques

- 01958b: Awaiting review from @bob [low]
```

## Step 4: Bob Reviews and Finds an Issue

Bob examines Alice's code and finds a security concern. He creates a critique:

```bash
# Bob's terminal
$ jjj critique new "JWT" "Token expiration is too long" --severity high
Created critique 01958c: Token expiration is too long on solution 01958a
  Severity: high
```

Bob keeps his review critique open because he's not done reviewing yet - he wants to see the fix before signing off.

## Step 5: Alice Sees the Critique and Responds

Alice checks her status:

```bash
# Alice's terminal
$ jjj status
Next actions:

1. [BLOCKED] s/01958a: JWT-based authentication -- 2 open critique(s)
   c/01958b: Awaiting review from @bob [low]
   c/01958c: Token expiration is too long [high]
   -> jjj critique show "expiration"
```

She views the critique details:

```bash
$ jjj critique show "expiration"
# Critique 01958c: Token expiration is too long

Solution: 01958a (JWT-based authentication)
Status: open
Severity: high
Author: bob

## Argument

(Bob's critique text would appear here)
```

Alice fixes the code and marks the critique as addressed:

```bash
# Alice fixes the token expiration in her code, then:
$ jjj critique address "expiration"
Critique 01958c marked as addressed
```

## Step 6: Bob Verifies the Fix

Bob's status now shows he needs to verify Alice's fix:

```bash
# Bob's terminal
$ jjj status
Next actions:

1. [REVIEW] c/01958b: Awaiting review from @bob -- Review requested on s/01958a
   -> jjj critique show "review"

2. [VERIFY] c/01958c: Token expiration is too long -- was addressed
   -> jjj critique show "expiration"
```

Bob checks that Alice's fix is good. If satisfied, he can dismiss his original critique (the expiration critique was addressed by Alice, but Bob raised it so he might want to validate):

```bash
# Bob checks the fix looks good
$ jjj critique show "expiration"
# Shows the addressed critique

# Bob is satisfied with the fix
```

## Step 7: Bob Completes His Review (LGTM)

Bob is now satisfied with the solution. He dismisses his review critique to sign off:

```bash
# Bob's terminal
$ jjj critique dismiss "review"
Critique 01958b dismissed (shown to be incorrect or irrelevant)
```

Dismissing the "Awaiting review" critique is Bob's sign-off. It means "I've looked at this and have no concerns."

## Step 8: Alice Accepts the Solution

Alice checks her status:

```bash
# Alice's terminal
$ jjj status
Next actions:

1. [READY] s/01958a: JWT-based authentication -- All critiques resolved
   -> jjj solution accept "JWT"
```

All critiques are resolved:
- 01958b (review request): dismissed by Bob (LGTM)
- 01958c (token expiration): addressed by Alice

Alice accepts the solution:

```bash
$ jjj solution accept "JWT"
Solution 01958a accepted
Solution accepted. Mark problem 01957d as solved? [y/N] y
Problem 01957d marked as solved
```

## Summary: The Complete Flow

```
Alice                              Bob
─────                              ───
1. problem new "Auth"
2. solution new --reviewer bob
   → creates solution, review req
                                   3. status → sees review request
                                   4. critique new "JWT" "Issue"
                                      → creates critique
5. status → sees critiques
6. (fixes code)
7. critique address "Issue"
                                   8. status → verify fix
                                   9. (checks fix is good)
                                   10. critique dismiss "review" (LGTM)
11. status → ready
12. solution accept "JWT"
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
jjj solution new "Feature" --problem "auth" --reviewer alice --reviewer bob --reviewer carol
```

Each reviewer gets their own review critique. All must be resolved before acceptance.

### Reviewer Severity

Specify severity for review requests:

```bash
jjj solution new "Critical fix" --problem "security issue" --reviewer bob:critical
```

This creates a critical-severity review critique, useful for signaling urgency.

### Multi-Round Review

Bob can raise multiple critiques during his review while keeping his review critique open. Only when fully satisfied does he dismiss it. This prevents the "forgot to sign off" problem.

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
