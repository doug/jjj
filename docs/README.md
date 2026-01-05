# jjj Documentation

This directory contains the complete documentation for jjj, built with [MkDocs Material](https://squidfunk.github.io/mkdocs-material/).

## Building the Documentation

### Prerequisites

You need Python 3.8+ and pip installed.

### Setup

1. **Install dependencies**:
   ```bash
   pip install -r requirements.txt
   ```

2. **Serve locally** (with live reload):
   ```bash
   mkdocs serve
   ```

   Open http://127.0.0.1:8000 in your browser.

3. **Build static site**:
   ```bash
   mkdocs build
   ```

   Output will be in `site/` directory.

## Documentation Structure

```
docs/
├── index.md                     # Homepage
├── getting-started/
│   ├── installation.md          # How to install jjj
│   ├── quick-start.md           # 5-minute introduction
│   └── first-project.md         # Complete walkthrough
├── guides/
│   ├── work-hierarchy.md        # Milestones, features, tasks, bugs
│   ├── code-review.md           # Code review workflow
│   ├── task-management.md       # Task tracking
│   └── board-dashboard.md       # Board and dashboard views
├── reference/
│   ├── cli.md                   # Complete CLI reference
│   ├── cli-task.md              # Task commands
│   ├── cli-feature.md           # Feature commands
│   ├── cli-milestone.md         # Milestone commands
│   ├── cli-bug.md               # Bug commands
│   ├── cli-review.md            # Review commands
│   └── configuration.md         # Configuration options
├── examples/
│   ├── feature-workflow.md      # Real-world feature development
│   ├── bug-triage.md            # Bug tracking workflow
│   ├── release-planning.md      # Release management
│   └── code-review-process.md   # Review process examples
└── architecture/
    ├── design-philosophy.md     # Why jjj exists
    ├── storage.md               # Shadow graph and metadata
    ├── change-tracking.md       # Change ID stability
    └── comment-relocation.md    # Context fingerprinting
```

## Writing Documentation

### Style Guide

- **Clear and concise**: Get to the point quickly
- **Code examples**: Every concept needs an example
- **User-centric**: Focus on "what can I do" not "how it works"
- **Progressive disclosure**: Basic → Intermediate → Advanced

### Formatting

We use MkDocs Material extensions:

#### Admonitions

```markdown
!!! note "Optional Title"
    This is a note

!!! tip
    This is a helpful tip

!!! warning
    This is a warning

!!! danger
    This is dangerous!
```

#### Code Blocks

```markdown
​```bash
jjj task new "My task" --feature F-1
​```

​```rust
pub struct Task {
    id: String,
    title: String,
}
​```
```

#### Tabs

```markdown
=== "macOS"

    ​```bash
    brew install jjj
    ​```

=== "Linux"

    ​```bash
    cargo install jjj
    ​```
```

#### Diagrams (Mermaid)

```markdown
​```mermaid
graph TD
    A[Milestone] --> B[Feature]
    B --> C[Task]
​```
```

### Link Conventions

Use relative links:

```markdown
[Getting Started](../getting-started/quick-start.md)
[CLI Reference](../reference/cli.md)
```

## Contributing to Docs

1. **Edit locally**: Make changes and preview with `mkdocs serve`
2. **Check links**: Ensure all links work
3. **Test code examples**: Verify all examples actually work
4. **Check formatting**: Preview renders correctly

## Deploying

### GitHub Pages

```bash
# Build and deploy to gh-pages branch
mkdocs gh-deploy
```

### Manual Deploy

```bash
# Build static site
mkdocs build

# Upload site/ directory to your hosting
rsync -avz site/ user@host:/var/www/docs/
```

## Customization

Edit `mkdocs.yml` in the project root to:

- Change theme colors
- Add/remove navigation sections
- Enable/disable features
- Modify site metadata

## Getting Help

- [MkDocs Documentation](https://www.mkdocs.org/)
- [Material Theme Docs](https://squidfunk.github.io/mkdocs-material/)
- [Markdown Guide](https://www.markdownguide.org/)
