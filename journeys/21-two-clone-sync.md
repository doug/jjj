---
title: "Two clones: a review that crosses machines"
description: "Alice raises a problem, Bob solves it in his own clone, Alice critiques, Bob addresses, Alice approves — all through push and fetch"
mode: two-clone
covers:
  - "push publishes metadata to a shared remote"
  - "fetch adopts entities created in another clone"
  - "A solution written in clone 2 reaches clone 1"
  - "A critique written in clone 1 blocks approval in clone 2"
  - "Addressing a critique unblocks approval"
  - "Events from both clones merge into one log"
  - "Automation rules never travel between clones"
tags: [sync, push, fetch, multi-user, review]
---

# A Review Across Two Clones

This is the workflow jjj exists for, and the one a single-repo journey cannot
show: two people, two machines, one shared bookmark, no server. Blocks marked
`jjj:2` run in the second clone.

Both clones start attached to the same bare remote. Repo 1 is Alice; repo 2 is
Bob.

```jjj:setup
init
```

```jjj:2:setup
init
```

## Alice Raises a Problem and Publishes It

```jjj
problem new "Session tokens never expire" --priority critical --force
```

```jjj
push
> Pushed to origin.
```

Bob has not fetched yet, so his clone knows nothing about it:

```jjj:2
problem list
>! Session tokens never expire
```

## Bob Fetches and Picks It Up

```jjj:2
fetch
> Fetched from origin.
```

```jjj:2
problem list
> Session tokens never expire
```

The problem arrives whole, not as a summary — including the priority Alice set:

```jjj:2
problem show "Session tokens"
> critical
```

## Bob Proposes a Solution

```jjj:2
solution new "Add a 24h TTL and refresh flow" --problem "Session tokens" --force
```

```jjj:2
solution submit "Add a 24h TTL"
```

```jjj:2
push
> Pushed to origin.
```

## Alice Sees the Solution and Critiques It

```jjj
fetch
> Fetched from origin.
```

```jjj
solution list
> Add a 24h TTL and refresh flow
> submitted
```

```jjj
critique new "Add a 24h TTL" "Refresh tokens need rotation or the TTL buys nothing" --severity high
```

```jjj
push
```

## The Critique Blocks Approval in Bob's Clone

```jjj:2
fetch
```

```jjj:2
critique list
> Refresh tokens need rotation
```

Approval is refused while the critique is open — the block travels with the
metadata, so Bob cannot approve past feedback he has not answered:

```jjj:2:fail
solution approve "Add a 24h TTL" --no-rationale
>~ critique
```

## Bob Addresses It, Then Approves

```jjj:2:setup
critique list --json
>= CRITIQUE_ID "id": "([0-9a-f-]{36})"
```

```jjj:2
critique address $CRITIQUE_ID
```

```jjj:2
solution approve "Add a 24h TTL" --no-rationale
> approved.
```

```jjj:2
push
```

## Alice Sees the Approval and the Whole History

```jjj
fetch
```

```jjj
solution list
> approved
```

The event log carries both clones' work, in one timeline:

```jjj
events
> problem_created
> solution_created
> solution_submitted
> critique_raised
> critique_addressed
> solution_approved
```

The narrative view reads the same history in the language of the workflow:

```jjj
timeline "Session tokens"
> problem created
> proposed
> moved to review
```

Both actors are represented — the log is a record of who did what, not just
what happened:

```jjj
events --json
> "by": "Test User"
> "by": "Bob"
```

## Automation Never Crosses the Boundary

Bob adds a machine-local automation rule and pushes:

```shell:2:setup
cat > .jj/jjj-meta/automation.toml << TOMLEOF
[[automation]]
on = "problem_created"
action = "shell"
command = "echo BOBS_RULE >> $REPO2/.bob-marker"
TOMLEOF
```

```jjj:2
push
```

Alice fetches. Bob's rule is not in her clone, and never will be — automation
lives outside the synced metadata precisely so that pushing cannot hand anyone
else a command to run:

```jjj
fetch
```

```shell
test ! -f .jj/jjj-meta/automation.toml && echo NO_RULES_ARRIVED
> NO_RULES_ARRIVED
```

```jjj
automation list
> No automation rules are active.
```

Bob's own rule still works on Bob's machine:

```jjj:2
problem new "Unrelated local work" --force
```

```shell:2
cat $REPO2/.bob-marker
> BOBS_RULE
```
