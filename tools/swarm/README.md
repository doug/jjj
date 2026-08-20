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

29 operations, 115 conformance cases. Expected values were computed by a
reference implementation, not written by hand — a wrong expectation is
indistinguishable from a broken agent — and the full set is verified achievable
at 115/115 before any trial runs.

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

## Findings so far

The rig earned its keep on its first run.

**Per-pod push was broken in the shipped release.** Every `jjj push` from a pod
failed with `cannot lock ref 'refs/heads/jjj/pod-2': 'refs/heads/jjj' exists`.
A git ref is a path, so the bare `jjj` bookmark being a *file* means no ref can
nest beneath it — and every real repository has that bookmark. Break #5's
remedy for ref contention had therefore never worked. Fixed to sibling refs
(`jjj-{pod}`); see `pod_and_bare_bookmarks_coexist_on_a_remote`.

**The published Linux binaries did not run on most distros.** Building the agent
image against Debian bookworm surfaced `GLIBC_2.39 not found`: the release built
`*-linux-gnu` on ubuntu-latest. Linux targets are now static musl.

**A naive auto-resolve loses work silently.** The harness resolved code merge
conflicts with `git checkout --ours`, which discarded another agent's registry
entry — a fully correct `roman` implementation scored zero because its
registration vanished. That is decision 10's hazard in miniature. The append-only
shared files now use a union merge driver, and the harness no longer picks a side.

**Scale is bounded by container memory, not tokens.** Agents need ~3 GiB
(node plus the CLI); at 1 GiB they are OOM-killed mid-turn, which looks
identical to a crash. Size the podman machine accordingly.

### Findings about swarm design itself

Three came from the rig rather than from jjj, and each cost a run to learn.

**A shared priority list does not distribute.** With every agent told "review if
anything is reviewable, otherwise take new work", six agents produced **193
reviewing calls against 13 producing ones** and implemented two operations in 36
turns. Reviewing is always available and cheaper than building, so the fleet
starved itself. Fixed by specialising pods into builders and critics — which is
what decision 4's "soft domain specialization per pod" is for.

**Do not resolve semantic merge conflicts in bash.** Three shell policies were
tried and all three lost work: `checkout --ours` discarded another agent's
registry entry (a correct implementation scored zero), `merge --abort` dropped
the incoming side, and `checkout -- .` threw away the agent's own. Every agent is
a Claude session that resolves a Python conflict in seconds, so the conflict is
now left in the tree and handed to the agent as its task. **Fitness went 4/31 to
12/31 on that change alone.**

**Never publish an unresolved conflict.** One committed `<<<<<<< HEAD` broke the
package import and took the whole fleet's score to zero. jjj already refuses this
for metadata (it validates entity bodies before push); the code path had no such
guard and now does.

## Known gaps this rig should expose

Four decisions in `docs/design/scaling-for-agent-swarms.md` are locked but
unimplemented, and a long run is expected to hit all four:

| Decision | Missing |
|---|---|
| 15 — stale-claim expiry | no `claimed_at`; a dead agent strands its work forever |
| 10 — auto-resolve + `ConflictAutoResolved` | only manual `jjj resolve` exists |
| 8 — `jjj sync --now`, pod debouncing | no `--now` flag |
| 12 — per-group ranking weight | not implemented |
