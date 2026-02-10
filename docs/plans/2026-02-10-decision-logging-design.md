# Decision Logging and Evidence Design

## Overview

This design adds decision logging to jjj, enabling teams to understand how and why decisions were made over time. The primary use cases are:

1. **During current work** — "Why did we reject the caching approach last month? Is my new proposal hitting the same issue?"
2. **Onboarding / knowledge transfer** — "New team member wants to understand why we chose architecture X over Y"

The goal is to visualize an evolution timeline of problems being solved 2+ years into complex work.

## Key Decisions

### Evidence: Inline, Not a Separate Entity

Evidence (observations, experiments, benchmarks, opinions, constraints) is stored inline in existing markdown files rather than as a separate entity type.

**Rationale:** In a Popperian framework, there's no theory-free observation. "Evidence" is always an interpretation — a claim about what was observed and what it means. The evidence *is* the prose in Problem/Solution/Critique descriptions.

- Problems capture observations in their markdown body
- Solutions reference evidence in their approach
- Critiques cite evidence in their arguments

For complex evidence (experiments, studies), write up the findings in the entity file and link to external data files or URLs.

**Trade-off:** Same evidence used in multiple places must be described multiple times, or referenced by linking to another entity ("see benchmark in p1"). This is acceptable given the reduced complexity.

### Decision History: Append-Only Event Log

Rather than storing decision history on each entity, we use a single append-only event log file.

**File:** `.jjj/events.jsonl`

Each line is a self-contained JSON event:

```json
{"when":"2024-01-15T10:30:00Z","type":"solution_accepted","entity":"s1","by":"alice","rationale":"Critique c3 addressed, benchmarks pass","refs":["c3","p1"]}
{"when":"2024-01-16T14:00:00Z","type":"critique_raised","entity":"c4","by":"bob","target":"s2","severity":"high","title":"Race condition in write path"}
{"when":"2024-01-17T09:15:00Z","type":"solution_refuted","entity":"s2","by":"bob","rationale":"Critique c4 validated - fundamental flaw","refs":["c4"]}
```

**Rationale:** A single file is fast to query and simple to manage. Entity files store current state; the event log stores history. No sync issues because both are written atomically by jjj commands.

### Commit Messages: Structured Backup

Commit messages to the shadow graph include a JSON one-liner for audit trail and rebuild capability:

```
Accept solution s1

Critique c3 addressed, benchmarks pass.

jjj: {"type":"solution_accepted","entity":"s1","by":"alice","refs":["c3","p1"]}
```

**Rationale:** The event log file is the primary query source. Commit messages are an immutable backup. If the event log is ever corrupted or suspect, it can be rebuilt by parsing commit history.

## Event Schema

### Common Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `when` | ISO 8601 timestamp | yes | When the event occurred |
| `type` | string | yes | Event type (see below) |
| `entity` | string | yes | Primary entity ID (p1, s1, c1, m1) |
| `by` | string | yes | Who triggered the event |
| `rationale` | string | no | Human explanation of why |
| `refs` | string[] | no | Related entity IDs |

### Event Types

**Problem events:**
- `problem_created`
- `problem_solved`
- `problem_dissolved`
- `problem_reopened`

**Solution events:**
- `solution_created`
- `solution_accepted`
- `solution_refuted`

**Critique events:**
- `critique_raised`
- `critique_addressed`
- `critique_dismissed`
- `critique_validated`

**Milestone events:**
- `milestone_created`
- `milestone_completed`

### Type-Specific Fields

**critique_raised:**
- `target` — solution ID being critiqued
- `severity` — low, medium, high, critical
- `title` — critique title

**solution_created:**
- `problem` — problem ID this solution addresses
- `supersedes` — solution ID this supersedes (if any)

## Commands

### Event Queries

```bash
# Recent events (default: last 20)
jjj events

# Filter by time range
jjj events --from 2024-01 --to 2024-06

# Filter by entity
jjj events --problem p1
jjj events --solution s1

# Filter by type
jjj events --type solution_accepted
jjj events --type critique_raised

# Full-text search on rationales
jjj events --search "cache"

# JSON output for tooling
jjj events --json
```

### Timeline Visualization

```bash
# Full timeline for a problem and all related entities
jjj timeline p1
```

**Example output:**

```
p1: Authentication is unreliable
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

2024-01-10  problem created                     alice
2024-01-12  s1 proposed: "Add retry logic"      alice
2024-01-14  c1 raised: "Doesn't handle timeouts" bob
2024-01-15  s2 proposed: "Token refresh + retry" alice
            supersedes s1
2024-01-16  c1 dismissed                         alice
            "Superseded by s2 which handles this"
2024-01-18  s1 refuted                           alice
            "s2 is more comprehensive"
2024-01-20  s2 accepted                          bob
            "All critiques addressed, tests pass"
2024-01-20  problem solved                       bob
```

### Maintenance

```bash
# Rebuild events.jsonl from commit history
jjj events rebuild

# Validate event log matches entity states
jjj events validate
```

## Integration with Existing Commands

Existing commands automatically log events. No extra work for users:

```bash
jjj solution accept s1       # logs solution_accepted
jjj solution refute s1       # logs solution_refuted
jjj critique new s1 "..."    # logs critique_raised
jjj critique address c1      # logs critique_addressed
jjj problem solve p1         # logs problem_solved
```

### Rationale Capture

When accepting or refuting, users can provide rationale inline:

```bash
jjj solution accept s1 --rationale "Benchmarks pass, critique c3 addressed"
```

If omitted, prompt interactively:

```bash
jjj solution accept s1
> Rationale (optional): _
```

Skip prompt for quick workflows:

```bash
jjj solution accept s1 --no-rationale
```

## Example Workflow

```bash
# Problem captures the observation
jjj problem new "Search is slow under load"
# Edit p1.md body: "Benchmark X showed 450ms p99 at 1000 concurrent users"

# Solution references the evidence
jjj solution new "Add Redis caching" --problem p1
# Edit s1.md approach: "Based on the benchmark in p1, caching reduces DB load..."

# Critique challenges with evidence
jjj critique new s1 "Cache invalidation not handled"
# Edit c1.md: "When data updates, stale results returned for TTL window.
#              See incident report from 2024-01-05..."

# Address critique
jjj critique address c1

# Decision captures the resolution
jjj solution accept s1 --rationale "c1 addressed with write-through invalidation"
# Event logged: {"when":"...","type":"solution_accepted","entity":"s1","by":"alice","rationale":"c1 addressed with write-through invalidation","refs":["c1","p1"]}

# Query later
jjj timeline p1   # shows full journey with rationales
```

## Future Considerations

These are not part of the initial implementation but may be added later:

### Cross-Problem Discovery

Tags on events for categorization:
```bash
jjj events --tag caching
jjj events --tag authentication
```

### Reopening Problems

When a solved problem resurfaces:
```bash
jjj problem reopen p1 --rationale "Issue resurfaced in production"
```

Timeline shows the loop: solved → reopened → solved again.

### Supersession Lineage

Visual representation of solution evolution:
```
s1 proposed → s1 refuted
    └── s2 proposed (supersedes s1) → s2 accepted
```

### Retention/Archival

For very long-running projects, option to archive old events:
```bash
jjj events archive --before 2023-01-01 --output events-2022.jsonl
```

## Implementation Notes

### Storage

- `events.jsonl` lives in `.jjj/` alongside other metadata
- File is append-only; events are never edited or deleted
- Committed to shadow graph with each status-changing operation

### Atomicity

Status-changing commands must:
1. Update entity file
2. Append to events.jsonl
3. Commit both with structured message

All three happen in a single `with_metadata()` transaction.

### Parsing Commit Messages

For `jjj events rebuild`, parse commits looking for lines starting with `jjj: ` and extract the JSON payload.

```rust
if line.starts_with("jjj: ") {
    let json = &line[5..];
    let event: Event = serde_json::from_str(json)?;
    events.push(event);
}
```

## Summary

This design enables decision logging without adding entity complexity:

- **Evidence** stays inline in existing markdown files
- **Decision history** lives in a single append-only event log
- **Commit messages** provide an immutable backup
- **Timeline visualization** shows the evolution of problem-solving over years
- **Automatic logging** — existing commands log events with no extra work
