---
title: Using jjj with AI Agents
description: Teach Claude Code or Gemini CLI to use jjj so your AI assistant can create problems, propose solutions, add critiques, and manage your workflow.
---

# Using jjj with AI Agents

jjj ships a skill file that teaches AI coding assistants — Claude Code, Gemini CLI, or any agent that supports skills — to use `jjj` commands natively. Once installed, your agent can create problems, propose solutions, add critiques, and manage milestones without you having to explain the commands each time.

## Installing the Skill

### Claude Code

Skills live in `~/.claude/skills/`. Download the jjj skill with:

```bash
mkdir -p ~/.claude/skills/jjj && \
  curl -fsSL https://jjj.recursivewhy.com/skill.md \
    -o ~/.claude/skills/jjj/SKILL.md
```

Then invoke it with `/jjj` in any Claude Code session, or Claude will pick it up automatically when working in a jj repository.

### Gemini CLI

Add the skill as a custom slash command in your `GEMINI.md`:

```bash
mkdir -p ~/.gemini/skills/jjj && \
  curl -fsSL https://jjj.recursivewhy.com/skill.md \
    -o ~/.gemini/skills/jjj/SKILL.md
```

Or paste the skill content directly into your project's `GEMINI.md` under a `## jjj` section.

### Raw skill file

The skill file is plain Markdown and works with any agent that accepts context files:

```
https://jjj.recursivewhy.com/skill.md
```

## What the Skill Teaches

The skill gives the agent:

- The full command reference with correct syntax (current vocabulary: `submit` / `approve` / `withdraw`, not older aliases)
- Entity resolution rules — fuzzy title match, UUID prefix, or full UUID
- Blocking rules — what prevents `solution approve` from succeeding
- The core workflow from problem creation through critique resolution to approval
- JSON output flags for every command
- GitHub integration commands

## Example Prompts

Once the skill is active, you can give natural-language instructions:

```
Create a critical problem for the login crash we just found,
propose a nil-guard solution, and link it to the current change.
```

```
Show me everything blocking the v1.0 milestone.
```

```
The rate-limiting critique on the auth solution is resolved —
address it and approve the solution with a rationale.
```

```
Search the event log for any decisions we made about caching.
```

## What the Agent Can Do

| Task | What to say |
|------|-------------|
| Triage a bug | "Add a critical problem for X" |
| Start work | "Create a solution for problem Y and attach it to the current change" |
| Code review | "Add a high-severity critique about missing error handling to solution Z" |
| Check status | "What's blocking progress right now?" |
| Roadmap | "Show the milestone roadmap as JSON" |
| Audit | "Search the event log for decisions about authentication" |

## Keeping the Skill Current

The skill file at `jjj.recursivewhy.com/skill.md` is updated with each release. Re-run the install command to get the latest version:

```bash
curl -fsSL https://jjj.recursivewhy.com/skill.md \
  -o ~/.claude/skills/jjj/SKILL.md
```
