# mdbook Documentation Redesign

**Date:** 2026-02-10
**Status:** Draft

## Overview

Redesign the jjj documentation site to be visually appealing and effectively sell the use of jjj. Transform from stock mdbook styling to a distinctive, warm-but-modern design with a compelling landing page.

## Visual Identity

### Color Palette (Dark Theme Default)

| Role | Color | Usage |
|------|-------|-------|
| Background | `#0d0d14` | Deep purple-black, not pure black |
| Surface | `#1a1a24` | Cards, code blocks, sidebar |
| Text primary | `#e8e6f0` | Body text, warm off-white |
| Text secondary | `#9690b0` | Muted text, labels |
| Accent primary | `#a78bfa` | Links, highlights, key UI elements (violet-400) |
| Accent hover | `#c4b5fd` | Lighter violet for hover states |
| Accent secondary | `#7c3aed` | Buttons, emphasis (violet-600) |

### Typography

- Headings: System sans-serif stack (clean, fast loading)
- Body: Same stack, slightly larger line-height for readability
- Code: JetBrains Mono or system monospace

### Visual Feel

- Generous whitespace
- Soft shadows and rounded corners (8px radius)
- Subtle purple glow effects on key elements (hero, diagrams)
- No harsh borders - use color differentiation instead

---

## Landing Page Structure

### 1. Hero Section

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│              Distributed Project Management                 │
│                        (large, bold)                        │
│                                                             │
│     Problems, solutions, and critiques — all in your repo   │
│                      (muted subtitle)                       │
│                                                             │
│         [Get Started]          [View on GitHub]             │
│         (primary btn)          (secondary btn)              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Styling:**
- Headline: ~48px, white, medium weight
- Subtitle: ~20px, muted purple-gray
- Primary button: Solid violet background, white text
- Secondary button: Transparent with violet border
- Subtle radial gradient glow behind the text (purple → transparent)

---

### 2. Workflow Diagrams

Two diagrams side by side (stacked on mobile).

**Diagram 1: The Core Loop (P→S→C)**

```
    ┌──────────┐      ┌──────────┐      ┌──────────┐
    │ PROBLEM  │─────►│ SOLUTION │─────►│ CRITIQUE │
    │          │      │          │◄─────│          │
    └──────────┘      └──────────┘ refine└──────────┘
     "What to          "Conjecture       "Error
      solve"            to try"        elimination"
```

Label: "The Refinement Cycle"

**Diagram 2: The Hierarchy**

```
    MILESTONES
    ─────────────────────────────────────────────────►

    ┌───────────────┐  ┌───────────────┐  ┌───────────────┐
    │ v0.1 Alpha    │  │ v0.2 Beta     │  │ v1.0 Release  │
    │               │  │               │  │               │
    │ ┌───────────┐ │  │ ┌───────────┐ │  │ ┌───────────┐ │
    │ │ PROBLEM   │ │  │ │ PROBLEM   │ │  │ │ PROBLEM   │ │
    │ │  ┌─────┐  │ │  │ └───────────┘ │  │ └───────────┘ │
    │ │  │ SUB │  │ │  │ ┌───────────┐ │  │               │
    │ │  └─────┘  │ │  │ │ PROBLEM   │ │  │               │
    │ └───────────┘ │  │ └───────────┘ │  │               │
    └───────────────┘  └───────────────┘  └───────────────┘
```

Label: "Roadmap & Structure"

**Implementation:**
- SVG with soft purple fill (`#1a1a24`) and violet borders
- Subtle glow effects
- Responsive: side by side on desktop, stacked on mobile

---

### 3. Terminal Demo

Styled static code blocks showing collaboration workflow:

```
┌─ Terminal ──────────────────────────────────────────────────┐
│                                                             │
│  # Alice identifies a problem                               │
│  alice$ jjj init                                            │
│  Initialized jjj in /projects/myapp                         │
│                                                             │
│  alice$ jjj problem new "Search is slow" --priority P1      │
│  Created p1: Search is slow                                 │
│                                                             │
│  alice$ jjj push                                            │
│  Pushed code and metadata to origin                         │
│                                                             │
│  ──────────────────────────────────────────────────────────│
│                                                             │
│  # Bob proposes a solution                                  │
│  bob$ jjj fetch                                             │
│  Fetched 1 problem                                          │
│                                                             │
│  bob$ jjj solution new "Add search index" --problem p1      │
│  Created s1: Add search index                               │
│  Working copy now at: kpqxywon                              │
│                                                             │
│  bob$ jjj push                                              │
│  Pushed code and metadata to origin                         │
│                                                             │
│  ──────────────────────────────────────────────────────────│
│                                                             │
│  # Alice reviews and critiques                              │
│  alice$ jjj fetch                                           │
│  Fetched 1 solution                                         │
│                                                             │
│  alice$ jjj critique new s1 "Missing error handling"        │
│  Created c1: Missing error handling                         │
│                                                             │
│  alice$ jjj push                                            │
│  Pushed code and metadata to origin                         │
│                                                             │
│  ──────────────────────────────────────────────────────────│
│                                                             │
│  # Bob addresses and resolves                               │
│  bob$ jjj fetch                                             │
│  Fetched 1 critique                                         │
│                                                             │
│  bob$ jjj critique address c1                               │
│  Addressed c1: Missing error handling                       │
│                                                             │
│  bob$ jjj solution accept s1                                │
│  Accepted s1: Add search index                              │
│  Solved p1: Search is slow                                  │
│                                                             │
│  bob$ jjj push                                              │
│  Pushed code and metadata to origin                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Styling:**
- Terminal header bar with fake window controls (subtle)
- Prompt in violet accent color
- Commands in white, output in muted gray
- Subtle rounded corners

---

### 4. Feature Cards

Five cards highlighting key benefits:

| Card | Title | Description |
|------|-------|-------------|
| 1 | Offline First | All metadata lives in your repo. Works on a plane. |
| 2 | Survives Rebases | Change IDs persist across history rewrites. No orphaned references. |
| 3 | Critique Driven | Solutions must survive criticism before acceptance. |
| 4 | No Server Required | Sync via standard git push/pull. Self-host or use any git remote. |
| 5 | AI Agent Native | CLI and text files work seamlessly with AI assistants. Same controls for humans and agents. |

**Styling:**
- Surface background (`#1a1a24`) with subtle violet border on hover
- Small icon/emoji at top
- Bold title, muted description text
- Responsive grid: 3+2 on desktop, single column on mobile

---

### 5. Comparison Table

| | jjj | GitHub Issues | Linear | Jira |
|---|:---:|:---:|:---:|:---:|
| **Works offline** | ✓ | ✗ | ✗ | ✗ |
| **No server required** | ✓ | ✗ | ✗ | ✗ |
| **Survives rebases** | ✓ | ✗ | ✗ | ✗ |
| **Data in your repo** | ✓ | ✗ | ✗ | ✗ |
| **Structured critiques** | ✓ | ✗ | ✗ | ✗ |
| **AI agent native** | ✓ | ✗ | ✗ | ✗ |
| **Built for Jujutsu** | ✓ | ✗ | ✗ | ✗ |
| **Team collaboration** | ✓ | ✓ | ✓ | ✓ |
| **VS Code extension** | ✓ | ✓ | ✓ | ✓ |
| **Terminal TUI** | ✓ | ✗ | ✗ | ✗ |
| **Web UI** | ✗ | ✓ | ✓ | ✓ |
| **Mobile app** | ✗ | ✓ | ✓ | ✗ |

**Styling:**
- Clean table with alternating row backgrounds
- Checkmarks in violet, X marks in muted gray
- Header row slightly brighter
- Intro line: "How jjj compares to hosted project management tools"

---

### 6. Why Popperian?

```
jjj is built on Karl Popper's theory of knowledge growth:
we make progress not by proving ideas right, but by
finding and eliminating errors.

┌─────────────────────────────────────────────────────┐
│                                                     │
│  "All knowledge grows through conjecture and        │
│   refutation. We propose bold ideas, then try       │
│   our hardest to prove them wrong."                 │
│                                                     │
│                        — Karl Popper (paraphrased)  │
│                                                     │
└─────────────────────────────────────────────────────┘

In practice:

• Problems are explicit — not vague tickets, but things
  that need solving

• Solutions are conjectures — tentative attempts, not
  commitments

• Critiques are required — a solution cannot be accepted
  until criticism is addressed

This isn't bureaucracy. It's intellectual honesty
encoded in your workflow.
```

**Styling:**
- Blockquote styled with left violet border
- Link to *Conjectures and Refutations* by Karl Popper
- Link to full Design Philosophy page for deeper reading

---

## Technical Implementation

### Files to Create/Modify

| File | Purpose |
|------|---------|
| `docs/theme/index.hbs` | Custom page template for landing page layout |
| `docs/theme/css/custom.css` | Purple palette, dark theme, component styles |
| `docs/theme/head.hbs` | Custom fonts (if any) |
| `docs/index.md` | Rewritten with HTML blocks for hero, cards, etc. |
| `docs/assets/diagram-cycle.svg` | P→S→C cycle diagram |
| `docs/assets/diagram-roadmap.svg` | Milestones/hierarchy diagram |
| `book.toml` | Updated to use custom theme, default dark |

### Implementation Approach

1. Run `mdbook init --theme` to extract default theme files
2. Modify `index.hbs` to add custom classes for landing page detection
3. Override CSS variables for colors
4. Add custom component styles (hero, cards, terminal, table)

### No JavaScript Required

All styling via CSS. Diagrams as inline SVG.

---

## Page Flow Summary

1. **Hero** — Hook with bold statement
2. **Diagrams** — Explain the model visually (cycle + roadmap)
3. **Demo** — Show collaboration workflow with real commands
4. **Features** — Reinforce key benefits with cards
5. **Comparison** — Differentiate from alternatives
6. **Philosophy** — Depth for the curious

---

## Next Steps

1. Create theme directory and extract default theme
2. Implement CSS with purple palette
3. Create SVG diagrams
4. Rewrite index.md with landing page content
5. Update book.toml configuration
6. Test and iterate
