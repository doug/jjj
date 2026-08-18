---
title: "Coordination: identity, conflicts, and deterministic resolution"
description: "Resolve the acting identity, find entities left conflicted by a fetch, and collapse one to a chosen side"
covers:
  - "whoami resolves actor, pod, and push bookmark"
  - "JJJ_USER and JJJ_POD override identity per process"
  - "A pod gets its own single-writer push bookmark"
  - "conflicts lists entities with unresolved markers"
  - "resolve --ours / --theirs strips markers"
  - "resolve emits a conflict_resolved event"
tags: [coordination, conflicts, identity, pods]
---

# Coordination

Several agents or people can write the same repository at once. Two mechanisms
keep that survivable: every writer has an **identity** (which decides the
bookmark it pushes, so writers never contend for one ref), and a fetch that
cannot merge a body cleanly leaves **conflict markers** for a human to settle
rather than silently picking a winner.

```jjj:setup
init
```

## Who Am I

With nothing configured, the actor comes from the jj user and the push bookmark
is the shared one:

```jjj
whoami
> actor:         Test User
> push bookmark: jjj
```

```jjj
whoami --json
> "actor": "Test User"
> "pod": null
```

`JJJ_USER` and `JJJ_POD` override it for a single process — which is how a
supervisor gives each agent it spawns a distinct identity without writing any
config:

```shell
JJJ_USER=carol JJJ_POD=pod7 $JJJ whoami
> actor:         carol
> pod:           pod7
```

The pod gets **its own** push bookmark. That is the whole trick behind pushing
from many processes at once: each one is the single writer of its own ref, so
there is nothing to race over:

```shell
JJJ_USER=carol JJJ_POD=pod7 $JJJ whoami
> push bookmark: jjj/pod7
```

```shell
JJJ_USER=dave JJJ_POD=pod8 $JJJ whoami
> push bookmark: jjj/pod8
```

## A Clean Repository Has No Conflicts

```jjj:setup
problem new "Rate limiter drops bursts" --priority high --force
```

```jjj
conflicts
> No unresolved conflicts.
```

```jjj
conflicts --json
> []
```

## Finding an Entity a Fetch Could Not Merge

When two clones edit the same body, the three-way merge writes both sides into
the file rather than choosing. Here is what that leaves behind:

```shell:setup
PROBLEM_FILE=$(ls .jj/jjj-meta/problems/*.md | head -1)
python3 - "$PROBLEM_FILE" << 'PYEOF'
import sys
path = sys.argv[1]
text = open(path).read()
head, sep, _body = text.partition("\n---\n")
open(path, "w").write(
    head + sep
    + "<<<<<<< local\nToken bucket, refill per second.\n"
    + "=======\nLeaky bucket, smooth drain.\n>>>>>>> remote\n"
)
PYEOF
```

`conflicts` finds it and says which entity needs attention:

```jjj
conflicts
> Rate limiter drops bursts
>~ 1 unresolved conflict
```

```jjj
conflicts --json
>~ "title": "Rate limiter drops bursts"
```

## Resolving to One Side

Capture the id, then collapse the entity to the local edit:

```jjj:setup
conflicts --json
>= CONFLICT_ID "id": "([0-9a-f-]{36})"
```

```jjj
resolve $CONFLICT_ID --ours --rationale "token bucket matches the upstream limiter"
> Resolved
```

The markers are gone and the chosen side is what remains:

```jjj
problem show $CONFLICT_ID
> Token bucket, refill per second.
>! <<<<<<<
>! Leaky bucket
```

Nothing is left for `conflicts` to report:

```jjj
conflicts
> No unresolved conflicts.
```

## The Resolution Is Auditable

Choosing a side is a decision, so it is recorded as one — with the rationale
attached:

```jjj
events --event-type conflict_resolved
> conflict_resolved
> token bucket matches the upstream limiter
```
