# Installation

This guide will help you install jjj on your system.

## Prerequisites

Before installing jjj, you need:

1. **Jujutsu** (jj) - version 0.12.0 or later
2. **Rust toolchain** - for building from source

### Installing Jujutsu

If you don't have Jujutsu installed yet:

**macOS:**

```bash
brew install jj
```

**Linux:**

```bash
# Arch Linux
pacman -S jujutsu

# Or build from source
cargo install --git https://github.com/martinvonz/jj jj-cli
```

**Windows:**

```powershell
# Using winget
winget install jujutsu

# Or using cargo
cargo install --git https://github.com/martinvonz/jj jj-cli
```

Verify jj is installed:

```bash
jj --version
```

## Installing jjj

### From Source (Current Method)

Currently, jjj must be built from source:

```bash
# Clone the repository
git clone https://github.com/yourusername/jjj.git
cd jjj

# Build and install
cargo install --path .
```

This will install the `jjj` binary to `~/.cargo/bin/`, which should be in your PATH.

### Verify Installation

Check that jjj is installed correctly:

```bash
jjj --version
```

You should see output like:

```
jjj 0.1.0
```

## Shell Completion (Optional)

jjj uses `clap` for CLI argument parsing, which supports shell completion.

### Bash

```bash
# Generate completion script
jjj completion bash > ~/.local/share/bash-completion/completions/jjj

# Or add to your ~/.bashrc:
eval "$(jjj completion bash)"
```

### Zsh

```bash
# Add to your ~/.zshrc:
eval "$(jjj completion zsh)"
```

### Fish

```bash
# Generate completion script
jjj completion fish > ~/.config/fish/completions/jjj.fish
```

> **Completion Support**
>
> Shell completion will be available in a future release. The command structure above is planned but not yet implemented.
```
## Editor Integration

### VSCode

A VSCode extension for jjj is in development. It will provide:

- Interactive board view
- Task management sidebar
- Code review integration
- Quick actions from the command palette

Stay tuned for the official release!

### Other Editors

jjj provides JSON output for all commands via the `--json` flag, making it easy to integrate with any editor:

```bash
# Get tasks as JSON
jjj task list --json

# Get board data as JSON
jjj board --json

# Get feature progress as JSON
jjj feature list --json
```

## Updating jjj

To update jjj to the latest version:

```bash
cd /path/to/jjj/repo
git pull
cargo install --path . --force
```

## Uninstalling

To remove jjj:

```bash
cargo uninstall jjj
```

## Troubleshooting

### jjj command not found

If `jjj` is not found after installation:

1. Ensure `~/.cargo/bin` is in your PATH:
   ```bash
   echo $PATH | grep cargo
   ```

2. Add to your shell profile if needed:
   ```bash
   # ~/.bashrc or ~/.zshrc
   export PATH="$HOME/.cargo/bin:$PATH"
   ```

3. Reload your shell:
   ```bash
   source ~/.bashrc  # or source ~/.zshrc
   ```

### Build Errors

If you encounter build errors:

1. **Update Rust**:
   ```bash
   rustup update
   ```

2. **Clear cargo cache**:
   ```bash
   cargo clean
   cargo build --release
   ```

3. **Check Rust version** (requires 1.70+):
   ```bash
   rustc --version
   ```

### jj not found

If jjj complains that `jj` is not found:

1. Verify jj is installed:
   ```bash
   which jj
   jj --version
   ```

2. Ensure jj is in your PATH
3. Try running with full path:
   ```bash
   JJ_PATH=/path/to/jj jjj init
   ```

## Next Steps

Now that jjj is installed, proceed to:

- [**Quick Start**](quick-start.md) - Get started with your first jjj project
- [**Your First Project**](first-project.md) - A complete walkthrough

## Getting Help

If you encounter issues:

- Check the [troubleshooting section](#troubleshooting) above
- Search [existing issues](https://github.com/yourusername/jjj/issues)
- Ask in [GitHub Discussions](https://github.com/yourusername/jjj/discussions)
- Open a [new issue](https://github.com/yourusername/jjj/issues/new)
