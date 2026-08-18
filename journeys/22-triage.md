---
title: "Triage: what to work on, what collides, what the log says"
description: "next picks the highest-value action, overlaps warns about colliding solutions, tags and insights summarize the board"
covers:
  - "next on an empty repo"
  - "next surfaces the highest-priority action with the command to run"
  - "next --top N and --claim"
  - "tags counts tags in use"
  - "overlaps is quiet when solutions touch different files"
  - "overlaps names a file two active solutions both touch"
  - "insights aggregates the event log"
tags: [triage, next, overlaps, tags, insights]
---

# Triage

Four read-only commands answer the questions you actually have at the start of a
session: what should I do, what will I collide with, what is this project about,
and how are we doing.

```jjj:setup
init
```

## Nothing To Do Is a Real Answer

```jjj
next
> Nothing to do — all caught up!
```

```jjj
insights
> No events recorded yet.
```

```jjj
tags
> No tags in use.
```

## next Names the Action and the Command

```jjj:setup
problem new "Auth bypass on token refresh" --priority critical --tags security,auth --force
```

```jjj:setup
problem new "Slow dashboard query" --priority medium --tags perf --force
```

An open problem with no solution is a `TODO`, and `next` prints the exact
command that moves it forward — the output is meant to be pasted, not read:

```jjj
next
> [TODO]
> Auth bypass on token refresh
>~ -> jjj solution new
```

The critical problem outranks the medium one:

```jjj
next --json
>~ "title": "Auth bypass on token refresh"
```

`--top` widens the queue instead of showing only the front of it:

```jjj
next --top 2
> Auth bypass on token refresh
> Slow dashboard query
```

`--claim` takes the top item and assigns it to you in one step:

```jjj
next --claim
> Claimed:
> Auth bypass on token refresh
```

```jjj
problem list --assignee "Test User"
> Auth bypass on token refresh
```

## tags Summarizes What the Board Is About

```jjj
tags
>~ auth\s+1
>~ perf\s+1
>~ security\s+1
```

```jjj
tags --json
> "tag": "security"
```

## overlaps Warns Before Two Solutions Collide

Two solutions, each attached to its own jj change:

```jjj:setup
solution new "Rotate refresh tokens" --problem "Auth bypass" --force
```

```jjj:setup
solution new "Cache the dashboard aggregate" --problem "Slow dashboard" --force
```

While they touch different files there is nothing to warn about:

```shell:setup
jj new -m "work on auth" 'root()'
echo "fn rotate() {}" > auth.rs
```

```jjj:setup
solution attach "Rotate refresh"
```

```shell:setup
jj new -m "work on dashboard" 'root()'
echo "fn cache() {}" > dashboard.rs
```

```jjj:setup
solution attach "Cache the dashboard"
```

```jjj
overlaps
> No file overlaps between active solutions.
```

Now both changes touch a shared module. That is a merge conflict in waiting, and
`overlaps` is how you find out before you hit it rather than after:

```shell:setup
jj new -m "auth touches config" 'root()'
echo "fn rotate() {}" > auth.rs
echo "shared" > config.rs
```

```jjj:setup
solution attach "Rotate refresh" --force
```

```shell:setup
jj new -m "dashboard touches config" 'root()'
echo "fn cache() {}" > dashboard.rs
echo "shared" > config.rs
```

```jjj:setup
solution attach "Cache the dashboard" --force
```

```jjj
overlaps
> config.rs
> Rotate refresh tokens
> Cache the dashboard aggregate
```

The files each solution owns alone are not flagged — only the collision is:

```jjj
overlaps --json
> "config.rs"
```

## insights Reads the Event Log Back

Every state change is an event, so the aggregate view needs no separate
bookkeeping:

```jjj
insights
> Project Insights
>~ Total events: [0-9]+
> Top contributors:
> Test User
```

```jjj
insights --json
>~ "total_events": [0-9]+
```
