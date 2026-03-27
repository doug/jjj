---
title: "Critique Depth"
description: "Full critique lifecycle: annotations, reviewer, filters, edit, reply, dismiss, validate, and blocking behavior"
replaces: "uxr/scenarios/08-critique-validate.sh"
covers:
  - "File/line annotations on critiques"
  - "Reviewer assignment and filtering"
  - "Critique list filters: --status, --solution, --reviewer"
  - "Critique edit: severity and title"
  - "Critique reply (comment threading)"
  - "Critique dismiss"
  - "Critique validate hard-blocks approve"
  - "Validate -> address -> approve flow"
  - "Validate -> withdraw flow"
  - "Critique show --json"
tags: [critique, validate, dismiss, reply, annotations]
---

# Critique Depth

## Setup

```jjj:setup
init
```

```jjj:setup
problem new "API lacks input validation" --priority critical --force
```

```jjj:setup
solution new "Add JSON schema validation" --problem "API lacks" --force
```

## Step 1: Critique with file and line annotations

Create a critique with `--file` and `--line` to pinpoint where in the code the issue lives:

```jjj
critique new "Add JSON schema" "Schema validator does not reject unknown fields" --severity high --file "src/api/validate.rs" --line 42
> Schema validator
> validate.rs
> 42
```

File and line annotations let reviewers jump directly to the problem in code.
Critique show displays reviewer, file, and line in plain text output:

```jjj
critique show "unknown fields"
> validate.rs
> 42
```

```jjj
critique show "unknown fields" --json
> "file_path"
> src/api/validate.rs
> "line_start"
```

## Step 2: Critique with reviewer

```jjj
critique new "Add JSON schema" "No rate limiting on validation endpoint" --severity medium --reviewer "bob@example.com"
```

Reviewer appears in JSON output:

```jjj
critique show "rate limiting" --json
> bob@example.com
```

Assigning a reviewer makes ownership explicit from the start.

## Step 3: Critique list filters

```jjj
critique list --solution "JSON schema"
> unknown fields
> rate limiting
```

```jjj
critique list --status open
> unknown fields
```

Reviewer filter uses substring matching -- partial email or username works:

```jjj
critique list --reviewer "bob@example.com"
> rate limiting
```

```jjj
critique list --reviewer "bob"
> rate limiting
```

```jjj
critique list --json
> "id"
> "title"
> "status"
```

## Step 4: Critique edit

```jjj
critique edit "rate limiting" --severity high
```

```jjj
critique show "rate limiting"
> high
```

```jjj
critique edit "rate limiting" --title "Validation endpoint has no rate limiting — DoS risk"
```

```jjj
critique show "DoS risk"
> DoS risk
```

Critique edit lets you refine severity and title as understanding improves.

## Step 5: Critique reply (comment threading)

```jjj
critique reply "unknown fields" "The strictMode option handles this — see schema.json line 8"
> reply
```

```jjj:setup
critique reply "unknown fields" "Confirmed — strictMode only applies to top-level keys, not nested objects"
```

```jjj
critique show "unknown fields"
```

Replies keep the discussion in context alongside the critique.

## Step 6: Critique dismiss

```jjj
critique dismiss "unknown fields"
> dismissed
```

```jjj
critique list --status dismissed
> unknown fields
```

Dismissed critiques should not appear in the open list:

```jjj
critique list --status open
>! unknown fields
```

Dismissed critiques are archived, not deleted -- the reasoning remains visible.

## Step 7: Validate hard-blocks approve; address then approve

The DoS risk critique is still open -- validate it to confirm the flaw is real:

```jjj
critique validate "DoS risk"
> validated
```

```jjj
critique list --status valid
> DoS risk
```

Validate means: this critique is confirmed correct -- the solution has a flaw.

Submit the solution so that approve hits the critique check, not a state check:

```jjj:setup
solution submit "JSON schema"
```

Valid critiques hard-block approval (same as open critiques):

```jjj:fail
solution approve "JSON schema" --no-rationale
```

Validated critiques hard-block approval -- must resolve them first.

Correct flow: address (or dismiss) the blocking critique, then approve:

```jjj:setup
critique address "DoS risk"
```

```jjj
solution approve "JSON schema" --no-rationale
> approved
```

Address or dismiss a validated critique to unblock approval.
Convention: if a critique is validated, fix the flaw, address the critique, then approve.

## Step 8: Validate then withdraw

Demonstrate the proper withdraw flow with a new solution:

```jjj:setup
solution new "Rewrite validation layer with type-safe parser" --problem "API lacks input validation" --force
```

```jjj:setup
critique new "Rewrite validation" "Parser library adds 2MB to binary" --severity low
```

```jjj:setup
critique validate "adds 2MB"
```

```jjj
solution withdraw "Rewrite validation" --rationale "2MB binary increase violates our 1MB size budget for this service"
> withdrawn
```

Validated critique leads to explicit withdraw -- clear audit trail of why the approach failed.

## Step 9: Full cycle -- critique, address, submit, approve

```jjj:setup
solution new "Inline schema validation with zero dependencies" --problem "API lacks input validation" --force
```

```jjj:setup
critique new "Inline schema validation" "Test coverage for edge cases needed" --severity medium
```

```jjj:setup
critique address "edge cases"
```

```jjj:setup
solution submit "Inline schema"
```

```jjj
solution approve "Inline schema" --rationale "Zero-dependency validation eliminates size concern; test coverage added"
> approved
```

Full validate, withdraw, new solution, approve cycle completes cleanly.

## Step 10: Critique show --json

```jjj
critique show "DoS risk" --json
> "addressed"
> "severity"
> "reviewer"
```
