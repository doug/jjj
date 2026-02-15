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
    customCss: [
      './src/styles/global.css',
      './src/styles/custom.css',
    ],
    sidebar: [
      {
        label: 'Getting Started',
        items: [
          { label: 'Installation', slug: 'getting-started/installation' },
          { label: 'Quick Start', slug: 'getting-started/quick-start' },
          { label: 'Concepts & Terminology', slug: 'getting-started/concepts' },
        ],
      },
      {
        label: 'User Guides',
        items: [
          { label: 'Philosophy in Practice', slug: 'guides/philosophy-in-practice' },
          { label: 'Problem Solving', slug: 'guides/problem-solving' },
          { label: 'Cookbook (Recipes)', slug: 'guides/cookbook' },
          { label: 'Critique Guidelines', slug: 'guides/critique-guidelines' },
          { label: 'Code Review Workflow', slug: 'guides/code-review' },
          { label: 'Review User Journey', slug: 'guides/review-user-journey' },
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
          { label: 'Search Commands', slug: 'reference/cli-search' },
          { label: 'Database Commands', slug: 'reference/cli-db' },
          { label: 'Configuration', slug: 'reference/configuration' },
        ],
      },
      {
        label: 'Architecture',
        items: [
          { label: 'Design Philosophy', slug: 'architecture/design-philosophy' },
          { label: 'Storage & Metadata', slug: 'architecture/storage' },
          { label: 'Change ID Tracking', slug: 'architecture/change-tracking' },
          { label: 'Event Lifecycle', slug: 'architecture/event-lifecycle' },
          { label: 'Consistency Model', slug: 'architecture/consistency' },
        ],
      },
      {
        label: 'Contributing',
        items: [
          { label: 'Testing', slug: 'contributing/testing' },
        ],
      },
    ],
  }), react()],

  vite: {
    plugins: [tailwindcss()],
  },
});
