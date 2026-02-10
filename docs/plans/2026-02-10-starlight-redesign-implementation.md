# Starlight Documentation Redesign - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace mdbook with Astro + Starlight documentation site featuring a polished landing page, warm color theme with fuchsia accents, and light/dark mode support.

**Architecture:** New `docs-site/` directory with Astro + Starlight + React + Tailwind. Landing page is a standalone Astro page; docs use Starlight's three-column layout. Markdown content migrates with frontmatter adjustments.

**Tech Stack:** Astro 4.x, @astrojs/starlight, @astrojs/react, @astrojs/tailwind, shadcn/ui components, Geist font family

---

## Task 1: Initialize Astro + Starlight Project

**Files:**
- Create: `docs-site/package.json`
- Create: `docs-site/astro.config.mjs`
- Create: `docs-site/tsconfig.json`

**Step 1: Create project directory and initialize**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign
mkdir docs-site
cd docs-site
npm create astro@latest . -- --template starlight --yes --no-git --no-install
```

**Step 2: Install dependencies**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
npm install
```

**Step 3: Add React and Tailwind integrations**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
npx astro add react tailwind --yes
```

**Step 4: Verify dev server starts**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
npm run dev &
sleep 5
curl -s http://localhost:4321 | head -20
pkill -f "astro dev"
```

Expected: HTML response with Starlight content

**Step 5: Commit**

```bash
git add docs-site/
git commit -m "feat(docs): initialize Astro + Starlight project"
```

---

## Task 2: Configure Custom Theme Colors

**Files:**
- Modify: `docs-site/astro.config.mjs`
- Create: `docs-site/src/styles/custom.css`
- Modify: `docs-site/tailwind.config.mjs`

**Step 1: Create custom CSS with color tokens**

Create `docs-site/src/styles/custom.css`:

```css
/* Warm theme with fuchsia accent */
:root {
  /* Light mode */
  --sl-color-white: #FDFBF7;
  --sl-color-gray-1: #F7F5F0;
  --sl-color-gray-2: #E5E2DC;
  --sl-color-gray-3: #C5C0B8;
  --sl-color-gray-4: #A8A49E;
  --sl-color-gray-5: #6B6660;
  --sl-color-gray-6: #2D2A26;
  --sl-color-black: #1A1918;

  --sl-color-accent-low: #FDF4FF;
  --sl-color-accent: #D946EF;
  --sl-color-accent-high: #A21CAF;

  --sl-color-text: var(--sl-color-gray-6);
  --sl-color-text-accent: var(--sl-color-accent);
  --sl-color-bg: var(--sl-color-white);
  --sl-color-bg-nav: var(--sl-color-gray-1);
  --sl-color-bg-sidebar: var(--sl-color-gray-1);
  --sl-color-hairline-light: var(--sl-color-gray-2);
  --sl-color-hairline: var(--sl-color-gray-2);
}

:root[data-theme='dark'] {
  /* Dark mode */
  --sl-color-white: #F5F3EF;
  --sl-color-gray-1: #A8A49E;
  --sl-color-gray-2: #6B6660;
  --sl-color-gray-3: #3D3A36;
  --sl-color-gray-4: #2D2A26;
  --sl-color-gray-5: #252322;
  --sl-color-gray-6: #1A1918;
  --sl-color-black: #0F0E0D;

  --sl-color-accent-low: #4A1D5C;
  --sl-color-accent: #D946EF;
  --sl-color-accent-high: #F0ABFC;

  --sl-color-text: var(--sl-color-white);
  --sl-color-bg: var(--sl-color-gray-6);
  --sl-color-bg-nav: var(--sl-color-gray-5);
  --sl-color-bg-sidebar: var(--sl-color-gray-5);
  --sl-color-hairline-light: var(--sl-color-gray-3);
  --sl-color-hairline: var(--sl-color-gray-3);
}

/* Typography - Geist font */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');

:root {
  --sl-font: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  --sl-font-mono: 'Geist Mono', 'JetBrains Mono', 'Fira Code', monospace;
}

/* Code blocks - dark theme */
.expressive-code {
  --ec-codeBg: #1E1E1E;
  --ec-codeSelectionBg: #3D3A36;
  --ec-codePadBlock: 1.5rem;
  --ec-codePadInline: 1.5rem;
  --ec-codeFontFam: var(--sl-font-mono);
  --ec-uiFontFam: var(--sl-font);
  --ec-brdRad: 8px;
}

/* Generous spacing */
.sl-markdown-content {
  --sl-content-width: 720px;
}

.sl-markdown-content > * + * {
  margin-top: 1.5rem;
}

.sl-markdown-content h2 {
  margin-top: 3rem;
}

.sl-markdown-content h3 {
  margin-top: 2rem;
}
```

**Step 2: Update Tailwind config with custom colors**

Replace `docs-site/tailwind.config.mjs`:

```js
import starlightPlugin from '@astrojs/starlight-tailwind';

/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}'],
  theme: {
    extend: {
      colors: {
        // Light mode base
        background: '#FDFBF7',
        surface: '#F7F5F0',
        border: '#E5E2DC',
        // Text
        'text-primary': '#2D2A26',
        'text-secondary': '#6B6660',
        // Accent
        accent: {
          DEFAULT: '#D946EF',
          hover: '#E879F9',
          low: '#FDF4FF',
          high: '#A21CAF',
        },
        // Semantic
        success: '#6B9080',
        info: '#5B8A8A',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['Geist Mono', 'JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [starlightPlugin()],
};
```

**Step 3: Update Astro config to use custom CSS**

Replace `docs-site/astro.config.mjs`:

```js
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import react from '@astrojs/react';
import tailwind from '@astrojs/tailwind';

export default defineConfig({
  integrations: [
    starlight({
      title: 'jjj',
      description: 'Distributed project management for Jujutsu',
      defaultLocale: 'root',
      locales: {
        root: { label: 'English', lang: 'en' },
      },
      social: {
        github: 'https://github.com/doug/jjj',
      },
      customCss: ['./src/styles/custom.css'],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Installation', slug: 'getting-started/installation' },
            { label: 'Quick Start', slug: 'getting-started/quick-start' },
          ],
        },
        {
          label: 'User Guides',
          items: [
            { label: 'Problem Solving', slug: 'guides/problem-solving' },
            { label: 'Critique Guidelines', slug: 'guides/critique-guidelines' },
            { label: 'Code Review Workflow', slug: 'guides/code-review' },
            { label: 'TUI & Status', slug: 'guides/board-dashboard' },
            { label: 'Jujutsu Integration', slug: 'guides/jujutsu-integration' },
            { label: 'VS Code Extension', slug: 'guides/vscode-extension' },
          ],
        },
        {
          label: 'CLI Reference',
          items: [
            { label: 'Entity Resolution', slug: 'reference/entity-resolution' },
            { label: 'Problem Commands', slug: 'reference/cli-problem' },
            { label: 'Solution Commands', slug: 'reference/cli-solution' },
            { label: 'Critique Commands', slug: 'reference/cli-critique' },
            { label: 'Milestone Commands', slug: 'reference/cli-milestone' },
            { label: 'Workflow Commands', slug: 'reference/cli-workflow' },
            { label: 'Configuration', slug: 'reference/configuration' },
          ],
        },
        {
          label: 'Architecture',
          items: [
            { label: 'Design Philosophy', slug: 'architecture/design-philosophy' },
            { label: 'Storage & Metadata', slug: 'architecture/storage' },
            { label: 'Change ID Tracking', slug: 'architecture/change-tracking' },
          ],
        },
      ],
    }),
    react(),
    tailwind({ applyBaseStyles: false }),
  ],
});
```

**Step 4: Verify theme applies**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
npm run dev &
sleep 5
curl -s http://localhost:4321 | grep -o "FDFBF7\|D946EF" | head -5
pkill -f "astro dev"
```

Expected: Color values appear in output

**Step 5: Commit**

```bash
git add docs-site/
git commit -m "feat(docs): configure warm theme with fuchsia accent"
```

---

## Task 3: Create Placeholder Logo and Favicon

**Files:**
- Create: `docs-site/public/logo.svg`
- Create: `docs-site/public/favicon.svg`
- Modify: `docs-site/astro.config.mjs`

**Step 1: Create placeholder logo SVG**

Create `docs-site/public/logo.svg`:

```svg
<svg width="32" height="32" viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
  <rect width="32" height="32" rx="6" fill="#D946EF"/>
  <text x="16" y="22" text-anchor="middle" fill="white" font-family="Inter, sans-serif" font-weight="700" font-size="14">jjj</text>
</svg>
```

**Step 2: Create favicon SVG**

Create `docs-site/public/favicon.svg`:

```svg
<svg width="32" height="32" viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
  <rect width="32" height="32" rx="6" fill="#D946EF"/>
  <text x="16" y="22" text-anchor="middle" fill="white" font-family="Inter, sans-serif" font-weight="700" font-size="14">jjj</text>
</svg>
```

**Step 3: Update Astro config to use logo**

Add to starlight config in `docs-site/astro.config.mjs` (inside the starlight() call):

```js
      logo: {
        src: './public/logo.svg',
        alt: 'jjj',
      },
      favicon: '/favicon.svg',
```

**Step 4: Commit**

```bash
git add docs-site/public/ docs-site/astro.config.mjs
git commit -m "feat(docs): add placeholder logo and favicon"
```

---

## Task 4: Migrate Documentation Content

**Files:**
- Create: `docs-site/src/content/docs/` (directory structure)
- Copy and modify all markdown files from `docs/`

**Step 1: Create content directory structure**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
mkdir -p src/content/docs/getting-started
mkdir -p src/content/docs/guides
mkdir -p src/content/docs/reference
mkdir -p src/content/docs/architecture
```

**Step 2: Copy markdown files**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign

# Getting started
cp docs/getting-started/installation.md docs-site/src/content/docs/getting-started/
cp docs/getting-started/quick-start.md docs-site/src/content/docs/getting-started/

# Guides
cp docs/guides/problem-solving.md docs-site/src/content/docs/guides/
cp docs/guides/critique-guidelines.md docs-site/src/content/docs/guides/
cp docs/guides/code-review.md docs-site/src/content/docs/guides/
cp docs/guides/board-dashboard.md docs-site/src/content/docs/guides/
cp docs/guides/jujutsu-integration.md docs-site/src/content/docs/guides/
cp docs/guides/vscode-extension.md docs-site/src/content/docs/guides/

# Reference
cp docs/reference/cli-problem.md docs-site/src/content/docs/reference/
cp docs/reference/cli-solution.md docs-site/src/content/docs/reference/
cp docs/reference/cli-critique.md docs-site/src/content/docs/reference/
cp docs/reference/cli-milestone.md docs-site/src/content/docs/reference/
cp docs/reference/cli-workflow.md docs-site/src/content/docs/reference/
cp docs/reference/configuration.md docs-site/src/content/docs/reference/

# Architecture
cp docs/architecture/design-philosophy.md docs-site/src/content/docs/architecture/
cp docs/architecture/storage.md docs-site/src/content/docs/architecture/
cp docs/architecture/change-tracking.md docs-site/src/content/docs/architecture/
```

**Step 3: Check for entity-resolution.md and create if needed**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign
if [ -f docs/reference/entity-resolution.md ]; then
  cp docs/reference/entity-resolution.md docs-site/src/content/docs/reference/
else
  echo "Need to create entity-resolution.md"
fi
```

If file doesn't exist, create `docs-site/src/content/docs/reference/entity-resolution.md`:

```markdown
---
title: Entity Resolution
description: How jjj resolves entity references by UUID, prefix, or fuzzy title match
---

# Entity Resolution

jjj supports flexible entity resolution, allowing you to reference problems, solutions, critiques, and milestones in multiple ways.

## Resolution Methods

### Full UUID

```bash
jjj problem show 01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a
```

### Truncated Prefix

Use at least 6 hex characters:

```bash
jjj problem show 01957d
```

### Fuzzy Title Match

Match by title keywords:

```bash
jjj problem show "auth bug"
```

## Type Prefixes

In mixed-type contexts (like `jjj show`), use type prefixes:

- `p/` - Problem
- `s/` - Solution
- `c/` - Critique
- `m/` - Milestone

```bash
jjj show p/01957d
jjj show s/"add caching"
```

## Listing Display

Lists automatically show the shortest unique prefix for each entity:

```
01957d  Open    Authentication fails on refresh
01958a  Solved  Database connection timeout
```
```

**Step 4: Update frontmatter for Starlight format**

Each markdown file needs Starlight-compatible frontmatter. For each file, ensure it has:

```yaml
---
title: Page Title
description: Brief description for SEO
---
```

Run this to check/fix frontmatter (manual review needed):

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
for f in src/content/docs/**/*.md; do
  echo "=== $f ==="
  head -10 "$f"
  echo ""
done
```

**Step 5: Verify docs build**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
npm run build
```

Expected: Build completes without errors

**Step 6: Commit**

```bash
git add docs-site/src/content/
git commit -m "feat(docs): migrate markdown content to Starlight"
```

---

## Task 5: Set Up shadcn/ui Components

**Files:**
- Create: `docs-site/components.json`
- Create: `docs-site/src/components/ui/button.tsx`
- Create: `docs-site/src/components/ui/card.tsx`
- Create: `docs-site/src/lib/utils.ts`

**Step 1: Install shadcn/ui dependencies**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
npm install class-variance-authority clsx tailwind-merge lucide-react
npm install -D @types/node
```

**Step 2: Create utils file**

Create `docs-site/src/lib/utils.ts`:

```typescript
import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

**Step 3: Create Button component**

Create `docs-site/src/components/ui/button.tsx`:

```tsx
import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../../lib/utils';

const buttonVariants = cva(
  'inline-flex items-center justify-center whitespace-nowrap rounded-lg text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:pointer-events-none disabled:opacity-50',
  {
    variants: {
      variant: {
        default: 'bg-accent text-white hover:bg-accent-hover',
        outline: 'border border-border bg-transparent hover:bg-surface hover:border-accent hover:text-accent',
        ghost: 'hover:bg-surface hover:text-accent',
      },
      size: {
        default: 'h-10 px-6 py-2',
        sm: 'h-8 px-4 text-xs',
        lg: 'h-12 px-8 text-base',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => {
    return (
      <button
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  }
);
Button.displayName = 'Button';

export { Button, buttonVariants };
```

**Step 4: Create Card component**

Create `docs-site/src/components/ui/card.tsx`:

```tsx
import * as React from 'react';
import { cn } from '../../lib/utils';

const Card = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={cn(
      'rounded-xl border border-border bg-surface p-6 transition-all hover:border-accent/50 hover:shadow-md',
      className
    )}
    {...props}
  />
));
Card.displayName = 'Card';

const CardHeader = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div ref={ref} className={cn('flex flex-col space-y-1.5', className)} {...props} />
));
CardHeader.displayName = 'CardHeader';

const CardTitle = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLHeadingElement>
>(({ className, ...props }, ref) => (
  <h3
    ref={ref}
    className={cn('text-lg font-semibold text-text-primary', className)}
    {...props}
  />
));
CardTitle.displayName = 'CardTitle';

const CardDescription = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLParagraphElement>
>(({ className, ...props }, ref) => (
  <p ref={ref} className={cn('text-sm text-text-secondary', className)} {...props} />
));
CardDescription.displayName = 'CardDescription';

const CardContent = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div ref={ref} className={cn('pt-4', className)} {...props} />
));
CardContent.displayName = 'CardContent';

export { Card, CardHeader, CardTitle, CardDescription, CardContent };
```

**Step 5: Commit**

```bash
git add docs-site/src/components/ docs-site/src/lib/
git commit -m "feat(docs): add shadcn/ui Button and Card components"
```

---

## Task 6: Build Landing Page Hero Component

**Files:**
- Create: `docs-site/src/components/Hero.tsx`

**Step 1: Create Hero component**

Create `docs-site/src/components/Hero.tsx`:

```tsx
import { Button } from './ui/button';

export function Hero() {
  return (
    <section className="relative overflow-hidden py-24 lg:py-32">
      {/* Background gradient */}
      <div className="absolute inset-0 -z-10">
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[800px] bg-gradient-radial from-accent/10 via-accent/5 to-transparent rounded-full blur-3xl" />
      </div>

      <div className="container mx-auto px-6 text-center">
        {/* Logo placeholder */}
        <div className="mb-8">
          <span className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-accent text-white text-2xl font-bold">
            jjj
          </span>
        </div>

        {/* Headline */}
        <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold text-text-primary tracking-tight mb-6">
          Distributed Project Management
          <br />
          <span className="text-accent">for Jujutsu</span>
        </h1>

        {/* Subtitle */}
        <p className="text-xl text-text-secondary max-w-2xl mx-auto mb-10">
          Problems, solutions, and critiques — all in your repo.
          <br />
          No server. No database. Works offline.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row gap-4 justify-center">
          <a href="/docs/getting-started/installation">
            <Button size="lg">Get Started</Button>
          </a>
          <a href="https://github.com/doug/jjj" target="_blank" rel="noopener noreferrer">
            <Button variant="outline" size="lg">
              View on GitHub
            </Button>
          </a>
        </div>
      </div>
    </section>
  );
}
```

**Step 2: Commit**

```bash
git add docs-site/src/components/Hero.tsx
git commit -m "feat(docs): add Hero component for landing page"
```

---

## Task 7: Build Feature Cards Component

**Files:**
- Create: `docs-site/src/components/FeatureCard.tsx`
- Create: `docs-site/src/components/Features.tsx`

**Step 1: Create FeatureCard component**

Create `docs-site/src/components/FeatureCard.tsx`:

```tsx
import { Card, CardHeader, CardTitle, CardDescription } from './ui/card';

interface FeatureCardProps {
  icon: React.ReactNode;
  title: string;
  description: string;
}

export function FeatureCard({ icon, title, description }: FeatureCardProps) {
  return (
    <Card>
      <CardHeader>
        <div className="text-3xl mb-2">{icon}</div>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
    </Card>
  );
}
```

**Step 2: Create Features section component**

Create `docs-site/src/components/Features.tsx`:

```tsx
import { FeatureCard } from './FeatureCard';

const features = [
  {
    icon: '📡',
    title: 'Offline First',
    description: 'All metadata lives in your repo. Works on a plane.',
  },
  {
    icon: '🔀',
    title: 'Survives Rebases',
    description: 'Change IDs persist across history rewrites.',
  },
  {
    icon: '💬',
    title: 'Critique-Driven',
    description: 'Solutions must survive criticism before acceptance.',
  },
  {
    icon: '📦',
    title: 'No Server Required',
    description: 'Sync via standard git push/pull.',
  },
];

export function Features() {
  return (
    <section className="py-24 bg-surface">
      <div className="container mx-auto px-6">
        <h2 className="text-3xl font-bold text-center text-text-primary mb-12">
          Why jjj?
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {features.map((feature) => (
            <FeatureCard key={feature.title} {...feature} />
          ))}
        </div>
      </div>
    </section>
  );
}
```

**Step 3: Commit**

```bash
git add docs-site/src/components/FeatureCard.tsx docs-site/src/components/Features.tsx
git commit -m "feat(docs): add FeatureCard and Features components"
```

---

## Task 8: Build Terminal Demo Component

**Files:**
- Create: `docs-site/src/components/Terminal.tsx`

**Step 1: Create Terminal component**

Create `docs-site/src/components/Terminal.tsx`:

```tsx
interface TerminalLine {
  type: 'command' | 'output';
  user?: string;
  content: string;
}

interface TerminalProps {
  title?: string;
  lines: TerminalLine[];
}

export function Terminal({ title = 'Terminal', lines }: TerminalProps) {
  return (
    <div className="rounded-xl overflow-hidden border border-border shadow-lg">
      {/* Title bar */}
      <div className="bg-[#2D2A26] px-4 py-3 flex items-center gap-2">
        <div className="flex gap-2">
          <div className="w-3 h-3 rounded-full bg-[#ff5f56]" />
          <div className="w-3 h-3 rounded-full bg-[#ffbd2e]" />
          <div className="w-3 h-3 rounded-full bg-[#27c93f]" />
        </div>
        <span className="ml-3 text-sm text-gray-400 font-mono">{title}</span>
      </div>

      {/* Terminal content */}
      <div className="bg-[#1E1E1E] p-6 font-mono text-sm leading-relaxed">
        {lines.map((line, i) => (
          <div key={i} className="mb-1">
            {line.type === 'command' ? (
              <div>
                <span className="text-accent">{line.user || 'user'}</span>
                <span className="text-gray-500"> $ </span>
                <span className="text-gray-100">{line.content}</span>
              </div>
            ) : (
              <div className="text-gray-400 pl-0">{line.content}</div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
```

**Step 2: Commit**

```bash
git add docs-site/src/components/Terminal.tsx
git commit -m "feat(docs): add Terminal demo component"
```

---

## Task 9: Build Workflow Diagram Component

**Files:**
- Create: `docs-site/src/components/WorkflowDiagram.tsx`
- Create: `docs-site/src/components/HowItWorks.tsx`

**Step 1: Create WorkflowDiagram component**

Create `docs-site/src/components/WorkflowDiagram.tsx`:

```tsx
export function WorkflowDiagram() {
  return (
    <div className="flex flex-col md:flex-row items-center justify-center gap-4 md:gap-8 py-8">
      {/* Problem */}
      <div className="flex flex-col items-center">
        <div className="w-32 h-24 rounded-xl bg-red-500/90 flex items-center justify-center text-white font-bold text-lg shadow-lg">
          PROBLEM
        </div>
        <p className="mt-2 text-sm text-text-secondary">Identify & articulate</p>
      </div>

      {/* Arrow */}
      <div className="text-2xl text-accent hidden md:block">→</div>
      <div className="text-2xl text-accent md:hidden">↓</div>

      {/* Solution */}
      <div className="flex flex-col items-center">
        <div className="w-32 h-24 rounded-xl bg-blue-500/90 flex items-center justify-center text-white font-bold text-lg shadow-lg">
          SOLUTION
        </div>
        <p className="mt-2 text-sm text-text-secondary">Propose conjecture</p>
      </div>

      {/* Arrow */}
      <div className="text-2xl text-accent hidden md:block">→</div>
      <div className="text-2xl text-accent md:hidden">↓</div>

      {/* Critique */}
      <div className="flex flex-col items-center">
        <div className="w-32 h-24 rounded-xl bg-purple-500/90 flex items-center justify-center text-white font-bold text-lg shadow-lg">
          CRITIQUE
        </div>
        <p className="mt-2 text-sm text-text-secondary">Eliminate errors</p>
      </div>

      {/* Refine loop */}
      <div className="hidden md:flex flex-col items-center ml-4">
        <div className="text-success text-xl">↻</div>
        <p className="text-sm text-success italic">refine</p>
      </div>
    </div>
  );
}
```

**Step 2: Create HowItWorks section**

Create `docs-site/src/components/HowItWorks.tsx`:

```tsx
import { WorkflowDiagram } from './WorkflowDiagram';

export function HowItWorks() {
  return (
    <section className="py-24">
      <div className="container mx-auto px-6">
        <h2 className="text-3xl font-bold text-center text-text-primary mb-4">
          How It Works
        </h2>
        <p className="text-center text-text-secondary max-w-2xl mx-auto mb-12">
          jjj implements Popperian epistemology: knowledge grows through bold
          conjectures and rigorous criticism.
        </p>

        <WorkflowDiagram />

        <blockquote className="mt-12 max-w-2xl mx-auto border-l-4 border-accent pl-6 italic text-text-secondary">
          "The method of science is the method of bold conjectures and ingenious
          and severe attempts to refute them."
          <cite className="block mt-2 text-sm text-text-secondary/70 not-italic">
            — Karl Popper
          </cite>
        </blockquote>
      </div>
    </section>
  );
}
```

**Step 3: Commit**

```bash
git add docs-site/src/components/WorkflowDiagram.tsx docs-site/src/components/HowItWorks.tsx
git commit -m "feat(docs): add WorkflowDiagram and HowItWorks components"
```

---

## Task 10: Build Quick Install Component

**Files:**
- Create: `docs-site/src/components/QuickInstall.tsx`

**Step 1: Create QuickInstall component**

Create `docs-site/src/components/QuickInstall.tsx`:

```tsx
export function QuickInstall() {
  return (
    <section className="py-16 bg-surface">
      <div className="container mx-auto px-6 text-center">
        <h2 className="text-2xl font-bold text-text-primary mb-4">
          Quick Install
        </h2>
        <p className="text-text-secondary mb-6">
          Get started in seconds with Cargo:
        </p>
        <div className="inline-flex items-center gap-3 bg-[#1E1E1E] rounded-lg px-6 py-3 font-mono text-gray-100">
          <span className="text-accent">$</span>
          <code>cargo install jjj</code>
          <button
            onClick={() => navigator.clipboard.writeText('cargo install jjj')}
            className="ml-2 text-gray-400 hover:text-accent transition-colors"
            title="Copy to clipboard"
          >
            📋
          </button>
        </div>
      </div>
    </section>
  );
}
```

**Step 2: Commit**

```bash
git add docs-site/src/components/QuickInstall.tsx
git commit -m "feat(docs): add QuickInstall component"
```

---

## Task 11: Build Comparison Table Component

**Files:**
- Create: `docs-site/src/components/ComparisonTable.tsx`

**Step 1: Create ComparisonTable component**

Create `docs-site/src/components/ComparisonTable.tsx`:

```tsx
const features = [
  { name: 'Works offline', jjj: true, github: false, linear: false, jira: false },
  { name: 'No server required', jjj: true, github: false, linear: false, jira: false },
  { name: 'Survives rebases', jjj: true, github: false, linear: false, jira: false },
  { name: 'Data in your repo', jjj: true, github: false, linear: false, jira: false },
  { name: 'Structured critiques', jjj: true, github: false, linear: false, jira: false },
  { name: 'AI agent native', jjj: true, github: false, linear: false, jira: false },
  { name: 'Built for Jujutsu', jjj: true, github: false, linear: false, jira: false },
  { name: 'Team collaboration', jjj: true, github: true, linear: true, jira: true },
  { name: 'Web UI', jjj: false, github: true, linear: true, jira: true },
  { name: 'Mobile app', jjj: false, github: true, linear: true, jira: true },
];

function Check() {
  return <span className="text-accent font-bold">✓</span>;
}

function Cross() {
  return <span className="text-text-secondary/50">✗</span>;
}

export function ComparisonTable() {
  return (
    <section className="py-24">
      <div className="container mx-auto px-6">
        <h2 className="text-3xl font-bold text-center text-text-primary mb-4">
          How jjj Compares
        </h2>
        <p className="text-center text-text-secondary mb-12">
          jjj vs hosted project management tools
        </p>

        <div className="overflow-x-auto">
          <table className="w-full max-w-4xl mx-auto">
            <thead>
              <tr className="border-b border-border">
                <th className="text-left py-4 px-4 font-semibold text-text-primary">Feature</th>
                <th className="text-center py-4 px-4 font-semibold text-accent">jjj</th>
                <th className="text-center py-4 px-4 font-semibold text-text-secondary">GitHub Issues</th>
                <th className="text-center py-4 px-4 font-semibold text-text-secondary">Linear</th>
                <th className="text-center py-4 px-4 font-semibold text-text-secondary">Jira</th>
              </tr>
            </thead>
            <tbody>
              {features.map((feature) => (
                <tr key={feature.name} className="border-b border-border/50 hover:bg-surface/50">
                  <td className="py-3 px-4 text-text-primary">{feature.name}</td>
                  <td className="py-3 px-4 text-center">{feature.jjj ? <Check /> : <Cross />}</td>
                  <td className="py-3 px-4 text-center">{feature.github ? <Check /> : <Cross />}</td>
                  <td className="py-3 px-4 text-center">{feature.linear ? <Check /> : <Cross />}</td>
                  <td className="py-3 px-4 text-center">{feature.jira ? <Check /> : <Cross />}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}
```

**Step 2: Commit**

```bash
git add docs-site/src/components/ComparisonTable.tsx
git commit -m "feat(docs): add ComparisonTable component"
```

---

## Task 12: Build Footer CTA Component

**Files:**
- Create: `docs-site/src/components/FooterCTA.tsx`

**Step 1: Create FooterCTA component**

Create `docs-site/src/components/FooterCTA.tsx`:

```tsx
import { Button } from './ui/button';

export function FooterCTA() {
  return (
    <section className="py-24 bg-gradient-to-br from-surface to-accent/5">
      <div className="container mx-auto px-6 text-center">
        <h2 className="text-3xl font-bold text-text-primary mb-4">
          Ready to start?
        </h2>
        <p className="text-text-secondary max-w-md mx-auto mb-8">
          Join the developers using jjj to manage projects with intellectual honesty.
        </p>
        <a href="/docs/getting-started/installation">
          <Button size="lg">Read the Docs</Button>
        </a>
      </div>
    </section>
  );
}
```

**Step 2: Commit**

```bash
git add docs-site/src/components/FooterCTA.tsx
git commit -m "feat(docs): add FooterCTA component"
```

---

## Task 13: Assemble Landing Page

**Files:**
- Create: `docs-site/src/pages/index.astro`
- Create: `docs-site/src/layouts/Landing.astro`

**Step 1: Create Landing layout**

Create `docs-site/src/layouts/Landing.astro`:

```astro
---
interface Props {
  title: string;
  description: string;
}

const { title, description } = Astro.props;
---

<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="description" content={description} />
    <title>{title}</title>
    <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet" />
    <style>
      :root {
        --background: #FDFBF7;
        --surface: #F7F5F0;
        --border: #E5E2DC;
        --text-primary: #2D2A26;
        --text-secondary: #6B6660;
        --accent: #D946EF;
        --accent-hover: #E879F9;
        --success: #6B9080;
      }

      @media (prefers-color-scheme: dark) {
        :root {
          --background: #1A1918;
          --surface: #252322;
          --border: #3D3A36;
          --text-primary: #F5F3EF;
          --text-secondary: #A8A49E;
        }
      }

      * {
        box-sizing: border-box;
        margin: 0;
        padding: 0;
      }

      body {
        font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        background-color: var(--background);
        color: var(--text-primary);
        -webkit-font-smoothing: antialiased;
      }
    </style>
  </head>
  <body>
    <slot />
  </body>
</html>
```

**Step 2: Create landing page**

Create `docs-site/src/pages/index.astro`:

```astro
---
import Landing from '../layouts/Landing.astro';
import { Hero } from '../components/Hero';
import { Features } from '../components/Features';
import { HowItWorks } from '../components/HowItWorks';
import { QuickInstall } from '../components/QuickInstall';
import { ComparisonTable } from '../components/ComparisonTable';
import { FooterCTA } from '../components/FooterCTA';
---

<Landing
  title="jjj - Distributed Project Management for Jujutsu"
  description="Problems, solutions, and critiques — all in your repo. No server. No database. Works offline."
>
  <Hero client:load />
  <Features client:visible />
  <HowItWorks client:visible />
  <QuickInstall client:visible />
  <ComparisonTable client:visible />
  <FooterCTA client:visible />
</Landing>
```

**Step 3: Remove default Starlight index if present**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
rm -f src/content/docs/index.md src/content/docs/index.mdx
```

**Step 4: Verify landing page builds and renders**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
npm run build
```

Expected: Build completes without errors

**Step 5: Commit**

```bash
git add docs-site/src/layouts/ docs-site/src/pages/
git commit -m "feat(docs): assemble landing page with all components"
```

---

## Task 14: Add Dark Mode Toggle to Landing Page

**Files:**
- Modify: `docs-site/src/layouts/Landing.astro`
- Create: `docs-site/src/components/ThemeToggle.tsx`

**Step 1: Create ThemeToggle component**

Create `docs-site/src/components/ThemeToggle.tsx`:

```tsx
import { useState, useEffect } from 'react';

export function ThemeToggle() {
  const [theme, setTheme] = useState<'light' | 'dark'>('light');

  useEffect(() => {
    const stored = localStorage.getItem('theme');
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const initial = stored || (prefersDark ? 'dark' : 'light');
    setTheme(initial as 'light' | 'dark');
    document.documentElement.setAttribute('data-theme', initial);
  }, []);

  const toggle = () => {
    const next = theme === 'light' ? 'dark' : 'light';
    setTheme(next);
    localStorage.setItem('theme', next);
    document.documentElement.setAttribute('data-theme', next);
  };

  return (
    <button
      onClick={toggle}
      className="p-2 rounded-lg hover:bg-surface transition-colors"
      aria-label="Toggle theme"
    >
      {theme === 'light' ? '🌙' : '☀️'}
    </button>
  );
}
```

**Step 2: Add navigation header to Landing layout**

Update `docs-site/src/layouts/Landing.astro` to include a header:

Add after `<body>` tag:

```astro
<header class="fixed top-0 left-0 right-0 z-50 bg-background/80 backdrop-blur-sm border-b border-border">
  <div class="container mx-auto px-6 h-16 flex items-center justify-between">
    <a href="/" class="flex items-center gap-2 font-bold text-text-primary">
      <span class="w-8 h-8 rounded-lg bg-accent text-white flex items-center justify-center text-sm">jjj</span>
      <span>jjj</span>
    </a>
    <nav class="flex items-center gap-4">
      <a href="/docs/getting-started/installation" class="text-text-secondary hover:text-accent transition-colors">
        Docs
      </a>
      <a href="https://github.com/doug/jjj" class="text-text-secondary hover:text-accent transition-colors" target="_blank">
        GitHub
      </a>
      <div id="theme-toggle"></div>
    </nav>
  </div>
</header>
<div class="pt-16">
  <slot />
</div>
```

And add ThemeToggle script at end of body:

```astro
<script>
  import { ThemeToggle } from '../components/ThemeToggle';
  import { createRoot } from 'react-dom/client';

  const container = document.getElementById('theme-toggle');
  if (container) {
    const root = createRoot(container);
    root.render(<ThemeToggle />);
  }
</script>
```

**Step 3: Update CSS for dark mode**

Add to the `<style>` section in Landing.astro:

```css
[data-theme='dark'] {
  --background: #1A1918;
  --surface: #252322;
  --border: #3D3A36;
  --text-primary: #F5F3EF;
  --text-secondary: #A8A49E;
}
```

**Step 4: Commit**

```bash
git add docs-site/src/components/ThemeToggle.tsx docs-site/src/layouts/Landing.astro
git commit -m "feat(docs): add dark mode toggle to landing page"
```

---

## Task 15: Clean Up Old mdbook Files

**Files:**
- Remove: `docs/theme/` directory
- Remove: `book.toml`
- Keep: `docs/` markdown files (for reference/backup)

**Step 1: Remove mdbook-specific files**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign
rm -rf docs/theme
rm -f book.toml
rm -f docs/index.md  # Old landing page with HTML
rm -f docs/SUMMARY.md  # mdbook navigation
rm -f docs/README.md  # Often duplicate of index
```

**Step 2: Update CLAUDE.md documentation commands**

Update the Documentation section in `CLAUDE.md`:

```bash
### Documentation
```bash
cd docs-site && npm run dev    # Serve locally
cd docs-site && npm run build  # Build docs
```
```

**Step 3: Commit**

```bash
git add -A
git commit -m "chore(docs): remove mdbook files, update CLAUDE.md"
```

---

## Task 16: Final Testing and Polish

**Step 1: Run full build**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
npm run build
```

Expected: Build completes without errors

**Step 2: Preview the site**

```bash
cd /Users/dougfritz/src/jjj/.worktrees/starlight-redesign/docs-site
npm run preview
```

Manually verify:
- Landing page loads with all sections
- Dark/light mode toggle works
- Navigation to docs works
- Docs pages render correctly
- Search works
- Mobile responsive

**Step 3: Fix any issues found**

Address any visual or functional issues.

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat(docs): complete Starlight documentation site"
```

---

## Summary

After completing all tasks:

1. New `docs-site/` directory with Astro + Starlight
2. Landing page with Hero, Features, HowItWorks, QuickInstall, ComparisonTable, FooterCTA
3. All documentation migrated with proper frontmatter
4. Custom warm theme with fuchsia accent
5. Light/dark mode support
6. shadcn/ui components (Button, Card)
7. Old mdbook files removed

To serve locally:
```bash
cd docs-site && npm run dev
```

To build for production:
```bash
cd docs-site && npm run build
```
