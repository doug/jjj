---
title: "Automation Rules"
description: "Config-driven automation rules fire on jjj events with template expansion"
replaces: "uxr/scenarios/18-automation-rules.sh"
covers:
  - "Shell automation on problem_created"
  - "Template variable expansion"
  - "Disabled rules skipped"
  - "Cross-entity template vars (problem.title)"
  - "Auto-solve triggers problem_solved automation"
tags: [automation, config, shell]
---

# Automation Rules

## Setup

```jjj:setup
init
```

Configure automation rules that write to a marker file:

```shell:setup
cat > .jj/jjj-meta/config.toml << TOMLEOF
name = "automation-test"

[[automation]]
on = "problem_created"
action = "shell"
command = "echo 'CREATED: {{title}}' >> $REPO/.marker"

[[automation]]
on = "problem_solved"
action = "shell"
command = "echo 'SOLVED: {{title}}' >> $REPO/.marker"

[[automation]]
on = "problem_dissolved"
action = "shell"
command = "echo 'DISSOLVED: {{title}}' >> $REPO/.marker"

[[automation]]
on = "solution_submitted"
action = "shell"
command = "echo 'SUBMITTED: {{title}} for {{problem.title}}' >> $REPO/.marker"

[[automation]]
on = "solution_approved"
action = "shell"
command = "echo 'APPROVED: {{title}}' >> $REPO/.marker"

[[automation]]
on = "critique_raised"
action = "shell"
command = "echo 'CRITIQUE: {{title}}' >> $REPO/.marker"

[[automation]]
on = "critique_raised"
action = "shell"
command = "echo 'CRITIQUE2: {{title}}' >> $REPO/.marker"
enabled = false
TOMLEOF
```

## Problem Creation Fires Automation

```shell:setup
rm -f $REPO/.marker
```

```jjj
problem new "Fix login timeout" --priority high --force
> auto: shell
```

The shell action wrote the problem title to the marker file:

```shell
cat $REPO/.marker
> CREATED: Fix login timeout
```

## Disabled Rules Are Skipped

```jjj:setup
solution new "Add retry logic" --problem "Fix login timeout" --force
```

```jjj
critique new "Add retry logic" "Missing tests"
```

The enabled critique rule fired, but the disabled one did not:

```shell
cat $REPO/.marker
> CRITIQUE: Missing tests
>! CRITIQUE2:
```

## Multiple Rules for Same Event

Only one critique rule fired (the enabled one):

```shell
count=$(grep -c "^CRITIQUE:" $REPO/.marker) && echo "$count lines"
> 1 lines
```

## Solution Submit with Template Variables

```jjj:setup
critique address "Missing tests"
```

```jjj
solution submit "Add retry logic"
```

The `{{problem.title}}` template resolved across entities:

```shell
cat $REPO/.marker
> SUBMITTED: Add retry logic for Fix login timeout
```

## Solution Approve Fires Automation

```jjj
solution approve "Add retry logic" --force --no-rationale
```

Approval triggers both `solution_approved` and auto-solve triggers `problem_solved`:

```shell
cat $REPO/.marker
> APPROVED: Add retry logic
> SOLVED: Fix login timeout
```

## Problem Dissolve Fires Automation

```jjj
problem new "False alarm" --priority low --force
```

```jjj
problem dissolve "False alarm" --reason "was user error, not a bug"
```

```shell
cat $REPO/.marker
> DISSOLVED: False alarm
```
