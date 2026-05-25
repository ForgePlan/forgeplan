---
depth: standard
id: RFC-011
kind: rfc
last_modified_at: 2026-05-22T23:26:47.711779+00:00
last_modified_by: claude-code/2.1.149
links:
- target: PRD-079
  relation: based_on
status: active
title: 'Blog scaffold architecture: Content Collection + mirror routes + scoped theme'
---

# RFC-011: Blog scaffold architecture — Content Collection + mirror routes + scoped theme

## Summary

Реализуем блог как **третий mode** в существующем Astro 6 + Starlight 0.38 + React 19 приложении `ForgePlan/website`. Используем стандартный Astro Content Collection с zod-схемой, MDX integration для inline-компонентов, mirror routes pattern (как у Starlight docs: `/blog` EN + `/ru/blog` RU), scoped CSS tokens под `.blog-post` selector — palette портируется из teaching-asset, но шрифтовой стек остаётся существующим сайтовым (Space Grotesk + Geist Mono). PR-1 = только инфраструктура + placeholder; контент идёт в PR-2.

Подход согласован с ADI на PRD-079 (gemini-3.1-pro-preview, 3 hypotheses High/Medium/High confidence). Open Questions resolved: sitemap deferred (не установлен сейчас, не блокер); RU copy = «Блог»; placeholder draft=false с минимальным content.

## Context

Реализация PRD-079: добавить блог-mode в существующее Astro приложение `ForgePlan/website`. Без отдельного репо, без отдельного деплоя, без нового шрифтового стека.

ADI (PRD-079, gemini-3.1-pro-preview) дал 3 hypotheses + risks, валидирует архитектуру ниже:
- **H1** Content Collections + zod schema — confidence High (нативный Astro)
- **H2** Scoped `.blog-post` CSS — confidence Medium (риск CSS bleed)
- **H3** `@astrojs/mdx` integration — confidence High (обычная задача), watch `client:*` дисциплину

## Options Considered

### Option 1: Three-mode in single Astro app (CHOSEN)

Mode A (Landing) + Mode B (Starlight docs) + **Mode C (blog, new)** живут в одном `ForgePlan/website` приложении, один deploy, один Header, общий шрифтовой стек.

- **Pros**: единый Header / шрифты / theme system / deploy / SSL / sitemap; reuse существующих React-компонентов; bilingual паттерн идентичен Starlight (`/blog` EN + `/ru/blog` RU).
- **Cons**: рост `astro.config.mjs` и `content.config.ts`; нужна аккуратность с порядком integrations (mdx ДО starlight).
- **When chosen**: explicitly per PRD-079 Non-Goals и user confirmation в чате.

### Option 2: Separate subdomain `blog.forgeplan.dev`

Отдельный Astro app, отдельный repo, отдельный deploy/SSL/sitemap.

- **Pros**: zero risk регрессии docs/landing; полная свобода стека (мог быть Next.js).
- **Cons**: дублирование Header/themes/i18n; отдельный CI; нужен cross-domain link discipline; пользовательский experience хуже (separate domain feels disconnected).
- **Rejected because**: PRD-079 Non-Goals явно отвергает.

### Option 3: Starlight as blog frontend

Использовать Starlight UI для блога (отдельная top-level entry в `astro.config.mjs` sidebar).

- **Pros**: zero new infrastructure, Starlight i18n built-in, sidebar nav бесплатно.
- **Cons**: Starlight навязывает docs-style layout (sidebar, prev/next), blog нужна другая визуальная семантика (hero, meta-time-author, RSS subscribe surface). Tight coupling блога с docs structure.
- **Rejected because**: блог имеет фундаментально другую UX-семантику (chronological, narrative) против docs (taxonomic, reference).

### Option 4: Astro top-level i18n middleware

Включить top-level `i18n: { locales: ['en','ru'], defaultLocale: 'en' }` в `astro.config.mjs` и использовать Astro routing magic вместо mirror routes.

- **Pros**: меньше дублирования файлов (один `[lang]/blog/[slug].astro`).
- **Cons**: Starlight уже использует mirror routes; добавление top-level i18n может изменить URL behavior существующих docs (тестово или silently). Risk to NFR-003 (no regression).
- **Rejected because**: pattern divergence с Starlight; zero-risk integration более ценен чем DRY.

### Option 5: Tailwind 4 `@theme` для blog tokens

Расширить Tailwind 4 theme через `@theme` директиву, использовать utility classes вместо raw CSS.

- **Pros**: единый стиль с landing (которая использует Tailwind); меньше CSS файлов.
- **Cons**: палитра из teaching-asset не маппится на дефолтные Tailwind tokens; **глобальное** extending = риск побочек на landing/docs (H2 ADI risk усиливается); custom hexes (`--f-color/--g-color/--r-color`) становятся global concerns.
- **Rejected for PR-1; reconsider in PR-2**: scoped raw CSS чище для PR-1 baseline; в PR-2 при появлении компонентов рассмотрим перенос в Tailwind `@theme` с blog-prefix.

## Architecture

### Three-mode topology in single Astro app

```
ForgePlan/website/ (single Astro 6 app, single deploy)
│
├── Mode A: Landing                         (existing — unchanged)
│   ├── src/pages/index.astro
│   ├── src/layouts/Landing.astro
│   └── src/components/{Hero,Trust,Pipeline,Artifacts,AI,Graph,Install}.astro/.tsx
│
├── Mode B: Docs (Starlight)                (existing — unchanged)
│   ├── src/content/docs/**
│   ├── src/content/i18n/**
│   └── Starlight handles routing automatically
│
└── Mode C: Blog                            ← NEW (this RFC)
    ├── src/content/blog/{en,ru}/**.mdx     (content)
    ├── src/content.config.ts               (extend: add blog collection)
    ├── src/pages/blog/{index,[...slug]}.astro       (EN routes)
    ├── src/pages/ru/blog/{index,[...slug]}.astro    (RU routes)
    ├── src/pages/blog/rss.xml.ts
    ├── src/pages/ru/blog/rss.xml.ts
    ├── src/layouts/BlogPost.astro
    ├── src/components/blog/                (BlogPostMeta, LangSwitcher для blog)
    └── src/styles/blog-theme.css           (scoped tokens)
```

### Data flow

```
.mdx file in src/content/blog/{lang}/*
    ↓ astro:content schema (zod)            ← FR-001, NFR-004
    ↓ astro:content getCollection('blog')
    ↓ filter by frontmatter.lang
    ↓ filter by !frontmatter.draft
    ↓ sort by publishedAt desc
    ↓ render via BlogPost.astro layout      ← FR-006
    ↓ extended-markdown rendering           ← FR-002
    ↓ scoped .blog-post wrapper             ← FR-008, H2 mitigation
```

## Component design

### Content Collection schema (extends existing config.ts)

`src/content.config.ts` — current shape:

```ts
import { defineCollection } from 'astro:content';
import { docsLoader, i18nLoader } from '@astrojs/starlight/loaders';
import { docsSchema, i18nSchema } from '@astrojs/starlight/schema';

export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
  i18n: defineCollection({ loader: i18nLoader(), schema: i18nSchema() }),
};
```

Extension (new): add `blog` collection via Astro standard `glob` loader (Astro 5+ pattern):

```ts
import { defineCollection, z } from 'astro:content';
import { glob } from 'astro/loaders';

// ... existing docs/i18n unchanged ...

const blog = defineCollection({
  loader: glob({
    pattern: '**/*.{md,mdx}',
    base: './src/content/blog',
    // generateId required to prevent slug-collision when EN and RU posts
    // share the same `slug` frontmatter value (e.g. both have slug: "welcome").
    // Default Astro: frontmatter.slug becomes the entry ID → collision.
    // Fix: use file path → ids are {lang}/{slug}.
    // SIX call-sites depend on this format (see post-coder note below).
    generateId: ({ entry }) => entry.replace(/\.(md|mdx)$/, ''),
  }),
  schema: z.object({ /* ... see PRD-079 FR-001 for full schema ... */ }),
});

export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
  i18n: defineCollection({ loader: i18nLoader(), schema: i18nSchema() }),
  blog,                                       // ← NEW
};
```

**Implementation note (post-coder, 2026-05-23)**: `generateId` override is required for slug-collision avoidance. Six call-sites use `post.id.replace(/^(en|ru)\//, '').replace(/\.(md|mdx)$/, '')`:
- `src/pages/blog/index.astro`
- `src/pages/blog/[...slug].astro`
- `src/pages/blog/rss.xml.ts`
- `src/pages/ru/blog/index.astro`
- `src/pages/ru/blog/[...slug].astro`
- `src/pages/ru/blog/rss.xml.ts`

If `generateId` format changes (e.g. to support categories like `en/methodology/post`), ALL six call-sites must be updated in lockstep. Consider extracting `function postUrl(post): string` helper in PR-2 to centralise this contract.

**ADI deduction H1 risk mitigation**: docs collection использует Starlight loader, blog использует Astro standard glob loader — они не конфликтуют (разные namespaces, разные base dirs). NFR-003 (no regression) гарантирован.

### Astro config extension

`astro.config.mjs` — current uses:
- `starlight()` integration
- `react()` integration
- `tailwindcss()` vite plugin

```js
import mdx from '@astrojs/mdx';
// ... existing imports

export default defineConfig({
  site: 'https://forgeplan.dev',
  integrations: [
    starlight({ /* existing */ }),  // MUST be first — see post-coder note below
    mdx(),
    react(),
  ],
  vite: { plugins: [tailwindcss()] },
});
```

**Implementation constraint (post-coder, empirically verified 2026-05-23)**: This RFC originally specified `mdx()` BEFORE `starlight()` per generic Astro docs. That ordering produces a hard build error:

> [astro-expressive-code] Incorrect integration order: To allow code blocks on MDX pages to use astro-expressive-code, please move astroExpressiveCode() before mdx() in the "integrations" array of your Astro config file.

Root cause: Starlight bundles `astro-expressive-code` (ECE) as a sub-integration. ECE enforces `starlight-before-mdx` at the `astro:config:setup` hook. The correct order in this project is therefore `[starlight({...}), mdx(), react()]`. This constraint is specific to Starlight ≥ 0.29 (bundled ECE). Without Starlight, `mdx()-before-others` remains the general Astro recommendation.

Note: Starlight UI **не использует** наш blog mdx — наша `BlogPost.astro` лежит вне Starlight. Поэтому мы не наследуем Starlight rendering pipeline для блога — это **намеренно**, для свободы дизайна.

### Routes pattern

Mirror pattern по Starlight аналогии:

| URL | File | Filter |
|-----|------|--------|
| `/blog` | `src/pages/blog/index.astro` | `lang === 'en' && !draft` |
| `/blog/<slug>` | `src/pages/blog/[...slug].astro` | `lang === 'en' && slug === params.slug` |
| `/blog/rss.xml` | `src/pages/blog/rss.xml.ts` | `lang === 'en'` |
| `/ru/blog` | `src/pages/ru/blog/index.astro` | `lang === 'ru' && !draft` |
| `/ru/blog/<slug>` | `src/pages/ru/blog/[...slug].astro` | `lang === 'ru' && slug === params.slug` |
| `/ru/blog/rss.xml` | `src/pages/ru/blog/rss.xml.ts` | `lang === 'ru'` |

`[...slug]` (catch-all) выбран вместо `[slug]` чтобы потом можно было организовать посты в категории (`/blog/methodology/r-eff-deep-dive`) без миграции routes.

### BlogPost layout

`src/layouts/BlogPost.astro` — обёртка для индивидуального поста.

Ключевые элементы:
1. Importtable `<Header />` — переиспользуем existing landing/docs header (только добавив Blog active state — см. FR-007).
2. `<html lang={frontmatter.lang}>` — для screen reader + Google language detection.
3. Wrapping `<article class="blog-post">` — scope для `blog-theme.css` (FR-008, H2 mitigation).
4. `<time datetime={frontmatter.publishedAt.toISOString()}>` — semantic time element (NFR-006).
5. Slot for content (mdx renderable).
6. Footer с LangSwitcher (FR-006): если `frontmatter.translations[opposite_lang]` exists → ссылка на counterpart, иначе grayed-out.
7. **Fonts via `@fontsource` self-hosted imports** (NOT Google Fonts CDN) — `@fontsource/space-grotesk` + `@fontsource/geist-mono` уже declared в `package.json`. Avoids external CDN dependency / GDPR concern / FOUT.

### Landing.astro shared between modes (post-coder note)

Existing `src/layouts/Landing.astro` теперь имеет `lang?: 'en' | 'ru'` prop (default `'en'`) и используется не только корневым лендингом, но и blog index pages (EN + RU). При его дальнейшем эволюционировании учитывать, что он stал site-level layout для трёх mode (landing + blog EN index + blog RU index). Рассмотреть rename в `SiteLayout.astro` или extract `BlogIndex.astro` в PR-2.

### Header.astro modification

Current `src/components/Header.astro` — модифицируется одной точкой:

```astro
<!-- existing nav -->
<a href="/docs">Docs</a>
<a href="/blog" class={isBlogActive ? 'active' : ''}>Blog</a>   <!-- ← NEW -->
<a href="https://github.com/ForgePlan/forgeplan">GitHub</a>
```

`isBlogActive` определяется через `Astro.url.pathname.startsWith('/blog')` или `startsWith('/ru/blog')`. Минимальная мутация existing component, no breaking change для landing/docs.

Russian copy: `Blog` (EN) / `Блог` (RU) — Open Question PRD-079 #2: я выбираю «Блог», не «Журнал» (короче, прямой equivalent, не нагружает Header).

### blog-theme.css — token scoping

Файл `src/styles/blog-theme.css` импортируется ТОЛЬКО `BlogPost.astro` и индекс-страницами блога (не глобально). H2 risk mitigation.

Структура: все tokens из `trust-calculus.html` (см. полный список ниже), завёрнуты под `.blog-post` selector. Шрифтовой стек — **существующий** Space Grotesk + Geist Mono.

Дополнительный scope `.blog-index` — для index pages (`/blog`, `/ru/blog`), которые используют `Landing.astro` (вне `.blog-post`). Tokens идентичны `.blog-post`, отдельные классы: `.blog-index-card`, `.blog-index-meta`, `.blog-index-rss` и т.д. (post-coder addition).

```css
.blog-post, .blog-index {
  --bg: #050505; --bg-1: #0b0b0b; --bg-2: #141414; --bg-3: #1a1a1a;
  --text: #f5f5f5; --text-1: #e5e5e5; --text-2: #a3a3a3; --text-3: #737373; --text-4: #525252;
  --line: rgba(255, 255, 255, 0.06); --line-2: rgba(255, 255, 255, 0.14);
  --accent: #ff5a1f; --accent-soft: #ff8a5b; --accent-bg: rgba(255, 90, 31, 0.18);
  --ok: #22c55e; --err: #ef4444; --warn: #f59e0b;
  --f-color: #ff5a1f; --g-color: #22c55e; --r-color: #60a5fa;
  --font-sans: 'Space Grotesk', ui-sans-serif, system-ui, -apple-system, sans-serif;
  --font-mono: 'Geist Mono', ui-monospace, SFMono-Regular, Menlo, monospace;
}
/* ... base typography rules under .blog-post, index card rules under .blog-index ... */
```

### Placeholder content (FR-009)

Underscore-prefix files в Astro Content Collection **исключаются** из getCollection (это для дефолтных templates). Поэтому используем `welcome.mdx` (без underscore), `draft: false`, kind: explainer, минимальный content (3 предложения «here will be posts»). Removed/replaced в PR-2.

Open Question PRD-079 #3 — resolved: draft=false, чтобы index не был пустым.

## Trade-offs considered

### Sitemap (PRD-079 NFR-005, Open Question #1)

ADI evidence E2 проверил — `@astrojs/sitemap` **уже установлен** через Starlight dependency tree, и блог routes **автоматически попадают** в `dist/sitemap-0.xml` с правильным `hreflang` cross-linking EN↔RU. Surprise win — отдельный follow-up PR не нужен (post-coder discovery 2026-05-23).

### Why glob loader, not docsLoader

Starlight loader **навязывает Starlight UI** (sidebar, prev/next nav). Blog нужна **полная свобода layout** для будущих interactive components. Стандартный `glob` loader = чистый DX, нет дополнительной зависимости.

### Why mirror routes, not Astro i18n middleware

Уже объяснено выше в Options. Дополнительный аргумент: при добавлении 3-го языка достаточно скопировать `src/pages/{lang}/blog/`, не трогая config.

### Why scoped CSS, not Tailwind

См. Option 5 выше. Резюме: scoped raw CSS = чище для PR-1 baseline.

### Why @fontsource, not Google Fonts CDN

Privacy (GDPR), reliability (no external CDN), bundle predictability. `@fontsource/*` packages уже declared в `package.json` (Astro best practice). Initial implementation использовала Google Fonts `<link>` tags — это caught в Step 6.5 audit и fixed (post-coder 2026-05-23).

## Invariants — что НЕ должно сломаться этим RFC

1. **Existing landing (`/`) рендерит идентично** до и после merge — pixel-perfect (визуально), HTML byte-diff ограничен только добавлением `<a href="/blog">Blog</a>` в Header и `lang` prop в Landing (default 'en' = no behavior change для root).
2. **Existing docs routes** (`/docs/*`, `/ru/docs/*`) — все resolve 200 после merge. Starlight sidebar, search, mermaid — продолжают работать.
3. **Шрифтовой стек НЕ меняется** — Space Grotesk + Geist Mono (via @fontsource self-hosted). Inter / JetBrains Mono НЕ вводятся. Google Fonts CDN НЕ используется.
4. **Tokens scoped** — никакой `--accent: #ff5a1f` на `:root`. Только под `.blog-post` или `.blog-index` selector.
5. **0 JS by default** — blog index/post pages ship 0 client-side JS, RSS endpoints — server-only.
6. **Build определённо проходит** — `npm run build` в worktree exits 0 БЕЗ degradation > 10% от baseline.

## Rollback Plan

Если PR-1 после merge вызывает регрессию:

1. **First option — revert merge commit на dev**:
   ```bash
   git checkout dev
   git pull
   git revert -m 1 <merge-commit-sha>
   git push origin dev
   ```
   Revert восстанавливает state до feat/blog-scaffold merge. Existing landing/docs не затронуты.

2. **Partial rollback — выключить только blog routes**:
   - Move `src/pages/blog/` и `src/pages/ru/blog/` в `_disabled/`.
   - Comment out `mdx()` и `blog` collection в configs.
   - `npm run build` снова проходит без блог-content.
   - Time-to-rollback: ~10 минут.

3. **Если revert невозможен (downstream commits)** — issue fix commit:
   - Удалить только `src/components/Header.astro` Blog link.
   - Удалить `src/styles/blog-theme.css` import из `BlogPost.astro`.
   - Оставить content collection / routes / RSS — они не активны без Header link.

Worst case downtime: 0 (rollback не требует обнуления deploy, Astro static build идёт ~30-60s в CI).

## Phases

### Phase A — this PR (PR-1, completed)

1. `npm install @astrojs/mdx @astrojs/rss` в `website/`.
2. Update `astro.config.mjs` — add `mdx()` integration AFTER `starlight()` (per ECE constraint).
3. Update `src/content.config.ts` — add `blog` collection with zod schema + `generateId` override.
4. Create `src/content/blog/{en,ru}/welcome.mdx` placeholder per locale.
5. Create `src/layouts/BlogPost.astro` (with @fontsource imports + html lang prop).
6. Modify `src/layouts/Landing.astro` — add `lang` prop with default 'en'.
7. Create `src/styles/blog-theme.css` (with `.blog-post` + `.blog-index` scopes).
8. Create `src/pages/blog/{index,[...slug]}.astro` + `rss.xml.ts`.
9. Create `src/pages/ru/blog/{index,[...slug]}.astro` + `rss.xml.ts` (RU index passes `lang="ru"` to Landing).
10. Modify `src/components/Header.astro` — Blog link + isBlogActive.
11. `npm run build` в worktree — exits 0 (verified 15.38s, 346 pages).

### Phase B — PR-2 (depends on Phase A merge)

- 6 MDX-компонентов (ArtifactCard, EvidenceTable, Pipeline, Hypotheses, WinnerCard, FGRChart).
- Pilot контент — **«Cycles tetralogy»** (4 поста, на оба языка): trust-calculus, decision-cycle, bmad-cycle, spec-cycle — все из `ForgePlanMarketing/teaching-assets/*.html` (4 файла, total ~178 KB).
- Delete welcome.mdx placeholder.
- Possibly: extract `postUrl(post): string` helper (centralise 6-callsite regex).
- Possibly: extract `BlogIndex.astro` layout (decouple from Landing.astro).

### Phase C — follow-up (отдельные PR, not blocking)

- OG image generator.
- Mermaid в blog (если PR-2 покажет необходимость).
- A11y: skip-to-content link in BlogPost.astro (PR-2 audit found this missing).

## Risks & mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| MDX integration breaks Starlight docs build | HIGH | mdx() добавляется ПОСЛЕ starlight() в integrations array (ECE constraint — see Astro config section). Empirically verified with full `npm run build`. |
| CSS bleed: blog-theme overrides landing/docs | MEDIUM (H2 ADI) | Все tokens scoped в `.blog-post` или `.blog-index` selector. Import только в blog layouts/pages, не глобально. Audit Step 6.5 confirmed 0 `:root` declarations, 0 inline hex in .astro pages. |
| Tailwind 4 + scoped CSS specificity | MEDIUM | Tailwind generates utility classes которые могут переопределить наши tokens. Mitigation: `.blog-post`/`.blog-index` wrapper увеличивает specificity. Visual regression deferred to PR-2 dogfood. |
| 0 JS NFR violated через `client:load` directive | LOW (H3 ADI) | Authoring guideline через RFC + audit. Не блокер PR-1 (нет компонентов в PR-1). PR-2 author guidance: use `client:visible` only. |
| Zod schema mismatch with placeholder frontmatter | LOW | Placeholder написан после schema, проверен `npm run build`. |
| Route collision /blog vs Starlight | NONE | Starlight использует только `/docs` + `/ru/docs`. `/blog` свободен. Проверено через `astro.config.mjs` Starlight `sidebar:` entries. |
| 6-callsite regex coupling for slug-resolution | MEDIUM | Comment in `content.config.ts` documents all 6 call-sites. PR-2 extracts helper. |

## Acceptance — RFC-011

- [x] Phase A 11 шагов выполнены в worktree `/Users/explosovebit/Work/forgeplan-blog-scaffold/website/`.
- [x] `npm run build` в worktree exits 0 (15.38s, 346 pages).
- [x] No regression: existing routes `/`, `/docs`, `/ru/docs` resolve 200 (dist artifacts verified).
- [x] All tokens из `trust-calculus.html` portированы в `blog-theme.css` под `.blog-post` + `.blog-index` scope.
- [x] Linked based_on PRD-079.
- [x] All 6 Invariants соблюдены (audit in Step 6.5, fix-coder applied 5 HIGH findings).

## Related Artifacts

- based_on PRD-079
- references: `ForgePlanMarketing/teaching-assets/{trust-calculus,decision-cycle,bmad-cycle,spec-cycle}.html` (token source + future PR-2 content tetralogy)
- references: existing `src/layouts/Landing.astro` (font/theme pattern, now shared with blog index)
- references: existing `src/components/Header.astro` (modification target)
- informs EVID-136 (code-reviewer audit verdict CONCERNS — all HIGH findings fixed post-audit)
- future: PRD-080 (PR-2 — Cycles tetralogy pilot + 6 components, depends on this RFC)




