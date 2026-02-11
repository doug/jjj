// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

import react from '@astrojs/react';
import tailwindcss from '@tailwindcss/vite';

// https://astro.build/config
export default defineConfig({
  integrations: [starlight({
    title: 'jjj',
    logo: {
      src: './public/logo.svg',
      alt: 'jjj',
    },
    favicon: '/favicon.svg',
    description: 'Distributed project management for Jujutsu',
    social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/doug/jjj' }],
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
  }), react()],

  vite: {
    plugins: [tailwindcss()],
  },
});
