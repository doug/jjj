---
title: "End-to-End Showcase"
description: "Realistic onboarding: init, problem, solution, critique, approve, search, and timeline"
replaces: "uxr/scenarios/06-end-to-end-showcase.sh"
covers:
  - "Init on a repo with existing commits"
  - "Problem creation with priorities"
  - "Solution new resolves problem by partial title"
  - "Problem auto-transitions to in_progress"
  - "Critique blocks approval until addressed"
  - "Solution approve with --force"
  - "Problem auto-solves when only solution approved"
  - "Push/fetch/sync help accessible"
  - "Search and timeline"
  - "Help discoverability"
tags: [end-to-end, onboarding, lifecycle]
---

# End-to-End Showcase

## Setup: Realistic Project Repo with Existing Commits

Create a realistic source tree across multiple commits:

```shell:setup
mkdir -p src
cat > src/auth.rs << 'EOF'
pub fn authenticate(user: &str, password: &str) -> bool {
    !user.is_empty() && !password.is_empty()
}
EOF
cat > src/api.rs << 'EOF'
pub fn handle_request(path: &str) -> String {
    match path {
        "/" => "Welcome!".to_string(),
        "/api/status" => "OK".to_string(),
        _ => "Not found".to_string(),
    }
}
EOF
cat > src/db.rs << 'EOF'
pub struct Database { connection: String }
impl Database {
    pub fn new(url: &str) -> Self { Self { connection: url.to_string() } }
    pub fn query(&self, sql: &str) -> Vec<String> {
        println!("Executing: {}", sql); vec![]
    }
}
EOF
jj describe -m "feat: initial project structure (auth, api, db)"
jj new -m "feat: add password hashing"
echo 'pub fn hash_password(p: &str) -> String { format!("hash_{}", p) }' >> src/auth.rs
jj new -m "feat: add user lookup endpoint"
echo 'pub fn get_user(id: u64) -> Option<String> { if id == 1 { Some("alice".into()) } else { None } }' >> src/api.rs
```

The repo has three commits across auth, api, and db -- representing a typical project in flight.

## Step 1: Initialize jjj

jjj init works on a repo already in use with no conflicts:

```jjj
init
> initialized
```

Double-init is rejected cleanly:

```jjj:fail
init
> already
```

## Step 2: Identify a Problem

```jjj
problem new "Auth has no rate limiting" --priority high
> Auth has no rate limiting
```

```jjj
problem new "DB queries not sanitized" --priority critical
> DB queries not sanitized
```

```jjj
problem list
> Auth has no rate limiting
> DB queries not sanitized
```

Two problems created -- the list gives a clear overview of open work.

## Step 3: Propose a Solution

Solution new resolves the problem by partial title:

```jjj
solution new "Add token bucket rate limiter" --problem "rate limiting"
> Add token bucket rate limiter
```

The problem auto-transitions to in_progress when a solution is proposed:

```jjj
problem show "rate limiting"
> in_progress
```

```jjj
solution list
> token bucket
```

## Step 4: Check Status (Mid-Workflow)

```jjj
status
> token bucket
```

jjj status gives a clear "what am I working on right now" view.

## Step 5: A Teammate Adds a Critique

```jjj
critique new "token bucket" "Rate limit state is not shared across replicas" --severity high
> Rate limit state
```

```jjj
critique list
> not shared across replicas
```

Status now shows the solution is blocked:

```jjj
status
> BLOCKED
```

The BLOCKED state is immediately visible -- no way to accidentally approve a critiqued solution.

Submit for review, then attempt approval:

```jjj:setup
solution submit "token bucket"
```

```jjj:fail
solution approve "token bucket"
> critique
```

The approval gate is enforced -- the critique must be resolved first.

## Step 6: Address the Critique

```jjj
critique address "not shared"
```

Status is no longer blocked:

```jjj
status
>! BLOCKED
```

Once addressed, the path to approval is clear.

## Step 7: Approve the Solution

```jjj
solution approve "token bucket" --force
> approved
```

The problem auto-solves when its only solution is approved:

```jjj
problem show "rate limiting"
> solved
```

Problem lifecycle closes automatically -- no manual bookkeeping needed.

## Step 8: Metadata Transport (Push/Fetch)

Push and fetch move the jjj bookmark just like code -- no extra infrastructure needed.

```jjj
push --help
> remote
```

```jjj:setup
fetch --help
```

```jjj
sync --help
> fetch
```

## Step 9: Search and Timeline

```jjj
search "rate limit"
```

```jjj
timeline "rate limit"
> proposed
> Rate limit state
```

The timeline gives a complete audit trail of every decision made on a problem.

## Step 10: Help Discoverability

```jjj
--help
> problem
> solution
> critique
> sync
> github
```

All top-level commands are visible in --help for good discoverability.
