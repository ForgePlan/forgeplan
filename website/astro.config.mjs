// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import react from '@astrojs/react';

export default defineConfig({
  site: 'https://forgeplan.dev',
  integrations: [starlight({
    title: {
      en: 'Forgeplan',
      ru: 'Forgeplan',
    },
    defaultLocale: 'root',
    locales: {
      root: { label: 'English', lang: 'en' },
      ru: { label: 'Русский', lang: 'ru' },
    },
    favicon: '/favicon.svg',
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
  }), react()],
  vite: {
    plugins: [tailwindcss()],
  },
});
