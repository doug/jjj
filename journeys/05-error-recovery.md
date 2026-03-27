---
title: "Error Recovery and Edge Cases"
description: "Invalid inputs, state violations, empty repos, boundary conditions, and error message quality"
replaces: "uxr/scenarios/05-error-recovery.sh"
covers:
  - "Commands outside a repository fail gracefully"
  - "Empty state commands return helpful messages"
  - "Invalid inputs rejected with useful errors"
  - "Invalid status transitions blocked"
  - "Duplicate detection with --force override"
  - "Critique/solution against wrong entity types"
  - "Approve blocked by open critiques"
  - "JSON output and sort flags"
  - "Milestone edge cases"
  - "Search and DB rebuild"
tags: [errors, edge-cases, validation]
---

# Error Recovery and Edge Cases

## Running jjj Outside a Repository

Commands should fail gracefully when run outside a jj repository:

```shell:fail
cd /tmp && $JJJ status
```

```shell:fail
cd /tmp && $JJJ problem list
```

## Setup

```jjj:setup
init
```

## Empty State Commands

All commands work gracefully on an empty project:

```jjj
status
```

```jjj
problem list
> No problems
```

```jjj:setup
solution list
```

```jjj:setup
critique list
```

```jjj:setup
milestone list
```

```jjj:setup
events
```

```jjj:setup
milestone roadmap
```

```jjj:setup
problem tree
```

## Invalid Inputs

Empty title should be rejected:

```jjj:fail
problem new ""
```

Very long titles are accepted:

```shell
$JJJ problem new "$(python3 -c "print('x' * 500)")" && echo "long title ok"
> long title ok
```

Special characters in titles are handled:

```shell
$JJJ problem new 'Fix "quoted" & <special> chars' && echo "special chars ok"
> special chars ok
```

Unicode in titles works:

```shell
$JJJ problem new "Fix emoji handling 🎉" && echo "unicode ok"
> unicode ok
```

## Invalid Status Transitions

```jjj:setup
problem new "Transition test" --priority medium --force
```

Cannot go from open to solved without an approved solution:

```jjj:fail
problem edit "Transition" --status solved
```

## Critique Without Solution

Critiques target solutions, not problems directly:

```jjj:fail
critique new "Transition test" "This is wrong" --severity low
```

## Duplicate Detection

```jjj:setup
problem new "Exact duplicate test" --force
```

Creating an exact duplicate title is rejected without --force:

```jjj:fail
problem new "Exact duplicate test"
```

Force create bypasses duplicate detection:

```jjj
problem new "Exact duplicate test" --force
> Exact duplicate test
```

## Solution for Non-existent Problem

```jjj:fail
solution new "Orphan solution" --problem "zzz-nonexistent"
```

## Critique for Non-existent Solution

```jjj:fail
critique new "zzz-nonexistent" "Ghost critique" --severity low
```

## Approve Solution With Open Critiques

```jjj:setup
problem new "Blocked approve test" --priority low --force
```

```jjj:setup
solution new "Blocked solution" --problem "Blocked approve" --force
```

```jjj:setup
solution submit "Blocked solution"
```

```jjj:setup
critique new "Blocked solution" "Blocking critique" --severity high
```

Approval is blocked by an open critique:

```jjj:fail
solution approve "Blocked solution" --no-rationale
> critique
```

Force approve bypasses the critique check:

```jjj
solution approve "Blocked solution" --force --no-rationale
```

## Double Operations (Idempotency)

```jjj:setup
problem new "Idempotency test" --priority low --force
```

```jjj:setup
solution new "Idemp solution" --problem "Idempotency" --force
```

Addressing a nonexistent critique fails:

```jjj:fail
critique address "zzz-no-such-critique"
```

## JSON Output Mode

```jjj
problem list --json
> [
```

```jjj:setup
solution list --json
```

```jjj:setup
milestone list --json
```

```jjj:setup
critique list --json
```

## Sort Flags

```jjj:setup
problem new "AAA first alphabetically" --priority low --force
```

```jjj:setup
problem new "ZZZ last alphabetically" --priority critical --force
```

```jjj:setup
problem list --sort title
```

```jjj:setup
problem list --sort priority
```

```jjj:setup
problem list --sort status
```

```jjj:setup
problem list --sort created
```

Invalid sort field is treated as default (no crash):

```jjj:setup
problem list --sort invalid_field
```

## Milestone Edge Cases

Milestone without a date:

```jjj
milestone new "No date milestone"
```

Milestone with a past date:

```jjj
milestone new "Past milestone" --date 2020-01-01
```

Adding the same problem to a milestone twice succeeds:

```jjj:setup
problem new "Double add test" --priority low --force
```

```jjj:setup
milestone add-problem "No date" "Double add"
```

```jjj
milestone add-problem "No date" "Double add"
```

## Search (FTS)

Search auto-syncs from markdown files:

```jjj
search "test"
```

Rebuild the database:

```jjj
db rebuild
```

Search finds results after rebuild:

```jjj
search "duplicate"
```
