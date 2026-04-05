// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import react from '@astrojs/react';

export default defineConfig({
  integrations: [starlight({
    title: 'Forgeplan',
    logo: {
      dark: './src/assets/logo-dark.svg',
      light: './src/assets/logo-light.svg',
      replacesTitle: false,
    },
    social: [
      { icon: 'github', label: 'GitHub', href: 'https://github.com/ForgePlan/forgeplan' },
    ],
    customCss: ['./src/styles/forge-theme.css'],
    sidebar: [
      {
        label: 'Getting Started',
        items: [
          { label: 'Installation', slug: 'getting-started/installation' },
          { label: 'Quick Start', slug: 'getting-started/quick-start' },
          { label: 'Configuration', slug: 'getting-started/configuration' },
        ],
      },
      {
        label: 'Methodology',
        items: [
          { label: 'Overview', slug: 'methodology/overview' },
          { label: 'Routing & Depth', slug: 'methodology/routing' },
          { label: 'Artifact Lifecycle', slug: 'methodology/lifecycle' },
          { label: 'Evidence & Scoring', slug: 'methodology/evidence' },
          { label: 'ADI Reasoning', slug: 'methodology/adi' },
        ],
      },
      {
        label: 'CLI Reference',
        autogenerate: { directory: 'cli' },
      },
      {
        label: 'MCP Reference',
        autogenerate: { directory: 'mcp' },
      },
      {
        label: 'Marketplace',
        autogenerate: { directory: 'marketplace' },
      },
      {
        label: 'Guides',
        autogenerate: { directory: 'guides' },
      },
      {
        label: 'Reference',
        autogenerate: { directory: 'reference' },
      },
    ],
  }), react()],
  vite: {
    plugins: [tailwindcss()],
  },
});
