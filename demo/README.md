# Demo Recordings

GIF demos for the project README, recorded with [VHS](https://github.com/charmbracelet/vhs).

## Prerequisites

```bash
brew install vhs
```

VHS also requires `ttyd` and `ffmpeg` (installed automatically by Homebrew as dependencies).

## Recording

Re-record all demos:

```bash
for tape in demo/*.tape; do vhs "$tape"; done
```

Or individually:

```bash
vhs demo/workflow.tape
vhs demo/tui.tape
vhs demo/code-review.tape
vhs demo/problem-solving.tape
vhs demo/jj-integration.tape
```

Each tape outputs a `.gif` in the `demo/` directory. GIFs are used in both the project `README.md` and the docs site (`docs-site/public/demo/`).

After re-recording, copy to the docs site:

```bash
cp demo/*.gif docs-site/public/demo/
```

## Tapes

| File | Output | Used in |
|------|--------|---------|
| `workflow.tape` | `workflow.gif` | README, docs landing page, Quick Start guide |
| `tui.tape` | `tui.gif` | README, TUI & Status guide |
| `code-review.tape` | `code-review.gif` | Code Review Workflow guide |
| `problem-solving.tape` | `problem-solving.gif` | Problem Solving guide |
| `jj-integration.tape` | `jj-integration.gif` | Jujutsu Integration guide |

## Editing

Tape files use VHS syntax — see the [VHS docs](https://github.com/charmbracelet/vhs#vhs) for the full reference.

Key commands used:
- `Hide` / `Show` — hide setup commands from the recording
- `Type "..."` — type text into the terminal
- `Enter`, `Tab`, `Down`, `Right` — send key presses
- `Sleep <duration>` — pause between actions
- `Set Theme "Catppuccin Mocha"` — terminal theme

Each tape creates a temporary jj repository in a hidden setup block, populates it with sample data, then demonstrates jjj commands.

## Tips

- After editing a tape, re-record and check the GIF before committing
- Keep GIFs under 2MB for fast README loading on GitHub
- Use `ffmpeg -i demo/workflow.gif -vf "select=gte(t\,15)" -vframes 1 /tmp/frame.png` to extract a frame for inspection
