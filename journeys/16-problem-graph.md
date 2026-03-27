---
title: "Problem Graph"
description: "Render problem hierarchies as ASCII DAG with --all and --milestone filters"
replaces: "uxr/scenarios/16-problem-graph.sh"
covers:
  - "3-level problem hierarchy rendered as tree"
  - "Multiple independent trees in one graph"
  - "Solved problems hidden by default, shown with --all"
  - "Milestone filter restricts graph to tagged problems"
tags: [graph, hierarchy, milestone, tree]
---

# Problem Graph

## Setup

```jjj:setup
init
```

## Step 1: Create 3-level hierarchy

```jjj
problem new "Authentication system" --priority high --force
> Authentication system
```

```jjj
problem new "Login flow" --parent "Authentication system" --priority medium --force
> Login flow
```

```jjj
problem new "OAuth2 integration" --parent "Login flow" --priority medium --force
> OAuth2 integration
```

3-level hierarchy created: root -> child -> grandchild.

## Step 2: problem graph shows all three with tree characters

```jjj
problem graph
> Authentication system
> Login flow
> OAuth2 integration
```

Graph renders all three levels with tree characters.

## Step 3: Add a second root; verify two separate trees shown

```jjj:setup
problem new "Performance monitoring" --priority low --force
```

```jjj:setup
problem new "Request latency tracking" --parent "Performance monitoring" --priority low --force
```

Add a second child to Authentication system so both branch characters appear:

```jjj:setup
problem new "Session management" --parent "Authentication system" --priority medium --force
```

```jjj
problem graph
> Authentication system
> Performance monitoring
> Login flow
> Session management
> Request latency tracking
```

Two independent trees shown with branch characters for non-last children and end-of-subtree characters for last children.

## Step 4: problem graph --all shows solved problems

Solve "OAuth2 integration" through the full solution lifecycle:

```jjj:setup
solution new "Implement OAuth2 flow" --problem "OAuth2 integration" --force
```

```jjj:setup
solution submit "Implement OAuth2 flow"
```

```jjj:setup
solution approve "Implement OAuth2 flow" --no-rationale
```

```jjj:setup
problem solve "OAuth2 integration"
```

Default graph hides solved problems:

```jjj
problem graph
>! OAuth2 integration
```

`--all` includes solved problems with a different icon:

```jjj
problem graph --all
> OAuth2 integration
```

`--all` includes solved/dissolved problems.

## Step 5: problem graph --milestone filters to milestone problems

```jjj
milestone new "Q1 Release" --date 2026-03-31
> Q1 Release
```

```jjj:setup
milestone add-problem "Q1 Release" "Authentication system"
```

```jjj:setup
milestone add-problem "Q1 Release" "Login flow"
```

```jjj
problem graph --milestone "Q1 Release"
> Login flow
>! Performance monitoring
>! Request latency tracking
```

`--milestone` filter restricts graph to problems in that milestone.
