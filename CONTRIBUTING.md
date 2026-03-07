# Contributing to jjj

Thank you for your interest in contributing to jjj! This guide will help you get started.

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Jujutsu (jj)](https://github.com/jj-vcs/jj) installed and on your PATH

### Building

```bash
cargo build              # Debug build
cargo build --release    # Release build
```

### Testing

```bash
cargo test               # Run all tests
cargo test <test_name>   # Run a specific test
```

### Linting and Formatting

```bash
cargo fmt                # Format code
cargo clippy             # Lint
```

All three must pass before submitting changes.

## Making Changes

1. Fork the repository and create a branch (or jj bookmark) for your work.
2. Make your changes, keeping commits focused and well-described.
3. Run `cargo test`, `cargo fmt --check`, and `cargo clippy` locally.
4. Open a pull request against `main`.

## Pull Request Guidelines

- Keep PRs focused on a single change. Smaller PRs are reviewed faster.
- Include a clear description of what changed and why.
- Add or update tests for any new or changed behavior.
- Update documentation if your change affects user-facing behavior.

## Architecture Overview

See [CLAUDE.md](CLAUDE.md) for a detailed architecture guide, including:

- Core model (Problems, Solutions, Critiques, Milestones)
- Storage layer (shadow graph, YAML frontmatter)
- How to add a new command
- Component layers and key files

## Reporting Issues

Use [GitHub Issues](https://github.com/doug/jjj/issues) for bug reports and feature requests. Please use the provided issue templates.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.
