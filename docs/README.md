# jjj Documentation

This directory contains the complete documentation for jjj, built with [mdBook](https://rust-lang.github.io/mdBook/).

## Building the Documentation

### Prerequisites

You need Rust installed (mdBook is a Rust tool).

### Setup

1. **Install mdBook**:
   ```bash
   cargo install mdbook
   ```


3. **Serve locally** (with live reload):
   ```bash
   mdbook serve
   ```

   Open http://localhost:3000 in your browser.

4. **Build static site**:
   ```bash
   mdbook build
   ```

   Output will be in `book/` directory.

## Documentation Structure

```
docs/
├── SUMMARY.md                   # Table of contents (navigation)
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

We use mdBook with mermaid plugins:

#### Admonitions

Use blockquotes with bold titles:

```markdown
> **Note**
>
> This is a note

> **Tip**
>
> This is a helpful tip

> **Warning**
>
> This is a warning
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

## Adding New Pages

1. **Create the markdown file** in the appropriate directory
2. **Add to SUMMARY.md** to include it in navigation:
   ```markdown
   - [New Page Title](path/to/page.md)
   ```
3. **Test locally**: Run `mdbook serve` and verify

## Contributing to Docs

1. **Edit locally**: Make changes and preview with `mdbook serve`
2. **Check links**: Ensure all links work
3. **Test code examples**: Verify all examples actually work
4. **Check formatting**: Preview renders correctly

## Deploying

### GitHub Pages

```bash
# Build the book
mdbook build

# Copy book/ directory to gh-pages branch
git checkout -b gh-pages
cp -r book/* .
git add .
git commit -m "Update documentation"
git push origin gh-pages
```

### Manual Deploy

```bash
# Build static site
mdbook build

# Upload book/ directory to your hosting
rsync -avz book/ user@host:/var/www/docs/
```

## Customization

Edit `book.toml` in the project root to:

- Change theme colors
- Add/remove plugins
- Modify build settings
- Configure search

## Getting Help

- [mdBook Documentation](https://rust-lang.github.io/mdBook/)
- [mdBook Mermaid](https://github.com/badboy/mdbook-mermaid)
- [Markdown Guide](https://www.markdownguide.org/)
