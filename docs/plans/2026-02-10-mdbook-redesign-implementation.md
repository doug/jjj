# mdbook Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform jjj docs from stock mdbook to a distinctive dark-purple themed site with a compelling landing page.

**Architecture:** Custom mdbook theme with CSS overrides for colors and components. Landing page uses raw HTML blocks in index.md. SVG diagrams inline. No JavaScript required.

**Tech Stack:** mdbook, CSS, SVG, HTML

---

## Task 1: Extract Default Theme

**Files:**
- Create: `docs/theme/` (directory with default theme files)

**Step 1: Extract mdbook default theme**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/mdbook-redesign
mdbook init --theme --force
```

This creates `docs/theme/` with: `index.hbs`, `head.hbs`, `header.hbs`, `css/`, `book.js`, etc.

**Step 2: Verify theme files exist**

```bash
ls docs/theme/
```

Expected: `book.js`, `css/`, `favicon.png`, `favicon.svg`, `fonts/`, `head.hbs`, `header.hbs`, `index.hbs`, `highlight.css`, `highlight.js`

**Step 3: Commit**

```bash
git add docs/theme/
git commit -m "chore: extract default mdbook theme"
```

---

## Task 2: Update book.toml Configuration

**Files:**
- Modify: `book.toml`

**Step 1: Update book.toml with dark theme default and custom CSS**

Replace entire file with:

```toml
[book]
title = "jjj - Jujutsu Project Manager"
authors = ["jjj Contributors"]
description = "Distributed project management and code review for Jujutsu"
language = "en"
src = "docs"

[output.html]
default-theme = "coal"
preferred-dark-theme = "coal"
git-repository-url = "https://github.com/doug/jjj"
edit-url-template = "https://github.com/doug/jjj/edit/main/{path}"
additional-css = ["theme/css/custom.css"]

[output.html.playground]
editable = false
copyable = true
```

**Step 2: Commit**

```bash
git add book.toml
git commit -m "config: set dark theme default and custom CSS"
```

---

## Task 3: Create Custom CSS with Purple Palette

**Files:**
- Create: `docs/theme/css/custom.css`

**Step 1: Create custom.css with full styling**

```css
/* jjj Custom Theme - Purple/Violet Dark Theme */

:root {
    /* Override mdbook CSS variables for coal theme */
    --bg: #0d0d14;
    --fg: #e8e6f0;
    --sidebar-bg: #0d0d14;
    --sidebar-fg: #e8e6f0;
    --sidebar-non-existant: #9690b0;
    --sidebar-active: #a78bfa;
    --sidebar-spacer: #1a1a24;
    --scrollbar: #1a1a24;
    --icons: #9690b0;
    --icons-hover: #a78bfa;
    --links: #a78bfa;
    --inline-code-color: #c4b5fd;
    --theme-popup-bg: #1a1a24;
    --theme-popup-border: #2a2a3a;
    --theme-hover: #2a2a3a;
    --quote-bg: #1a1a24;
    --quote-border: #a78bfa;
    --table-border-color: #2a2a3a;
    --table-header-bg: #1a1a24;
    --table-alternate-bg: #12121a;
    --searchbar-border-color: #2a2a3a;
    --searchbar-bg: #1a1a24;
    --searchbar-fg: #e8e6f0;
    --searchbar-shadow-color: transparent;
    --searchresults-header-fg: #a78bfa;
    --searchresults-border-color: #2a2a3a;
    --searchresults-li-bg: #1a1a24;
    --search-mark-bg: #7c3aed;
}

/* Apply to coal theme specifically */
.coal {
    --bg: #0d0d14;
    --fg: #e8e6f0;
    --sidebar-bg: #0d0d14;
    --sidebar-fg: #e8e6f0;
    --sidebar-non-existant: #9690b0;
    --sidebar-active: #a78bfa;
    --sidebar-spacer: #1a1a24;
    --scrollbar: #1a1a24;
    --icons: #9690b0;
    --icons-hover: #a78bfa;
    --links: #a78bfa;
    --inline-code-color: #c4b5fd;
    --theme-popup-bg: #1a1a24;
    --theme-popup-border: #2a2a3a;
    --theme-hover: #2a2a3a;
    --quote-bg: #1a1a24;
    --quote-border: #a78bfa;
    --table-border-color: #2a2a3a;
    --table-header-bg: #1a1a24;
    --table-alternate-bg: #12121a;
    --searchbar-border-color: #2a2a3a;
    --searchbar-bg: #1a1a24;
    --searchbar-fg: #e8e6f0;
    --searchbar-shadow-color: transparent;
    --searchresults-header-fg: #a78bfa;
    --searchresults-border-color: #2a2a3a;
    --searchresults-li-bg: #1a1a24;
    --search-mark-bg: #7c3aed;
}

/* Typography */
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, "Fira Sans", "Droid Sans", "Helvetica Neue", sans-serif;
    line-height: 1.7;
}

code, pre {
    font-family: "JetBrains Mono", ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace;
}

/* Code blocks */
pre {
    background-color: #1a1a24 !important;
    border-radius: 8px;
    padding: 1rem;
}

/* Links */
a {
    color: #a78bfa;
    text-decoration: none;
}

a:hover {
    color: #c4b5fd;
}

/* ============================================
   LANDING PAGE COMPONENTS
   ============================================ */

/* Hero Section */
.hero {
    text-align: center;
    padding: 4rem 2rem;
    margin: -1rem -1rem 3rem -1rem;
    background: radial-gradient(ellipse at center, rgba(124, 58, 237, 0.15) 0%, transparent 70%);
}

.hero h1 {
    font-size: 3rem;
    font-weight: 600;
    color: #ffffff;
    margin-bottom: 1rem;
    letter-spacing: -0.02em;
}

.hero .subtitle {
    font-size: 1.25rem;
    color: #9690b0;
    margin-bottom: 2rem;
}

.hero .buttons {
    display: flex;
    gap: 1rem;
    justify-content: center;
    flex-wrap: wrap;
}

.hero .btn-primary {
    background: #7c3aed;
    color: #ffffff;
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    font-weight: 500;
    text-decoration: none;
    transition: background 0.2s;
}

.hero .btn-primary:hover {
    background: #6d28d9;
    color: #ffffff;
}

.hero .btn-secondary {
    background: transparent;
    color: #a78bfa;
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    border: 1px solid #a78bfa;
    font-weight: 500;
    text-decoration: none;
    transition: all 0.2s;
}

.hero .btn-secondary:hover {
    background: rgba(167, 139, 250, 0.1);
    color: #c4b5fd;
}

/* Diagrams Section */
.diagrams {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 2rem;
    margin: 3rem 0;
}

.diagram {
    background: #1a1a24;
    border-radius: 12px;
    padding: 2rem;
    text-align: center;
}

.diagram h3 {
    color: #a78bfa;
    font-size: 0.875rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 1.5rem;
}

.diagram svg {
    max-width: 100%;
    height: auto;
}

/* Terminal Demo */
.terminal {
    background: #1a1a24;
    border-radius: 12px;
    overflow: hidden;
    margin: 3rem 0;
    font-family: "JetBrains Mono", ui-monospace, monospace;
    font-size: 0.875rem;
}

.terminal-header {
    background: #12121a;
    padding: 0.75rem 1rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.terminal-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #2a2a3a;
}

.terminal-title {
    color: #9690b0;
    font-size: 0.75rem;
    margin-left: 0.5rem;
}

.terminal-body {
    padding: 1.5rem;
    line-height: 1.6;
}

.terminal-body .comment {
    color: #9690b0;
}

.terminal-body .prompt {
    color: #a78bfa;
}

.terminal-body .command {
    color: #e8e6f0;
}

.terminal-body .output {
    color: #9690b0;
}

.terminal-body .divider {
    border-top: 1px solid #2a2a3a;
    margin: 1rem 0;
}

/* Feature Cards */
.features {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1.5rem;
    margin: 3rem 0;
}

.feature-card {
    background: #1a1a24;
    border-radius: 12px;
    padding: 1.5rem;
    border: 1px solid transparent;
    transition: border-color 0.2s;
}

.feature-card:hover {
    border-color: #a78bfa;
}

.feature-card .icon {
    font-size: 1.5rem;
    margin-bottom: 0.75rem;
}

.feature-card h3 {
    color: #e8e6f0;
    font-size: 1.125rem;
    margin-bottom: 0.5rem;
}

.feature-card p {
    color: #9690b0;
    font-size: 0.875rem;
    line-height: 1.5;
    margin: 0;
}

/* Comparison Table */
.comparison {
    margin: 3rem 0;
}

.comparison h2 {
    text-align: center;
    margin-bottom: 0.5rem;
}

.comparison .subtitle {
    text-align: center;
    color: #9690b0;
    margin-bottom: 2rem;
}

.comparison table {
    width: 100%;
    border-collapse: collapse;
}

.comparison th,
.comparison td {
    padding: 0.75rem 1rem;
    text-align: center;
    border-bottom: 1px solid #2a2a3a;
}

.comparison th {
    background: #1a1a24;
    color: #e8e6f0;
    font-weight: 600;
}

.comparison th:first-child,
.comparison td:first-child {
    text-align: left;
}

.comparison tr:nth-child(even) {
    background: #12121a;
}

.comparison .check {
    color: #a78bfa;
}

.comparison .cross {
    color: #4a4a5a;
}

/* Philosophy Section */
.philosophy {
    background: #1a1a24;
    border-radius: 12px;
    padding: 2rem;
    margin: 3rem 0;
}

.philosophy h2 {
    color: #e8e6f0;
    margin-bottom: 1rem;
}

.philosophy .intro {
    color: #9690b0;
    margin-bottom: 1.5rem;
}

.philosophy blockquote {
    border-left: 3px solid #a78bfa;
    padding-left: 1.5rem;
    margin: 1.5rem 0;
    font-style: italic;
    color: #e8e6f0;
}

.philosophy blockquote cite {
    display: block;
    margin-top: 0.5rem;
    font-style: normal;
    color: #9690b0;
    font-size: 0.875rem;
}

.philosophy ul {
    list-style: none;
    padding: 0;
}

.philosophy li {
    padding: 0.5rem 0;
    padding-left: 1.5rem;
    position: relative;
    color: #9690b0;
}

.philosophy li::before {
    content: "•";
    color: #a78bfa;
    position: absolute;
    left: 0;
}

.philosophy li strong {
    color: #e8e6f0;
}

.philosophy .closing {
    margin-top: 1.5rem;
    font-style: italic;
    color: #e8e6f0;
}

.philosophy .book-link {
    margin-top: 1.5rem;
    padding-top: 1rem;
    border-top: 1px solid #2a2a3a;
    font-size: 0.875rem;
}

/* Section Headers */
.section-header {
    text-align: center;
    margin: 3rem 0 2rem 0;
}

.section-header h2 {
    font-size: 1.75rem;
    color: #e8e6f0;
    margin-bottom: 0.5rem;
}

.section-header p {
    color: #9690b0;
}

/* Responsive */
@media (max-width: 768px) {
    .hero h1 {
        font-size: 2rem;
    }

    .hero .subtitle {
        font-size: 1rem;
    }

    .diagrams {
        grid-template-columns: 1fr;
    }

    .features {
        grid-template-columns: 1fr;
    }

    .comparison {
        overflow-x: auto;
    }
}
```

**Step 2: Commit**

```bash
git add docs/theme/css/custom.css
git commit -m "style: add purple theme custom CSS"
```

---

## Task 4: Create P→S→C Cycle SVG Diagram

**Files:**
- Create: `docs/assets/diagram-cycle.svg`

**Step 1: Create assets directory**

```bash
mkdir -p docs/assets
```

**Step 2: Create diagram-cycle.svg**

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 200">
  <defs>
    <filter id="glow">
      <feGaussianBlur stdDeviation="2" result="coloredBlur"/>
      <feMerge>
        <feMergeNode in="coloredBlur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
  </defs>

  <!-- Problem Box -->
  <rect x="20" y="40" width="140" height="80" rx="8" fill="#1a1a24" stroke="#a78bfa" stroke-width="2"/>
  <text x="90" y="75" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="16" font-weight="600">PROBLEM</text>
  <text x="90" y="100" text-anchor="middle" fill="#9690b0" font-family="system-ui" font-size="12">"What to solve"</text>

  <!-- Arrow 1 -->
  <path d="M 165 80 L 215 80" stroke="#a78bfa" stroke-width="2" fill="none" marker-end="url(#arrowhead)"/>

  <!-- Solution Box -->
  <rect x="230" y="40" width="140" height="80" rx="8" fill="#1a1a24" stroke="#a78bfa" stroke-width="2"/>
  <text x="300" y="75" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="16" font-weight="600">SOLUTION</text>
  <text x="300" y="100" text-anchor="middle" fill="#9690b0" font-family="system-ui" font-size="12">"Conjecture to try"</text>

  <!-- Arrow 2 -->
  <path d="M 375 80 L 425 80" stroke="#a78bfa" stroke-width="2" fill="none" marker-end="url(#arrowhead)"/>

  <!-- Critique Box -->
  <rect x="440" y="40" width="140" height="80" rx="8" fill="#1a1a24" stroke="#a78bfa" stroke-width="2"/>
  <text x="510" y="75" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="16" font-weight="600">CRITIQUE</text>
  <text x="510" y="100" text-anchor="middle" fill="#9690b0" font-family="system-ui" font-size="12">"Error elimination"</text>

  <!-- Refine Arrow (curved, going back) -->
  <path d="M 440 130 C 400 170, 270 170, 230 130" stroke="#a78bfa" stroke-width="2" fill="none" stroke-dasharray="5,5"/>
  <text x="335" y="175" text-anchor="middle" fill="#9690b0" font-family="system-ui" font-size="11" font-style="italic">refine</text>

  <!-- Arrowhead marker -->
  <defs>
    <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
      <polygon points="0 0, 10 3.5, 0 7" fill="#a78bfa"/>
    </marker>
  </defs>
</svg>
```

**Step 3: Commit**

```bash
git add docs/assets/diagram-cycle.svg
git commit -m "assets: add P-S-C cycle diagram SVG"
```

---

## Task 5: Create Roadmap SVG Diagram

**Files:**
- Create: `docs/assets/diagram-roadmap.svg`

**Step 1: Create diagram-roadmap.svg**

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 700 280">
  <!-- Timeline Arrow -->
  <text x="20" y="25" fill="#9690b0" font-family="system-ui" font-size="12" font-weight="500">MILESTONES</text>
  <line x1="20" y1="40" x2="680" y2="40" stroke="#a78bfa" stroke-width="2"/>
  <polygon points="680,40 670,35 670,45" fill="#a78bfa"/>

  <!-- Milestone 1: v0.1 Alpha -->
  <rect x="20" y="60" width="200" height="200" rx="8" fill="#1a1a24" stroke="#2a2a3a" stroke-width="1"/>
  <text x="120" y="90" text-anchor="middle" fill="#a78bfa" font-family="system-ui" font-size="14" font-weight="600">v0.1 Alpha</text>

  <!-- Problem with subproblem -->
  <rect x="40" y="110" width="160" height="130" rx="6" fill="#12121a" stroke="#2a2a3a" stroke-width="1"/>
  <text x="120" y="135" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="12">PROBLEM</text>

  <!-- Subproblem -->
  <rect x="60" y="155" width="120" height="60" rx="4" fill="#1a1a24" stroke="#2a2a3a" stroke-width="1"/>
  <text x="120" y="190" text-anchor="middle" fill="#9690b0" font-family="system-ui" font-size="11">SUB</text>

  <!-- Milestone 2: v0.2 Beta -->
  <rect x="250" y="60" width="200" height="200" rx="8" fill="#1a1a24" stroke="#2a2a3a" stroke-width="1"/>
  <text x="350" y="90" text-anchor="middle" fill="#a78bfa" font-family="system-ui" font-size="14" font-weight="600">v0.2 Beta</text>

  <!-- Two problems -->
  <rect x="270" y="110" width="160" height="50" rx="6" fill="#12121a" stroke="#2a2a3a" stroke-width="1"/>
  <text x="350" y="140" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="12">PROBLEM</text>

  <rect x="270" y="175" width="160" height="50" rx="6" fill="#12121a" stroke="#2a2a3a" stroke-width="1"/>
  <text x="350" y="205" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="12">PROBLEM</text>

  <!-- Milestone 3: v1.0 Release -->
  <rect x="480" y="60" width="200" height="200" rx="8" fill="#1a1a24" stroke="#2a2a3a" stroke-width="1"/>
  <text x="580" y="90" text-anchor="middle" fill="#a78bfa" font-family="system-ui" font-size="14" font-weight="600">v1.0 Release</text>

  <!-- One problem -->
  <rect x="500" y="110" width="160" height="50" rx="6" fill="#12121a" stroke="#2a2a3a" stroke-width="1"/>
  <text x="580" y="140" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="12">PROBLEM</text>
</svg>
```

**Step 2: Commit**

```bash
git add docs/assets/diagram-roadmap.svg
git commit -m "assets: add roadmap diagram SVG"
```

---

## Task 6: Rewrite Landing Page (index.md)

**Files:**
- Modify: `docs/index.md`

**Step 1: Replace docs/index.md with new landing page content**

```markdown
<div class="hero">
  <h1>Distributed Project Management</h1>
  <p class="subtitle">Problems, solutions, and critiques — all in your repo</p>
  <div class="buttons">
    <a href="getting-started/installation.html" class="btn-primary">Get Started</a>
    <a href="https://github.com/doug/jjj" class="btn-secondary">View on GitHub</a>
  </div>
</div>

<div class="diagrams">
  <div class="diagram">
    <h3>The Refinement Cycle</h3>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 200">
      <rect x="20" y="40" width="140" height="80" rx="8" fill="#1a1a24" stroke="#a78bfa" stroke-width="2"/>
      <text x="90" y="75" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="16" font-weight="600">PROBLEM</text>
      <text x="90" y="100" text-anchor="middle" fill="#9690b0" font-family="system-ui" font-size="12">"What to solve"</text>
      <path d="M 165 80 L 215 80" stroke="#a78bfa" stroke-width="2" fill="none"/>
      <polygon points="215,80 205,75 205,85" fill="#a78bfa"/>
      <rect x="230" y="40" width="140" height="80" rx="8" fill="#1a1a24" stroke="#a78bfa" stroke-width="2"/>
      <text x="300" y="75" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="16" font-weight="600">SOLUTION</text>
      <text x="300" y="100" text-anchor="middle" fill="#9690b0" font-family="system-ui" font-size="12">"Conjecture to try"</text>
      <path d="M 375 80 L 425 80" stroke="#a78bfa" stroke-width="2" fill="none"/>
      <polygon points="425,80 415,75 415,85" fill="#a78bfa"/>
      <rect x="440" y="40" width="140" height="80" rx="8" fill="#1a1a24" stroke="#a78bfa" stroke-width="2"/>
      <text x="510" y="75" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="16" font-weight="600">CRITIQUE</text>
      <text x="510" y="100" text-anchor="middle" fill="#9690b0" font-family="system-ui" font-size="12">"Error elimination"</text>
      <path d="M 440 130 C 400 170, 270 170, 230 130" stroke="#a78bfa" stroke-width="2" fill="none" stroke-dasharray="5,5"/>
      <text x="335" y="175" text-anchor="middle" fill="#9690b0" font-family="system-ui" font-size="11" font-style="italic">refine</text>
    </svg>
  </div>
  <div class="diagram">
    <h3>Roadmap & Structure</h3>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 700 280">
      <text x="20" y="25" fill="#9690b0" font-family="system-ui" font-size="12" font-weight="500">MILESTONES</text>
      <line x1="20" y1="40" x2="680" y2="40" stroke="#a78bfa" stroke-width="2"/>
      <polygon points="680,40 670,35 670,45" fill="#a78bfa"/>
      <rect x="20" y="60" width="200" height="200" rx="8" fill="#1a1a24" stroke="#2a2a3a" stroke-width="1"/>
      <text x="120" y="90" text-anchor="middle" fill="#a78bfa" font-family="system-ui" font-size="14" font-weight="600">v0.1 Alpha</text>
      <rect x="40" y="110" width="160" height="130" rx="6" fill="#12121a" stroke="#2a2a3a" stroke-width="1"/>
      <text x="120" y="135" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="12">PROBLEM</text>
      <rect x="60" y="155" width="120" height="60" rx="4" fill="#1a1a24" stroke="#2a2a3a" stroke-width="1"/>
      <text x="120" y="190" text-anchor="middle" fill="#9690b0" font-family="system-ui" font-size="11">SUB</text>
      <rect x="250" y="60" width="200" height="200" rx="8" fill="#1a1a24" stroke="#2a2a3a" stroke-width="1"/>
      <text x="350" y="90" text-anchor="middle" fill="#a78bfa" font-family="system-ui" font-size="14" font-weight="600">v0.2 Beta</text>
      <rect x="270" y="110" width="160" height="50" rx="6" fill="#12121a" stroke="#2a2a3a" stroke-width="1"/>
      <text x="350" y="140" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="12">PROBLEM</text>
      <rect x="270" y="175" width="160" height="50" rx="6" fill="#12121a" stroke="#2a2a3a" stroke-width="1"/>
      <text x="350" y="205" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="12">PROBLEM</text>
      <rect x="480" y="60" width="200" height="200" rx="8" fill="#1a1a24" stroke="#2a2a3a" stroke-width="1"/>
      <text x="580" y="90" text-anchor="middle" fill="#a78bfa" font-family="system-ui" font-size="14" font-weight="600">v1.0 Release</text>
      <rect x="500" y="110" width="160" height="50" rx="6" fill="#12121a" stroke="#2a2a3a" stroke-width="1"/>
      <text x="580" y="140" text-anchor="middle" fill="#e8e6f0" font-family="system-ui" font-size="12">PROBLEM</text>
    </svg>
  </div>
</div>

<div class="terminal">
  <div class="terminal-header">
    <span class="terminal-dot"></span>
    <span class="terminal-dot"></span>
    <span class="terminal-dot"></span>
    <span class="terminal-title">Collaboration in Action</span>
  </div>
  <div class="terminal-body">
    <div class="comment"># Alice identifies a problem</div>
    <div><span class="prompt">alice$</span> <span class="command">jjj init</span></div>
    <div class="output">Initialized jjj in /projects/myapp</div>
    <br>
    <div><span class="prompt">alice$</span> <span class="command">jjj problem new "Search is slow" --priority P1</span></div>
    <div class="output">Created p1: Search is slow</div>
    <br>
    <div><span class="prompt">alice$</span> <span class="command">jjj push</span></div>
    <div class="output">Pushed code and metadata to origin</div>
    <div class="divider"></div>
    <div class="comment"># Bob proposes a solution</div>
    <div><span class="prompt">bob$</span> <span class="command">jjj fetch</span></div>
    <div class="output">Fetched 1 problem</div>
    <br>
    <div><span class="prompt">bob$</span> <span class="command">jjj solution new "Add search index" --problem p1</span></div>
    <div class="output">Created s1: Add search index</div>
    <div class="output">Working copy now at: kpqxywon</div>
    <br>
    <div><span class="prompt">bob$</span> <span class="command">jjj push</span></div>
    <div class="output">Pushed code and metadata to origin</div>
    <div class="divider"></div>
    <div class="comment"># Alice reviews and critiques</div>
    <div><span class="prompt">alice$</span> <span class="command">jjj fetch</span></div>
    <div class="output">Fetched 1 solution</div>
    <br>
    <div><span class="prompt">alice$</span> <span class="command">jjj critique new s1 "Missing error handling"</span></div>
    <div class="output">Created c1: Missing error handling</div>
    <br>
    <div><span class="prompt">alice$</span> <span class="command">jjj push</span></div>
    <div class="output">Pushed code and metadata to origin</div>
    <div class="divider"></div>
    <div class="comment"># Bob addresses and resolves</div>
    <div><span class="prompt">bob$</span> <span class="command">jjj fetch</span></div>
    <div class="output">Fetched 1 critique</div>
    <br>
    <div><span class="prompt">bob$</span> <span class="command">jjj critique address c1</span></div>
    <div class="output">Addressed c1: Missing error handling</div>
    <br>
    <div><span class="prompt">bob$</span> <span class="command">jjj solution accept s1</span></div>
    <div class="output">Accepted s1: Add search index</div>
    <div class="output">Solved p1: Search is slow</div>
    <br>
    <div><span class="prompt">bob$</span> <span class="command">jjj push</span></div>
    <div class="output">Pushed code and metadata to origin</div>
  </div>
</div>

<div class="features">
  <div class="feature-card">
    <div class="icon">📡</div>
    <h3>Offline First</h3>
    <p>All metadata lives in your repo. Works on a plane.</p>
  </div>
  <div class="feature-card">
    <div class="icon">🔀</div>
    <h3>Survives Rebases</h3>
    <p>Change IDs persist across history rewrites. No orphaned references.</p>
  </div>
  <div class="feature-card">
    <div class="icon">💬</div>
    <h3>Critique Driven</h3>
    <p>Solutions must survive criticism before acceptance.</p>
  </div>
  <div class="feature-card">
    <div class="icon">📦</div>
    <h3>No Server Required</h3>
    <p>Sync via standard git push/pull. Self-host or use any git remote.</p>
  </div>
  <div class="feature-card">
    <div class="icon">🤖</div>
    <h3>AI Agent Native</h3>
    <p>CLI and text files work seamlessly with AI assistants. Same controls for humans and agents.</p>
  </div>
</div>

<div class="comparison">
  <h2>How jjj Compares</h2>
  <p class="subtitle">jjj vs hosted project management tools</p>
  <table>
    <thead>
      <tr>
        <th></th>
        <th>jjj</th>
        <th>GitHub Issues</th>
        <th>Linear</th>
        <th>Jira</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td><strong>Works offline</strong></td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td><strong>No server required</strong></td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td><strong>Survives rebases</strong></td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td><strong>Data in your repo</strong></td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td><strong>Structured critiques</strong></td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td><strong>AI agent native</strong></td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td><strong>Built for Jujutsu</strong></td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td><strong>Team collaboration</strong></td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
      </tr>
      <tr>
        <td><strong>VS Code extension</strong></td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
      </tr>
      <tr>
        <td><strong>Terminal TUI</strong></td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td><strong>Web UI</strong></td>
        <td class="cross">✗</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
      </tr>
      <tr>
        <td><strong>Mobile app</strong></td>
        <td class="cross">✗</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
      </tr>
    </tbody>
  </table>
</div>

<div class="philosophy">
  <h2>Why Popperian?</h2>
  <p class="intro">jjj is built on Karl Popper's theory of knowledge growth: we make progress not by proving ideas right, but by finding and eliminating errors.</p>

  <blockquote>
    "All knowledge grows through conjecture and refutation. We propose bold ideas, then try our hardest to prove them wrong."
    <cite>— Karl Popper (paraphrased)</cite>
  </blockquote>

  <p><strong>In practice:</strong></p>
  <ul>
    <li><strong>Problems are explicit</strong> — not vague tickets, but things that need solving</li>
    <li><strong>Solutions are conjectures</strong> — tentative attempts, not commitments</li>
    <li><strong>Critiques are required</strong> — a solution cannot be accepted until criticism is addressed</li>
  </ul>

  <p class="closing">This isn't bureaucracy. It's intellectual honesty encoded in your workflow.</p>

  <p class="book-link">📚 <a href="https://www.amazon.com/Conjectures-Refutations-Scientific-Knowledge-Routledge/dp/0415285941">Conjectures and Refutations</a> — Karl Popper</p>
</div>

---

## Getting Started

Ready to try jjj? Head to the [Installation Guide](getting-started/installation.md) to get started, or check out the [Quick Start](getting-started/quick-start.md) for a hands-on walkthrough.

## Learn More

- [Design Philosophy](architecture/design-philosophy.md) — Deep dive into the Popperian approach
- [CLI Reference](reference/cli-workflow.md) — Full command documentation
- [VS Code Extension](guides/vscode-extension.md) — IDE integration
```

**Step 2: Commit**

```bash
git add docs/index.md
git commit -m "docs: redesign landing page with hero, diagrams, and features"
```

---

## Task 7: Build and Test

**Step 1: Build the book**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/mdbook-redesign
mdbook build
```

Expected: Successful build, output in `book/` directory

**Step 2: Serve locally and verify**

```bash
mdbook serve --open
```

Opens browser at http://localhost:3000. Verify:
- Dark purple theme loads
- Hero section displays centered with gradient glow
- Both diagrams render side by side
- Terminal demo shows colored prompts
- Feature cards in grid layout
- Comparison table with styled checkmarks
- Philosophy section with blockquote styling

**Step 3: Commit any fixes if needed**

After visual verification, if everything looks good:

```bash
git add .
git commit -m "docs: complete mdbook redesign implementation"
```

---

## Task 8: Final Cleanup and PR Prep

**Step 1: Review all changes**

```bash
git log --oneline main..HEAD
git diff main --stat
```

**Step 2: Squash if desired (optional)**

If you want a cleaner history:

```bash
git rebase -i main
# Mark all but first commit as "squash"
```

**Step 3: Push branch**

```bash
git push -u origin feature/mdbook-redesign
```

**Step 4: Create PR**

```bash
gh pr create --title "Redesign mdbook documentation with purple theme" --body "## Summary
- Dark purple/violet theme with custom color palette
- New landing page with hero, workflow diagrams, terminal demo
- Feature cards highlighting key benefits
- Comparison table vs GitHub Issues/Linear/Jira
- Why Popperian philosophy section

## Test plan
- [ ] Run \`mdbook serve\` and verify all sections render correctly
- [ ] Check responsive layout on mobile viewport
- [ ] Verify dark theme colors match design spec
- [ ] Test all links work"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Extract default theme | `docs/theme/*` |
| 2 | Update book.toml | `book.toml` |
| 3 | Create custom CSS | `docs/theme/css/custom.css` |
| 4 | Create cycle diagram | `docs/assets/diagram-cycle.svg` |
| 5 | Create roadmap diagram | `docs/assets/diagram-roadmap.svg` |
| 6 | Rewrite landing page | `docs/index.md` |
| 7 | Build and test | — |
| 8 | Cleanup and PR | — |
