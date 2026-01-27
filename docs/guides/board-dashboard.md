# Board and Dashboard

jjj provides two views for understanding your project's state: the **board** shows solutions organized by status, and the **dashboard** shows your personal work and action items.

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
| S-5 Add Redis caching          | S-1 Use JWT tokens [1!]        | S-3 Batch query API            | S-2 Session cookies            |
| S-6 Streaming search           | S-4 Parameterized queries      |                                |                                |
| S-7 Lazy loading for images    |                                |                                |                                |
+--------------------------------+--------------------------------+--------------------------------+--------------------------------+

Total: 7 solutions
Problems: 4 open, 2 solved/dissolved
```

### Understanding the Columns

**Proposed** -- Solutions that have been conjectured but not yet implemented or tested. These are ideas waiting to be worked on. A solution starts here when you run `jjj solution new`.

**Testing** -- Solutions that are actively being implemented and tested. Move a solution here when work begins:

```bash
jjj solution test S-5
```

Or use the workflow shorthand, which creates the solution and moves it to testing in one step:

```bash
jjj start "Add Redis caching" --problem P-10
```

**Accepted** -- Solutions that have survived criticism. All critiques have been resolved, reviews are in, and the solution has been accepted as the current best answer to its problem:

```bash
jjj solution accept S-3
```

**Refuted** -- Solutions that criticism has shown will not work. A refuted solution is not a failure -- it is knowledge. The team now knows that approach does not solve the problem, and future solutions can build on that understanding:

```bash
jjj solution refute S-2
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

## The Dashboard

The dashboard shows your personal work context: what you are responsible for and what needs your attention.

### Viewing the Dashboard

```bash
jjj dashboard
```

Output:

```
Dashboard for @alice

My Problems (2):
  P-3 - Search results include deleted items [in_progress]
  P-8 - API rate limiting needed [open]

My Solutions (1):
  S-5 - Add Redis caching [testing] (P-10)

Open Critiques on My Solutions (2):
  CQ-8 - Cache invalidation not handled [high, S-5]
  CQ-9 - Redis single point of failure [medium, S-5]

Summary:
  Problems: 4 open, 2 in progress
  Solutions: 3 testing
  Critiques: 5 open
```

### Dashboard Sections

**My Problems** -- Problems assigned to you that are still open or in progress. These are the problems you are responsible for finding solutions to.

**My Solutions** -- Solutions assigned to you that are in an active state (proposed or testing). These are the solutions you are implementing.

**Open Critiques on My Solutions** -- Critiques that need your response. Each one is blocking your solution from acceptance. Prioritize critical and high severity critiques.

**Summary** -- Project-wide counts giving you a sense of overall workload and health.

### Acting on the Dashboard

The dashboard tells you what to do next:

1. **Open critiques?** Address them first. They are blocking your solutions.
   ```bash
   jjj critique show CQ-8
   # Understand the concern, then address, dismiss, or validate
   jjj critique address CQ-8
   ```

2. **Solutions in testing?** Continue implementation, request reviews when ready.
   ```bash
   jjj review @bob  # From the solution's change
   ```

3. **Problems assigned but no solutions?** Propose a solution.
   ```bash
   jjj solution new "Rate limit with token bucket algorithm" --problem P-8
   ```

4. **Nothing assigned?** Check `jjj next` for suggested work items.
   ```bash
   jjj next
   ```

## Combining Board and Dashboard

The board gives you the project view. The dashboard gives you the personal view. Use them together:

- **Planning**: Check the board to see what is proposed, what is being tested, and where the bottlenecks are.
- **Daily work**: Check the dashboard to see what needs your attention today.
- **Standup**: The board shows team progress; the dashboard shows individual status.

```bash
# Morning routine
jjj dashboard          # What do I need to do?
jjj board              # How is the project doing?
jjj next               # What should I pick up next?
```

## Next Steps

- [Problem Solving](problem-solving.md) -- Creating and managing problems
- [Critique Guidelines](critique-guidelines.md) -- Working with critiques
- [Code Review](code-review.md) -- The review and LGTM workflow
