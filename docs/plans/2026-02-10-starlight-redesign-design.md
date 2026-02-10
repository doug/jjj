# jjj Documentation Redesign: Astro + Starlight

## Overview

Redesign the jjj documentation site from mdbook to Astro + Starlight with React components and shadcn/ui. Goals: polished landing page, visual consistency throughout, light/dark theme support, and a modern developer experience inspired by Raycast developer docs.

## Inspiration

**Primary reference:** [Raycast Developer Docs](https://developers.raycast.com/)

Key qualities to capture:
- Clean, light theme with high readability
- Bold landing/intro pages
- Generous white space
- Icon-driven, categorized navigation
- Three-column layout (nav | content | page ToC)
- Dark code blocks for contrast
- Friendly but professional tone

## Technical Stack

- **Astro** - Static site generator, fast builds, modern DX
- **Starlight** - Astro plugin for documentation (sidebar, search, ToC)
- **React** - Interactive components integration
- **shadcn/ui** - Accessible, customizable component primitives
- **Tailwind CSS** - Utility-first styling (required by Starlight and shadcn)

## Project Structure

```
docs-new/
├── astro.config.mjs
├── tailwind.config.mjs
├── package.json
├── src/
│   ├── components/
│   │   ├── ui/              # shadcn/ui primitives
│   │   ├── Hero.tsx
│   │   ├── FeatureCard.tsx
│   │   ├── Terminal.tsx
│   │   ├── WorkflowDiagram.tsx
│   │   └── ComparisonTable.tsx
│   ├── content/
│   │   └── docs/            # Markdown documentation
│   │       ├── getting-started/
│   │       ├── guides/
│   │       ├── reference/
│   │       └── architecture/
│   ├── pages/
│   │   └── index.astro      # Landing page
│   └── styles/
│       └── globals.css      # Theme tokens, custom styles
├── public/
│   ├── logo.svg             # Placeholder logo
│   └── favicon.svg
```

## Color Palette

### Light Mode (Default)

| Token | Value | Usage |
|-------|-------|-------|
| `--background` | `#FDFBF7` | Page background (warm cream) |
| `--surface` | `#F7F5F0` | Cards, sidebar, elevated surfaces |
| `--text-primary` | `#2D2A26` | Headings, body text |
| `--text-secondary` | `#6B6660` | Muted text, descriptions |
| `--border` | `#E5E2DC` | Dividers, card borders |
| `--code-bg` | `#1E1E1E` | Code block background (dark) |
| `--accent` | `#D946EF` | Links, buttons, highlights (fuchsia) |
| `--accent-hover` | `#E879F9` | Hover states |
| `--success` | `#6B9080` | Success states (muted sage) |
| `--info` | `#5B8A8A` | Info states (soft teal) |

### Dark Mode

| Token | Value | Usage |
|-------|-------|-------|
| `--background` | `#1A1918` | Page background (warm dark) |
| `--surface` | `#252322` | Cards, sidebar, elevated surfaces |
| `--text-primary` | `#F5F3EF` | Headings, body text |
| `--text-secondary` | `#A8A49E` | Muted text, descriptions |
| `--border` | `#3D3A36` | Dividers, card borders |
| `--code-bg` | `#252322` | Code block background |
| `--accent` | `#D946EF` | Links, buttons, highlights (fuchsia) |
| `--accent-hover` | `#E879F9` | Hover states |
| `--success` | `#7CAA97` | Success states (brighter sage) |
| `--info` | `#6FA3A3` | Info states (brighter teal) |

All tones are warm - no cold blue-grays.

## Typography

### Fonts

- **Headings & Body:** Inter or Geist Sans - clean, modern, geometric
- **Code:** Geist Mono or JetBrains Mono - ligatures, clear at small sizes

### Type Scale

| Element | Size | Weight | Notes |
|---------|------|--------|-------|
| Hero headline | 3.5-4rem | 700 | Tight letter-spacing |
| Page title | 2.5rem | 600 | |
| Section heading | 1.5rem | 600 | |
| Body | 1rem (16px) | 400 | Line-height 1.6-1.7 |
| Code | 0.875rem | 400 | Generous block padding |

### Spacing Scale

4, 8, 12, 16, 24, 32, 48, 64, 96px

- Generous margins between sections (64-96px)
- Comfortable padding inside containers (24-32px)
- Content max-width: 720px for optimal line length

## Site Structure

### Landing Page (`/`)

Standalone marketing page, not using docs layout.

**Sections (in order):**

1. **Hero**
   - Headline: "Distributed Project Management for Jujutsu"
   - Subtitle: "Problems, solutions, and critiques — all in your repo"
   - CTAs: "Get Started" (fuchsia primary) / "View on GitHub" (outlined)
   - Terminal demo or code window showing quick workflow

2. **Feature Cards** (grid of 4)
   - Offline First - "All metadata lives in your repo. Works on a plane."
   - Survives Rebases - "Change IDs persist across history rewrites."
   - Critique-Driven - "Solutions must survive criticism before acceptance."
   - No Server Required - "Sync via standard git push/pull."

3. **How It Works + Philosophy**
   - Visual diagram: Problem → Solution → Critique cycle
   - Integrated Popper quote
   - Brief explanation of the Popperian approach

4. **Quick Install**
   - Code snippet: `cargo install jjj`

5. **Comparison Table** (optional/collapsible)
   - jjj vs GitHub Issues vs Linear vs Jira
   - Clean check/cross icons

6. **Footer CTA**
   - "Ready to start?" with link to docs

### Documentation (`/docs/...`)

Three-column Starlight layout:

- **Left sidebar (260px):** Logo, categorized navigation
- **Center content (max 720px):** Markdown with generous margins
- **Right sidebar (200px):** Page ToC, "Edit this page" link

**Navigation categories:**

```
Getting Started
  ├── Installation
  └── Quick Start

User Guides
  ├── Problem Solving
  ├── Code Review Workflow
  ├── Critique Guidelines
  ├── TUI & Status
  ├── Jujutsu Integration
  └── VS Code Extension

CLI Reference
  ├── Problem Commands
  ├── Solution Commands
  ├── Critique Commands
  ├── Milestone Commands
  ├── Workflow Commands
  └── Configuration

Architecture
  ├── Design Philosophy
  ├── Storage & Metadata
  └── Change ID Tracking
```

### Top Navigation

Consistent across all pages:

- Logo + "jjj" wordmark (left)
- GitHub link (right)
- Search with Cmd+K shortcut (right)
- Theme toggle (right)

## Components

### shadcn/ui Components to Use

- `Button` - CTAs, actions
- `Card` - Feature cards, content containers
- `Table` - Comparison table
- `Tabs` - Tabbed code examples
- `Collapsible` - Expandable sections
- `Badge` - Status indicators

### Custom Components

- `Hero.tsx` - Landing page hero section
- `FeatureCard.tsx` - Icon + title + description card
- `Terminal.tsx` - macOS-style code window with syntax highlighting
- `WorkflowDiagram.tsx` - Problem → Solution → Critique cycle visualization
- `ComparisonTable.tsx` - Feature comparison with check/cross icons
- `Callout.tsx` - Tip, warning, note boxes (colored left border)

### Code Blocks

- Dark theme (contrasts with light page background)
- Copy button
- Filename/language tabs
- Syntax highlighting via Shiki (built into Astro)

## Logo

Placeholder for now - simple "jjj" text in heading font or minimal geometric mark.

Proper logo design to be done later once the site is built.

## Migration Plan

1. Set up new Astro + Starlight project in `docs-new/`
2. Configure Tailwind with custom color tokens
3. Install shadcn/ui and configure components
4. Build landing page components
5. Migrate markdown content (adjust frontmatter for Starlight)
6. Build custom doc components (callouts, code blocks)
7. Test light/dark modes
8. Replace old `docs/` with new site
9. Update `book.toml` → `astro.config.mjs` build config

## Future Enhancements

- Interactive code playgrounds
- Animated SVG diagrams
- Command palette search (Cmd+K)
- Versioned documentation
- Proper logo design
- Social proof (GitHub stars, testimonials) when available
