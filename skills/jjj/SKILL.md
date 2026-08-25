---
name: jjj
description: Use when working in a repository that has jjj initialized (a .jj/jjj-meta directory), or when coordinating several agents on shared work — tracking problems, proposing competing solutions, critiquing them, and reaching decisions that survive history rewrites. Also use when asked to set up multi-agent collaboration, work queues, or review workflows on top of Jujutsu.
---

# jjj

jjj is a project tracker that lives inside a Jujutsu repository. There is no
server and no database: problems, solutions, and critiques are markdown files in
an orphaned `jjj` bookmark, synced with `jjj push` / `jjj fetch`.

For an agent, three properties matter:

1. **Every command speaks JSON.** `--json` on essentially everything, so state is
   readable without parsing prose.
2. **Identity is per-process.** `JJJ_USER` and `JJJ_POD` make each agent a
   distinct actor in the same checkout, with its own work queue and its own push
   bookmark.
3. **The data model enforces review.** An open critique *blocks* approval. Not by
   convention — the command refuses.

That third property is what makes jjj a coordination substrate rather than a
notepad: it lets a fleet of agents run a real selection process on ideas, where
surviving criticism is the only way through.

## The epistemic model

jjj implements conjecture and refutation as a workflow:

```
Problem          a question worth answering (not a task)
  └── Solution   a conjecture, attached to a jj change
        └── Critique   an attempted refutation — blocks approval until resolved
```

A **problem** is solved when a solution survives criticism, or *dissolved* when
it turns out to rest on a false premise. A **solution** is a bold guess you are
willing to have shot down. A **critique** is the mechanism of selection.

This maps onto multi-agent work with no translation: many agents propose rival
conjectures to one problem, other agents attack them, and what survives is what
ships. Nobody has to arbitrate, because refutation is recorded, attributed, and
enforced.

## First: establish identity

**Do this before any write.** Without it every agent shares the repository
owner's identity, so claims overwrite each other and `--mine` returns everyone's
work.

```bash
export JJJ_USER=reviewer-3        # who is acting
export JJJ_POD=pod-7              # optional: also gives an own push bookmark

jjj whoami --json
# {"actor": "reviewer-3", "pod": "pod-7", "push_bookmark": "jjj/pod-7"}
```

Resolution order is `JJJ_USER` → pod id → jj `user.name`. Set `JJJ_USER` per
agent; set `JJJ_POD` as well when the agent pushes, so it writes its own
`jjj/{pod}` ref and never contends with another pusher.

Verify it took effect. An agent that silently inherits the wrong identity
produces work nobody can attribute:

```bash
test "$(jjj whoami --json | jq -r .actor)" = "$JJJ_USER" || exit 1
```

## The machine interface

**Use IDs, never titles, in anything automated.** Every command accepts a fuzzy
title match, which is convenient at a prompt and dangerous in a loop: `"auth"`
resolves to whatever uniquely matches *today*, and tomorrow that is a different
entity. Read the id from `--json` and pass that.

```bash
id=$(jjj problem list --status open --json | jq -r '.[0].id')
jjj problem show "$id" --json
```

Other things worth knowing before scripting:

| Behaviour | Consequence |
|---|---|
| `problem new`, `solution new`, `critique new` accept `--json` | Capture the new id directly instead of parsing "Created ..." |
| Every creator accepts `--body TEXT` or `--body -` (stdin) | Write the argument in the body, not the title — see below |
| Non-zero exit on failure, message on stderr | Check the status; do not parse stdout for errors |
| `jjj next --json` emits `null`, one object, or an array | Normalize before iterating |
| `--force` skips duplicate detection (`problem new`) or the critique gate (`solution approve`) | Never default to it in a loop; it disables the thing that makes review real |
| An empty reference is rejected, not fuzzy-matched | `jjj solution approve "$SID"` with `SID` unset fails loudly instead of approving something arbitrary |
| Only `solution approve`, `solution withdraw` and `problem dissolve` prompt for a rationale | Pass `--rationale "..."` or `--no-rationale` to *those*; other commands reject the flag |
| `--mine` works on `next`, `status`, `events`, and `problem`/`solution`/`critique list` | It is shorthand for "assigned to me"; it does not exist elsewhere |
| Reads are served from a SQLite cache | `jjj db rebuild` after editing metadata files behind jjj's back |

## Write the argument in the body, not the title

Every entity has one free-form body: a problem's `description`, a solution's
`approach`, a critique's `argument`. That body is the payload — in a system
built on conjecture and refutation, the reasoning *is* the content, and a title
is only a label for it.

```sh
jjj solution new "Cache the parsed frontmatter" --problem "$pid" \
  --body "Parsing dominates list at 25K. Cache keyed by (path, mtime)..."

# `-` reads stdin, so a long argument survives without shell quoting mangling it
jjj critique new "$sid" "Validates the cache, not the content" --severity critical --body - <<'EOF'
The dirty flag means "a sync was interrupted", not "the markdown is unchanged".
Markdown written by any other tool leaves the cache clean but stale, so this
validates a stale cache: write a critique whose solution_id does not exist,
then push. On main it is refused; with this change push reports "All checks
passed" and publishes the dangling reference to every clone.
EOF
```

Agents that do not know about `--body` put paragraphs in the title instead —
observed in practice, producing critique titles hundreds of characters long
while the structured field stayed empty. Titles are for scanning a list; bodies
are for the argument that another agent has to be able to evaluate.

## The single-agent loop

```bash
jjj status                                   # what needs attention
jjj next --json                              # the single highest-value action

# Take a problem
pid=$(jjj next --claim --json | jq -r .entity_id)

# Conjecture: attach it to real work
jj new -m "rate limiter: token bucket"
sid=$(jjj solution new "Token bucket with burst credit" --problem "$pid" --json | jq -r .id)
# ... make the change ...
jjj solution attach "$sid"                   # links the CURRENT jj change
jjj solution submit "$sid"                   # opens it for critique

# After review
jjj solution approve "$sid" --rationale "burst accounting verified under load"
```

`jjj solution attach` links whatever jj change is checked out *now*. Make the
change first, then attach.

## Framing the work is the work

The hard part of a swarm is not doing the work in parallel — one agent with more
turns does that. It is **deciding what the work is**: splitting a problem into
pieces others can take independently, saying which pieces matter most, and
noticing when a problem is misconceived before six people solve it.

jjj has the machinery for all three, and it is the machinery agents most
consistently skip:

```sh
# Decompose. A sub-problem is a problem with a parent.
jjj problem new "Reduce allocations on the layout path" --parent "$pid" \
  --body "Profile says layout is 40% of frame allocations; here is the breakdown."

# Prioritise. Without this, agents converge on whatever `next` returns first.
jjj rank set "$a" "$b" "$c" --gap "$b:XL"   # everything under b is a different league
jjj rank move "$c" top
jjj rank show                                # the fleet's aggregate order

# Retract. A problem that turned out to be a bad question, or a duplicate.
jjj problem dissolve "$pid" --rationale "the measurement that motivated this was wrong"
jjj problem duplicate "$pid" --of "$other"
```

**Why it matters, measured.** In a four-hour trial with six agents and no
ranking, five problems drew solutions from more than one agent — one drew seven
solutions from six agents — while other problems went untouched, and 62% of all
solutions were withdrawn as superseded. That waste was concentration, not
competition: nobody had said which problems mattered, so everyone picked the
same one. A ranking is how six agents end up on six problems.

Rival solutions to *one* problem are still the point — competing conjectures are
how you find out which survives. The distinction is whether the rivalry is
chosen or accidental.

## Multi-agent patterns

### Pattern A — the claim loop (work distribution without a scheduler)

Each agent pulls its own work. No dispatcher, no queue service.

```bash
export JJJ_USER=worker-$i
while true; do
  item=$(jjj next --claim --json)
  [ "$item" = "null" ] && break

  id=$(echo "$item" | jq -r .entity_id)

  # Claim is ADVISORY, not a mutex — verify you actually hold it.
  holder=$(jjj problem show "$id" --json | jq -r .assignee)
  [ "$holder" = "$JJJ_USER" ] || continue      # someone else won; move on

  # ... do the work ...
done
```

**A claim is a lease, not a lock.** Two things follow.

*It does not exclude.* `--claim` reads the top item then assigns it, so two
agents reading before either writes both claim it and the last write wins. The
claim-then-verify above is what makes that safe.

*It expires.* The claim records when it was taken, and once the lease lapses the
item returns to the pool — otherwise an agent that dies mid-task would hold its
work forever, which over a long run is the expected failure, not a rare one. Your
own claim refreshes whenever you re-claim, so keep working and it stays yours.
An explicit `jjj problem assign` has **no** lease and never expires: handing work
to someone is a decision, not a claim. The default lease is an hour
(`claim_ttl_minutes` under `[settings]`).

Work another agent is actively holding is not offered to you at all, so the queue
you see is already filtered to what you may take. Two further ways to avoid
contending:

- **Partition up front.** Give each agent a disjoint slice — `jjj problem list
  --tag backend`, or one milestone per agent — so they never contend.
- **Offset the queue.** Agent *i* takes item *i* from `jjj next --top N --json`
  rather than all taking the head.

Once claimed, `jjj next --mine` is that agent's private queue.

### Pattern B — rival conjectures (the tournament)

The pattern jjj is actually built for. One problem, several agents, competing
approaches, selection by criticism.

```bash
# Fan out: each agent proposes a DIFFERENT approach to the same problem
JJJ_USER=proposer-a jjj solution new "Token bucket"       --problem "$pid" --force
JJJ_USER=proposer-b jjj solution new "Leaky bucket"       --problem "$pid" --force
JJJ_USER=proposer-c jjj solution new "Sliding window log" --problem "$pid" --force

for s in $(jjj solution list --json | jq -r '.[].id'); do
  jjj solution submit "$s"
done

# Critics attack every candidate
JJJ_USER=critic-1 jjj critique new "$sid_c" "O(n) memory per client" --severity critical

# A refuted conjecture is withdrawn, not deleted — the record of why survives
JJJ_USER=proposer-c jjj solution withdraw "$sid_c" --rationale "memory cost confirmed"

# The survivor is approved; the problem resolves
JJJ_USER=proposer-a jjj critique address "$cid"
JJJ_USER=proposer-a jjj solution approve "$sid_a" --rationale "burst accounting verified"
```

Afterwards `jjj timeline "$pid"` is the full record: who proposed what, who
objected, what was withdrawn and why. That trail is the point. A fleet that
discards its losing branches cannot learn from them; here the refutations are
first-class and permanently attached to the problem.

Keep the conjectures genuinely different. Three agents proposing three variants
of the same idea is not a tournament — it is one idea with extra steps.

jjj enforces this: `solution new` refuses a title too close to an existing one.
If you hit that, someone already has the work — take something else. Reach for
`--force` only when you really are proposing a different approach, and title it
so the difference is visible ("Regex-based wordcount with unicode handling", not
"wordcount again").

### Pattern C — adversarial review (enforced, not requested)

Separate the agent that proposes from the agent that criticizes, and let the
data model hold the line:

```bash
JJJ_USER=proposer jjj solution submit "$sid"
JJJ_USER=critic   jjj critique new "$sid" "No backpressure on the retry path" --severity high

JJJ_USER=proposer jjj solution approve "$sid" --no-rationale
# Error: Cannot approve solution: 1 open critique(s) must be addressed first
```

The proposer **cannot** approve past an open critique. This is the load-bearing
guarantee for autonomous fleets: an agent grading its own homework is the usual
failure mode, and here it is structurally unavailable.

Critique lifecycle:

| Command | Meaning |
|---|---|
| `jjj critique address <id>` | The solution was changed to handle it |
| `jjj critique validate <id>` | The objection is correct — the solution should be withdrawn |
| `jjj critique dismiss <id>` | The objection is wrong or no longer relevant |
| `jjj critique reply <id> "..."` | Argue about it first |
| `jjj solution lgtm <id> --rationale "..."` | You reviewed it and it holds up |

You do **not** need to be assigned a review to sign off — take submitted work
off the queue and `lgtm` it, and the review is recorded for you. You cannot sign
off your own solution: that is the one thing the gate exists to prevent.

Use `--severity critical|high|medium|low` so a triage agent can sort. Anchor a
critique to code with `--file` and `--line`. Route it to a specific reviewer with
`--reviewer <who>`; it then shows up in that agent's `jjj next --mine`.

Escalation path when a critique is contested: `reply` → if unresolved,
`validate` (forcing withdrawal) or `dismiss` with a rationale. Every step is an
event, so a supervisor can find deadlocks with
`jjj events --event-type critique_replied`.

### Pattern D — collision avoidance

Parallel agents editing the same files will conflict. Find out before it happens:

```bash
jjj overlaps --json     # files touched by more than one in-flight solution
```

Run it before starting work and after attaching a change. An overlap is a signal
to sequence the two solutions, split the problem, or have the agents negotiate
via a critique.

### Pattern E — distributed prioritization

Each agent authors its own ordering of a milestone's problems; the aggregate
decides what the fleet does next. Weighting is budget-normalized, so an agent
that ranks fifty items has exactly as much influence as one that ranks five — no
agent can dominate by being verbose.

```bash
jjj rank show "$milestone" --json        # aggregate
jjj rank show "$milestone" --by-user     # per-agent breakdown
```

Orderings live in `rankings/{milestone}/{actor}.json` — `order` (an array of
problem ids) plus `gaps` (`S`/`M`/`L`/`XL` below an item, expressing *how much*
lower). Agents can write these files directly; humans use `jjj ui`.

### Pattern F — pods and parallel push

Each pod pushes its own single-writer bookmark, so N agents push concurrently
without racing on a ref:

```bash
JJJ_POD=pod-3 jjj push        # writes jjj/pod-3
jjj fetch                     # merges every pod's bookmark
```

Fetch performs a three-way merge per entity. Divergent edits to the *same* body
produce conflict markers rather than a silent winner:

```bash
jjj conflicts --json
jjj resolve "$id" --ours --rationale "kept the token-bucket description"
```

## What is enforced vs. what is convention

Agents should rely on the first column and not assume the second.

| Enforced by jjj | Left to you |
|---|---|
| Open critiques block `solution approve` | Whether a critic is a *different* agent from the proposer |
| `problem solve` needs an approved solution or all sub-problems solved | Whether the solution is any good |
| Each pod is the sole writer of its bookmark | Whether two agents pick disjoint work |
| Conflicting body edits surface as conflicts | Which side is right |
| Every state change emits an attributed event | Whether anyone reads them |
| Automation rules are machine-local and never sync | — |

That last row is a security boundary: `config.toml` travels through the shared
bookmark, so automation rules deliberately do **not** live there. An agent cannot
push a rule that executes on another agent's machine. Rules go in
`.jj/jjj-meta/automation.toml`; `jjj automation list` shows what is active.

## Failure modes

| Symptom | Cause | Do this |
|---|---|---|
| Two agents did the same work | `--claim` is advisory | Claim-then-verify, or partition the work |
| `--mine` returns nothing | Identity not set, or nothing assigned | `jjj whoami`; assign before filtering |
| Entities look stale | The SQLite cache is derived | `jjj db rebuild` |
| `<<<<<<<` in an entity body | Divergent edits merged | `jjj conflicts`, then `jjj resolve --ours\|--theirs` |
| Fuzzy title hit the wrong entity | Titles are not identifiers | Use ids from `--json` |
| A push refuses | Another push holds the lock | `jjj doctor` names the holder |
| Anything unexplained | — | `jjj doctor` — versions, cache, locks, conflicts, automation, in one pass |

`jjj doctor --json` is the right first call in any failure branch, and the right
thing to attach to a bug report.

## Setting this up in an agent harness

jjj's integration surface is a CLI with JSON output and two environment
variables. Any harness that can run a shell command can drive it; what differs is
only where each one reads its instructions.

**Claude Code.** Copy this directory to `.claude/skills/jjj/` in the target
repository (or `~/.claude/skills/jjj/` for every repository). Claude loads it on
demand from the `description` above. For a fleet, give each subagent its own
`JJJ_USER` and have the supervisor read `jjj next --json` to decide dispatch. A
`SessionStart` hook that exports `JJJ_USER` is a reliable way to make identity
automatic rather than remembered.

**Codex and OpenCode.** Both read an `AGENTS.md` at the repository root. Add a
short section pointing at this file — the patterns are the same, only the
loading mechanism differs:

```markdown
## Project tracking
This repo uses jjj. Read `skills/jjj/SKILL.md` before creating or reviewing work.
Always `export JJJ_USER=<your agent name>` first.
```

**Antigravity** and other IDE-based agents: the same `AGENTS.md` pointer works
wherever the harness reads repository-level instructions; otherwise paste the
identity rule and the command reference into whatever custom-instruction field
it offers.

**Anything else.** The contract is: set `JJJ_USER`, use `--json`, pass ids not
titles, and never `--force` past a critique. Nothing above depends on a
harness-specific feature.

## Command reference

Verified against jjj 0.5.1. `jjj <command> --help` is authoritative.

```bash
# Orientation
jjj status [--mine] [--json]        jjj next [--top N] [--mine] [--claim] [--json]
jjj whoami [--json]                 jjj doctor [--json]
jjj insights [--json]               jjj overlaps [--json]         jjj tags [--json]

# Problems
jjj problem new "Title" [--body TEXT | --body -]
                        [--priority critical|high|medium|low] [--tags a,b]
                        [--parent ID] [--milestone REF] [--force]
jjj problem list [--status S] [--assignee WHO] [--tag T] [--milestone M] [--json]
jjj problem show ID [--json]        jjj problem assign ID [--to WHO]
jjj problem solve ID                jjj problem dissolve ID --reason "..."
jjj problem reopen ID               jjj problem duplicate ID --of OTHER
jjj problem tree [--json]           jjj problem graph [--json]

# Solutions
jjj solution new "Title" --problem ID [--body TEXT | --body -] [--force]
jjj solution attach ID              # links the CURRENT jj change
jjj solution submit ID              # open for critique
jjj solution approve ID [--rationale "..." | --no-rationale] [--force]
jjj solution withdraw ID [--rationale "..."]
jjj solution list [--json]          jjj solution show ID [--json]
jjj solution diff ID [--json]       jjj solution lgtm ID [--approve] [--json]

# Critiques
jjj critique new SOLUTION_ID "Title" [--body TEXT | --body -]
                                     [--severity critical|high|medium|low]
                                     [--file F --line N] [--reviewer WHO]
jjj critique address ID [--json]    jjj critique validate ID [--json]
jjj critique dismiss ID [--json]    jjj critique reply ID "..."
jjj critique list [--json]          jjj critique show ID [--json]

# Milestones and ranking
jjj milestone new "Title" [--body TEXT] [--date YYYY-MM-DD]

# Ranking — which problems matter most, and by how much
jjj rank set PROBLEM... [--gap PROBLEM:S|M|L|XL] [--milestone M] [--json]
jjj rank move PROBLEM top|bottom|up|down|before:OTHER [--milestone M]
jjj rank show [MILESTONE] [--by-user] [--json]
jjj milestone add-problem M P       jjj milestone status M [--json]
jjj rank show [MILESTONE] [--by-user] [--json]

# History
jjj events [--event-type T] [--problem P] [--solution S] [--since TS] [--json]
jjj timeline REF                    jjj search "query"

# Sync and coordination
jjj fetch                           jjj push               jjj sync
jjj conflicts [--json]              jjj resolve ID --ours|--theirs [--rationale "..."]
jjj automation list [--json]        jjj db rebuild
```

## Verified behaviour

The claims in this file are exercised by the repository's own tests, so they
cannot drift silently:

- Per-agent identity, `--claim`, and `--mine` scoping — `tests/identity_test.rs`
- Coordination, conflicts, and `doctor` — `journeys/20-coordination.md`
- A full review across two machines — `journeys/21-two-clone-sync.md`
- Ranking aggregation across agents — `journeys/19-ranking.md`
- Automation staying machine-local — `tests/automation_security_test.rs`
