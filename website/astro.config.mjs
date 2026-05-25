// @ts-check
import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import sitemap from '@astrojs/sitemap';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import react from '@astrojs/react';
import starlightClientMermaid from '@pasqal-io/starlight-client-mermaid';
import { remarkReadingTime } from './src/lib/reading-time.mjs';

export default defineConfig({
  site: 'https://forgeplan.dev',
  integrations: [
    // Sitemap before starlight(): generates sitemap-index.xml + sitemap-0.xml.
    // Filter /docs/* because Starlight emits its own sitemap entries for those
    // routes and we don't want duplicates in the index.
    sitemap({ filter: (page) => !page.includes('/docs/') }),
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
    head: [
      // 1. AI Overview snippet directive (March 2026 Google update)
      {
        tag: 'meta',
        attrs: {
          name: 'robots',
          content: 'max-snippet:-1, max-image-preview:large, max-video-preview:-1',
        },
      },
      // 2. Default og:image / twitter:image — applies to all docs pages
      {
        tag: 'meta',
        attrs: {
          property: 'og:image',
          content: 'https://forgeplan.dev/og-default.png',
        },
      },
      {
        tag: 'meta',
        attrs: {
          property: 'og:image:width',
          content: '1200',
        },
      },
      {
        tag: 'meta',
        attrs: {
          property: 'og:image:height',
          content: '630',
        },
      },
      {
        tag: 'meta',
        attrs: {
          property: 'og:image:alt',
          content: 'Forgeplan — engineering methodology for decisions that last',
        },
      },
      {
        tag: 'meta',
        attrs: {
          name: 'twitter:image',
          content: 'https://forgeplan.dev/og-default.png',
        },
      },
      {
        tag: 'meta',
        attrs: {
          property: 'og:site_name',
          content: 'Forgeplan',
        },
      },
      // 3. hreflang x-default — point to EN docs root
      {
        tag: 'link',
        attrs: {
          rel: 'alternate',
          hreflang: 'x-default',
          href: 'https://forgeplan.dev/docs/',
        },
      },
      // 4. GSC verification placeholder — replace TODO with code from Google Search Console
      {
        tag: 'meta',
        attrs: {
          name: 'google-site-verification',
          content: 'TODO_PASTE_GSC_VERIFICATION_CODE_HERE',
        },
      },
    ],
    components: {
      // Unified Header on /docs via DocsHeader (inline content; no nested <header>).
      // Prior attempt rendered our standalone <header position:fixed> here,
      // which Starlight wrapped inside its own <header.header> → invalid HTML
      // + broke grid (article 19px, sidebar 0). DocsHeader renders ONLY the
      // brand+nav+toggle row → Starlight's outer <header> remains the single
      // header element on the page.
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
