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
          { label: 'Installation', slug: 'docs/getting-started/installation' },
          { label: 'Quick Start', slug: 'docs/getting-started/quick-start' },
          { label: 'Configuration', slug: 'docs/getting-started/configuration' },
        ],
      },
      {
        label: 'Methodology',
        items: [
          { label: 'Overview', slug: 'docs/methodology/overview' },
          { label: 'Routing & Depth', slug: 'docs/methodology/routing' },
          { label: 'Artifact Lifecycle', slug: 'docs/methodology/lifecycle' },
          { label: 'Evidence & Scoring', slug: 'docs/methodology/evidence' },
          { label: 'ADI Reasoning', slug: 'docs/methodology/adi' },
        ],
      },
      {
        label: 'CLI Reference',
        autogenerate: { directory: 'docs/cli' },
      },
      {
        label: 'MCP Reference',
        autogenerate: { directory: 'docs/mcp' },
      },
      {
        label: 'Marketplace',
        autogenerate: { directory: 'docs/marketplace' },
      },
      {
        label: 'Guides',
        autogenerate: { directory: 'docs/guides' },
      },
      {
        label: 'Reference',
        autogenerate: { directory: 'docs/reference' },
      },
    ],
  }), react()],
  vite: {
    plugins: [tailwindcss()],
  },
});
