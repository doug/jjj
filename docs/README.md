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
│   └── quick-start.md           # 5-minute introduction
├── guides/
│   ├── problem-solving.md       # Creating and managing problems
│   ├── critique-guidelines.md   # Writing and responding to critiques
│   ├── code-review.md           # Code review and LGTM workflow
│   ├── board-dashboard.md       # Board and dashboard views
│   ├── jujutsu-integration.md   # Jujutsu (jj) integration
│   └── vscode-extension.md      # VS Code extension guide
├── reference/
│   ├── cli-problem.md           # Problem commands
│   ├── cli-solution.md          # Solution commands
│   ├── cli-critique.md          # Critique commands
│   ├── cli-milestone.md         # Milestone commands
│   ├── cli-workflow.md          # Workflow commands (init, start, submit, next)
│   └── configuration.md         # Configuration options
├── examples/
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
jjj problem new "Search is slow" --priority high
​```

​```rust
pub struct Problem {
    id: String,
    title: String,
}
​```
```

#### Diagrams (Mermaid)

```markdown
​```mermaid
graph TD
    A[Milestone] --> B[Problem]
    B --> C[Solution]
    C --> D[Critique]
​```
```

### Link Conventions

Use relative links:

```markdown
[Getting Started](../getting-started/quick-start.md)
[CLI Reference](../reference/cli-problem.md)
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
