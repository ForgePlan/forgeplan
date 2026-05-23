// @ts-check
import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import react from '@astrojs/react';
import starlightClientMermaid from '@pasqal-io/starlight-client-mermaid';
import { remarkReadingTime } from './src/lib/reading-time.mjs';

export default defineConfig({
  site: 'https://forgeplan.dev',
  integrations: [
    // ORDER: starlight() MUST come before mdx().
    // Starlight bundles astro-expressive-code (ECE) as a sub-integration.
    // ECE throws a hard error at astro:config:setup if it detects mdx() already
    // registered before it: "please move astroExpressiveCode() before mdx()".
    // RFC-011 §Astro config extension specifies mdx()-before-starlight per
    // generic Astro docs; however, the Starlight+ECE constraint overrides it.
    // Tested empirically: [mdx(), starlight(), react()] → ERROR from ECE.
    // See RFC-011 amendment proposal in EVID-136 coder findings for correction.
    starlight({
    plugins: [starlightClientMermaid()],
    title: {
      en: 'Forgeplan',
      ru: 'Forgeplan',
    },
    defaultLocale: 'root',
    locales: {
      root: { label: 'EN', lang: 'en' },
      ru: { label: 'RU', lang: 'ru' },
    },
    favicon: '/favicon.svg',
    credits: false,
    logo: {
      dark: './src/assets/logo-dark.svg',
      light: './src/assets/logo-light.svg',
      replacesTitle: false,
    },
    social: [
      { icon: 'github', label: 'GitHub', href: 'https://github.com/ForgePlan/forgeplan' },
    ],
    customCss: ['./src/styles/forge-theme.css'],
    components: {
      // Replace Starlight's default top header with our site Header.astro
      // so /docs/* shows the same nav (Docs · Blog · Guides · GitHub · CLI · MCP · Install)
      // with active route highlight, instead of Starlight's branded variant.
      Header: './src/components/StarlightHeaderWrapper.astro',
    },
    sidebar: [
      {
        label: 'Getting Started',
        translations: { ru: 'Начало работы' },
        items: [
          { label: 'Installation', slug: 'docs/getting-started/installation', translations: { ru: 'Установка' } },
          { label: 'Quick Start', slug: 'docs/getting-started/quick-start', translations: { ru: 'Быстрый старт' } },
          { label: 'Configuration', slug: 'docs/getting-started/configuration', translations: { ru: 'Настройка' } },
        ],
      },
      {
        label: 'Methodology',
        translations: { ru: 'Методология' },
        items: [
          { label: 'Overview', slug: 'docs/methodology/overview', translations: { ru: 'Обзор' } },
          { label: 'Routing & Depth', slug: 'docs/methodology/routing', translations: { ru: 'Роутинг и глубина' } },
          { label: 'Artifact Lifecycle', slug: 'docs/methodology/lifecycle', translations: { ru: 'Жизненный цикл' } },
          { label: 'Evidence & Scoring', slug: 'docs/methodology/evidence', translations: { ru: 'Доказательства и скоринг' } },
          { label: 'ADI Reasoning', slug: 'docs/methodology/adi', translations: { ru: 'ADI рассуждения' } },
          { label: 'Hint Contract', slug: 'docs/methodology/agent-protocol', translations: { ru: 'Hint Contract' } },
        ],
      },
      {
        label: 'Guides',
        translations: { ru: 'Руководства' },
        autogenerate: { directory: 'docs/guides' },
      },
      {
        label: 'Marketplace',
        translations: { ru: 'Маркетплейс' },
        autogenerate: { directory: 'docs/marketplace' },
      },
      {
        label: 'Reference',
        translations: { ru: 'Справочник' },
        autogenerate: { directory: 'docs/reference' },
      },
      {
        label: 'CLI Reference',
        translations: { ru: 'Справочник CLI' },
        collapsed: true,
        autogenerate: { directory: 'docs/cli' },
      },
      {
        label: 'MCP Reference',
        translations: { ru: 'Справочник MCP' },
        collapsed: true,
        autogenerate: { directory: 'docs/mcp' },
      },
      {
        label: 'Changelog',
        translations: { ru: 'История изменений' },
        slug: 'docs/changelog',
      },
    ],
  }),
    mdx(),
    react(),
  ],
  markdown: {
    remarkPlugins: [remarkReadingTime],
  },
  vite: {
    plugins: [tailwindcss()],
  },
});
