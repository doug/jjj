# Board and Dashboard

jjj provides two views for understanding your project's state: the **board** shows solutions organized by status, and **status** shows your personal context and prioritized next actions.

## The Board

The board displays all solutions grouped into four columns that reflect the solution lifecycle:

```
Proposed --> Testing --> Accepted
                    \-> Refuted
```

### Viewing the Board

```bash
jjj board
```

Output:

```
+--------------------------------+--------------------------------+--------------------------------+--------------------------------+
| PROPOSED (3)                   | TESTING (2)                    | ACCEPTED (1)                   | REFUTED (1)                    |
+--------------------------------+--------------------------------+--------------------------------+--------------------------------+
| s5 Add Redis caching          | s1 Use JWT tokens [1!]        | s3 Batch query API            | s2 Session cookies            |
| s6 Streaming search           | s4 Parameterized queries      |                                |                                |
| s7 Lazy loading for images    |                                |                                |                                |
+--------------------------------+--------------------------------+--------------------------------+--------------------------------+

Total: 7 solutions
Problems: 4 open, 2 solved/dissolved
```

### Understanding the Columns

**Proposed** -- Solutions that have been conjectured but not yet implemented or tested. These are ideas waiting to be worked on. A solution starts here when you run `jjj solution new`.

**Testing** -- Solutions that are actively being implemented and tested. Move a solution here when work begins:

```bash
jjj solution test s5
```

Or use `solution new`, which creates the solution, attaches a change, and moves it to testing in one step:

```bash
jjj solution new "Add Redis caching" --problem p10
```

**Accepted** -- Solutions that have survived criticism. All critiques have been resolved, assigned reviewers have signed off, and the solution has been accepted as the current best answer to its problem:

```bash
jjj solution accept s3
```

**Refuted** -- Solutions that criticism has shown will not work. A refuted solution is not a failure -- it is knowledge. The team now knows that approach does not solve the problem, and future solutions can build on that understanding:

```bash
jjj solution refute s2
```

### Reading the Board

The `[1!]` indicator next to a solution means it has open critiques. The number tells you how many. This is a signal that the solution needs attention -- critiques must be resolved before it can be accepted.

A healthy board typically has:
- A few solutions in Proposed (upcoming work)
- A small number in Testing (active work, limited by team capacity)
- Growing Accepted column (completed work)
- Some Refuted solutions (evidence of learning and exploration)

If Testing is overloaded, the team may be spreading too thin. If Proposed is empty, the team may need to identify new problems or propose alternative solutions.

### JSON Output

For scripting or integration with other tools:

```bash
jjj board --json
```

## Status

The `status` command shows your personal context: the active solution for your current change, prioritized next actions, and a summary of project health.

### Viewing Status

```bash
jjj status
```

Output:

```
Active: s5 "Add Redis caching" -> p10 [testing]
  Awaiting review: @bob
  Open critiques: 2
    c8: Cache invalidation not handled [high]
    c9: Redis single point of failure [medium]

Next actions:

1. [BLOCKED] s5: Add Redis caching -- 2 open critique(s)
   c8: Cache invalidation not handled [high]
   c9: Redis single point of failure [medium]
   -> jjj critique show c8

2. [TODO] p8: API rate limiting needed -- No solutions proposed
   -> jjj solution new "title" --problem p8

Summary: 4 open problems, 3 testing solutions, 5 open critiques
```

### Status Sections

**Active Solution** -- The solution linked to your current jj change. Shows its problem, status, pending reviewers, and open critiques.

**Next Actions** -- A prioritized list of items grouped by urgency: BLOCKED (solutions with open critiques), READY (solutions ready to accept), REVIEW (solutions waiting for your review), WAITING (your solutions awaiting others), and TODO (open problems with no solutions).

**Summary** -- Project-wide counts giving you a sense of overall workload and health.

### Acting on Status

1. **Open critiques?** Address them first. They are blocking your solutions.
   ```bash
   jjj critique show c8
   # Understand the concern, then address, dismiss, or validate
   jjj critique address c8
   ```

2. **Solutions in testing?** Continue implementation, request reviews when ready.
   ```bash
   jjj review @bob  # From the solution's change
   ```

3. **Problems assigned but no solutions?** Propose a solution.
   ```bash
   jjj solution new "Rate limit with token bucket algorithm" --problem p8
   ```

4. **Nothing assigned?** Run `jjj status --all` to see all items across the project.

## Combining Board and Status

The board gives you the project view. Status gives you the personal view. Use them together:

- **Planning**: Check the board to see what is proposed, what is being tested, and where the bottlenecks are.
- **Daily work**: Check status to see what needs your attention today.
- **Standup**: The board shows team progress; status shows individual context.

```bash
# Morning routine
jjj status             # What do I need to do?
jjj board              # How is the project doing?
```

## Next Steps

- [Problem Solving](problem-solving.md) -- Creating and managing problems
- [Critique Guidelines](critique-guidelines.md) -- Working with critiques
- [Code Review](code-review.md) -- The reviewer sign-off workflow
