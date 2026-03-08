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
cd demo
vhs workflow.tape
vhs tui.tape
```

Or from the project root:

```bash
vhs demo/workflow.tape
vhs demo/tui.tape
```

Each tape outputs a `.gif` in the `demo/` directory. The GIFs are referenced by `../README.md`.

## Tapes

| File | Output | Description |
|------|--------|-------------|
| `workflow.tape` | `workflow.gif` | Full Popperian cycle: create problem, propose solution, add critique, address it, submit, approve |
| `tui.tape` | `tui.gif` | Interactive TUI: navigate panes, expand tree nodes, scroll detail, help overlay |

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
