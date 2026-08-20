# Swarm trial

A rig for putting jjj under a real agent swarm, and for finding out whether
`skills/jjj/SKILL.md` actually makes agents behave.

## Shape

**One podman container per agent, each with its own working copy.** Nothing is
shared except a bare git remote, so every piece of coordination — what work
exists, who holds it, what was objected to, and the code itself — has to travel
through jjj. If jjj is not a sufficient substrate the swarm cannot function,
which is the claim under test.

Agents are grouped into **pods** that share a `JJJ_POD`, so several agents push
the same `jjj/{pod}` bookmark. That keeps ref contention (Break #5) in the
experiment; a bookmark per agent would have designed it away.

Each iteration is a **fresh `claude -p` session with no memory**. Agents are
stateless workers; all state is in jjj.

## The toy target

`toy/seed.py` generates `opkit`, a registry of small string operations. The
domain is deliberately dull — nobody should be debugging roman numerals. Its
shape exists to force collisions on four surfaces at once:

| Surface | Mechanism |
|---|---|
| Same-file edits | every op registers in one `opkit/registry.py` |
| Same-file edits | every op's cases live in one `tests/cases.py` |
| Claim contention | seed fewer problems than agents |
| Same-entity edits | agents critique and reply on each other's solutions |

**Fitness is `./score.py`: a count of passing conformance cases.** Counted, never
timed. A swarm saturates the machine it runs on, so any wall-clock metric would
measure its own contention rather than the code.

## Running it

```bash
export ANTHROPIC_API_KEY=sk-ant-...       # required; see Credentials below
./swarm.sh build                          # build the agent image
./swarm.sh init --pods 2 --agents 3 --problems 4
./swarm.sh start --max-iters 2            # bounded shakedown
./swarm.sh status
./swarm.sh logs pod-1-agent-01
./swarm.sh stop
./swarm.sh analyze
```

For a long run: `./swarm.sh start --hours 48`. Every agent checks a kill switch,
a deadline, and an iteration cap at the top of each turn, so a runaway is always
recoverable with `./swarm.sh stop`.

## Credentials

Set `ANTHROPIC_API_KEY`. The OAuth token in `~/.claude/.credentials.json` is
**not** usable here: it is short-lived and must be refreshed by writing back to
the file, so N containers sharing it would race to rotate one token and can
break the host login. (Mounting it read-only fails with
`OAuth access token has been revoked` once the access token expires.)

## What gets measured

`jjj-shim` shadows the real binary on the container's PATH, so every invocation
is recorded as JSONL — actor, pod, argv, exit, duration, output — whether or not
the agent cooperates. `analyze.py` turns that into answers to questions the
design's locked decisions assert but never verified:

- Did agents contend for claims, and is `--claim` really advisory? (decision 4)
- Did the critique gate hold, or did anything `--force` past an objection?
- Were conflicts produced, and did agents resolve them? (decision 10)
- Did per-pod bookmarks keep pushes from serialising? (decision 5)
- Did agents follow the skill — identity set, ids not titles?
- Did the fitness actually climb, or were agents merely busy?

## Known gaps this rig should expose

Four decisions in `docs/design/scaling-for-agent-swarms.md` are locked but
unimplemented, and a long run is expected to hit all four:

| Decision | Missing |
|---|---|
| 15 — stale-claim expiry | no `claimed_at`; a dead agent strands its work forever |
| 10 — auto-resolve + `ConflictAutoResolved` | only manual `jjj resolve` exists |
| 8 — `jjj sync --now`, pod debouncing | no `--now` flag |
| 12 — per-group ranking weight | not implemented |
