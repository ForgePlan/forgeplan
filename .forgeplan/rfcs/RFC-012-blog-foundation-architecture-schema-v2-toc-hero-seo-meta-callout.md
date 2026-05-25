---
depth: standard
id: RFC-012
kind: rfc
last_modified_at: 2026-05-23T00:14:10.057457+00:00
last_modified_by: claude-code/2.1.149
links:
- target: PRD-080
  relation: based_on
status: draft
title: 'Blog foundation architecture: schema v2 + TOC + hero + SEO meta + Callout'
---

# RFC-012: Blog foundation architecture — schema v2 + TOC + hero + SEO meta + Callout

## Summary

Реализуем 12 MUST-have primitive'ов из PRD-080 (FPF-determined scope из 22 кандидатов): series support, TOC, hybrid hero, card thumbnail, topic chips/colors, reading-time auto, author block, 4 SEO meta элементов (canonical, hreflang, og:image, JSON-LD), Callout component. Все — additions поверх PR-1 scaffold; placeholder `welcome.mdx` остаётся functional через backwards-compatible schema extension. Worker plan: 2 параллельных coder + 1 lead в отдельных worktrees.

## Motivation

PR-1 (PRD-079, RFC-011, merged commit 9d426e0) поставил scaffold с Content Collection + mirror routes + scoped theme. Это **минимально жизнеспособный** блог — но не готов для контент-launch'а: нет series-navigation для 5-cycles narrative arc, нет TOC для длинных постов, listing «лысый» без cover/thumbnail, bilingual SEO без canonical/hreflang рискует Google duplicate-content penalty.

PR-2A заполняет **12 primitive'ов** (PRD-080 FR-001..012) ДО заливания контента (PR-2B/3 = 5 циклов × 2 языка), иначе посты шипятся через черновой UX → rework-долг.

FPF Evaluate (Trust Calculus на 22 кандидата) определил эти 12 MUST с F+G+R ≥ 6, разделил на 3 группы: A (visual primitives), Б (discoverability), В (SEO baseline). Этот RFC = архитектура реализации.

## Options Considered

### Option 1: Один worker, последовательно

Один coder agent делает все 12 пунктов в одном worktree, последовательно.

- **Pros**: zero merge conflict risk, простой flow.
- **Cons**: ~3-4 часа single-threaded, дольше.
- **Rejected**: PR-1 показал, что parallel workers с строгим ownership работают; время — фактор.

### Option 2: 12 workers, по одному на feature

Каждый worker = 1 FR. 12 параллельных worktrees.

- **Pros**: максимальный параллелизм.
- **Cons**: overhead spawn × 12; FR-001 (schema) — shared dep всех других FRs, нельзя по-настоящему параллелить.
- **Rejected**: overhead не оправдан, schema dependency блокирует.

### Option 3: 2 workers + lead (CHOSEN)

- **Lead**: schema extension (FR-001) — выполняется первым, потому что все остальные FRs зависят от schema. После lead-commit на schema — spawn 2 workers параллельно.
- **Worker 1 «Visual primitives»**: FR-003 (series footer), FR-004 (TOC), FR-006 (hero), FR-007 (card thumbnail), FR-012 (Callout) — все «render-heavy», правят BlogPost.astro + new components.
- **Worker 2 «Discovery + SEO + author»**: FR-002 (series landing route), FR-005 (reading-time plugin), FR-008+FR-009 (topic chips + 6 colors), FR-010 (author block), FR-011 (4 SEO meta).
- Lead консолидирует, audits, ships.

**Why chosen**: clean ownership grid (по 5-6 FRs на worker), schema-first устраняет dependency, parallel в worktrees per CLAUDE.md «≥2 параллельных workers → separate worktrees». ETA ~2 часа end-to-end.

### Option 4: 3 workers + lead (3 группы FPF: А/Б/В)

Worker A = primitives (5 FRs), Worker B = discoverability (3 FRs), Worker C = SEO (4 FRs).

- **Pros**: 4-way parallel, ещё быстрее.
- **Cons**: SEO meta живёт в BlogPost.astro `<head>` — Worker C трогает тот же файл что Worker A (для hero/TOC). Файл-conflict вероятен.
- **Rejected**: 3-worker overhead не окупается; объединяем Б+В в Worker 2 чтобы избежать конфликта на BlogPost.astro.

## Architecture

### Schema v2 (Lead, FR-001)

`content.config.ts` — расширяется backwards-compatible:

```ts
const blog = defineCollection({
  loader: glob({
    pattern: '**/*.{md,mdx}',
    base: './src/content/blog',
    generateId: ({ entry }) => entry.replace(/\.(md|mdx)$/, ''),
  }),
  schema: z.object({
    // existing PR-1 fields
    title: z.string(),
    description: z.string(),
    slug: z.string(),
    lang: z.enum(['en', 'ru']),
    publishedAt: z.coerce.date(),
    updatedAt: z.coerce.date().optional(),
    kind: z.enum(['explainer', 'case-study', 'teaching', 'release-notes', 'deep-dive']),
    topic: z.enum(['r-eff', 'adi', 'fpf', 'mcp', 'methodology', 'release']),
    artifacts: z.array(z.string()).optional(),
    cover: z.string().optional(),
    draft: z.boolean().default(false),
    readingTime: z.number().optional(),
    translations: z.record(z.string(), z.string()).optional(),

    // NEW в PR-2A
    series: z.string().optional(),                     // FR-001
    seriesOrder: z.number().int().positive().optional(), // FR-001
    seriesDescription: z.string().optional(),          // только для seriesOrder=1
  }),
});
```

`cover` field уже был в PR-1 schema — теперь начинает рендериться.

### Worker 1 «Visual primitives» — owned files

```
src/layouts/BlogPost.astro                          (MODIFY — hero + TOC slot + author block placeholder)
src/components/blog/TOC.astro                       (NEW)
src/components/blog/Hero.astro                      (NEW — CSS hero, optionally consumes <img>)
src/components/blog/Callout.astro                   (NEW — 4 types)
src/components/blog/SeriesNav.astro                 (NEW — prev/next-in-series footer)
src/styles/blog-theme.css                           (APPEND — hero CSS + callout CSS)
```

Worker 1 does NOT touch:
- pages/blog/*, pages/ru/blog/* (Worker 2 owns)
- content.config.ts (Lead owns)
- astro.config.mjs (Worker 2 owns — remark plugin)
- Header.astro (untouched in PR-2A)

### Worker 2 «Discovery + SEO + author» — owned files

```
src/pages/blog/index.astro                          (MODIFY — topic chips + thumbnail rendering on cards)
src/pages/ru/blog/index.astro                       (MODIFY — same)
src/pages/blog/series/[name].astro                  (NEW — EN series landing)
src/pages/ru/blog/series/[name].astro               (NEW — RU series landing)
src/components/blog/TopicChip.astro                 (NEW — chip с topic color)
src/components/blog/AuthorBlock.astro               (NEW — static "Mike Kubal · elirum.me")
src/components/blog/SeoMeta.astro                   (NEW — canonical/hreflang/og:image/JSON-LD)
src/styles/blog-theme.css                           (APPEND — 6 topic colors + chip CSS + thumbnail CSS — coordinated с Worker 1 через separate sections)
astro.config.mjs                                    (MODIFY — add remark plugin для reading-time)
src/lib/reading-time.mjs                            (NEW — remark plugin implementation)
public/og-default.png                               (NEW — single default OG image)
```

Worker 2 does NOT touch:
- BlogPost.astro (Worker 1 owns) — но **integrates** через `<SeoMeta />` import (Worker 1 добавит slot/import после Worker 2 commit)
- content.config.ts (Lead owns)
- Existing pages/ru/blog/[...slug].astro and pages/blog/[...slug].astro (no changes needed)

### Integration coordination

Single shared file: `src/styles/blog-theme.css`. Workers 1 + 2 ОБА append к нему. Чтобы избежать merge conflict:
- Worker 1 appends в раздел `/* === PR-2A: visual primitives === */` (hero, callout)
- Worker 2 appends в раздел `/* === PR-2A: discovery === */` (topic chips, 6 colors, thumbnails)
- Lead в финале консолидирует если diff overlap detected.

BlogPost.astro: Worker 1 owns. После Worker 1 commit, lead-step додаёт `import SeoMeta from '../components/blog/SeoMeta.astro'` + `<SeoMeta {...metaProps} />` в `<head>`. Worker 2 при создании SeoMeta.astro обеспечивает чистый export.

### Phase order

```
PHASE 0 (Lead): schema extension + create worktree + verify PR-1 still builds
PHASE 1 (parallel): Worker 1 + Worker 2 spawn в отдельных sub-worktrees
PHASE 2 (Lead): merge workers' branches into feat/blog-foundation; integrate SeoMeta into BlogPost; audit; build verify
PHASE 3 (audit + evidence): 2 parallel reviewers, fix-coder if needed, EVID
PHASE 4 (commit): clean commits with Refs; STOP before push
```

### Component design

**TOC.astro** (Worker 1, FR-004):
- Props: `headings: { depth: number; slug: string; text: string }[]` (Astro's `getHeadings()` result)
- Renders: `<aside class="blog-toc">` с `<nav>` containing `<a href="#slug">text</a>` per h2/h3 (skip h1, h4+)
- Sticky positioning: `position: sticky; top: 80px; max-height: calc(100vh - 100px); overflow-y: auto;`
- Mobile: `@media (max-width: 1023px)` → collapses в `<details>` block above content
- A11y: `<nav aria-label="Table of contents">`; focused link gets accent border

**Hero.astro** (Worker 1, FR-006):
- Props: `cover?: string`, `topic: string`, `kind: string`, `title: string`
- If `cover` set → `<img src={cover} alt={title} class="blog-hero-img">`
- Else → CSS hero block: dotted bg + topic-colored accent stripe + topic eyebrow + h1
- Title h1 ВНУТРИ Hero (вытаскивается из BlogPost) — снимает duplicate `<h1>`

**Callout.astro** (Worker 1, FR-012):
- Props: `type: 'info' | 'warn' | 'tip' | 'danger'`
- Border-left 3px solid в variant color; icon (Unicode emoji или inline SVG) + slot content
- 4 type→color: info=`--r-color` (blue), warn=`--warn` (amber), tip=`--g-color` (green), danger=`--err` (red)
- A11y: `<aside role="note" aria-label={type}>`

**SeriesNav.astro** (Worker 1, FR-003):
- Props: `currentSlug: string`, `currentSeries: string`, `lang: 'en'|'ru'`
- Queries `getCollection('blog')` filtered by series + lang + sorted by seriesOrder
- Renders prev/next links если exist в серии, иначе скрывается
- Layout: 2-column flex, prev на left, next на right

**TopicChip.astro** (Worker 2, FR-008):
- Props: `topic: 'r-eff' | 'adi' | 'fpf' | 'mcp' | 'methodology' | 'release'`
- Renders: `<span class={`topic-chip topic-${topic}`}>{topic}</span>`
- CSS: padding 2px 8px, border-radius 3px, font-mono 10.5px uppercase letter-spacing 0.14em
- Color = `var(--t-${topic})`; background = `color-mix(in srgb, var(--t-${topic}) 12%, transparent)`

**AuthorBlock.astro** (Worker 2, FR-010):
- Static, no props (single author)
- Renders compact footer: «Author» eyebrow + «Mike Kubal» name + `<a href="https://elirum.me" rel="external noopener" target="_blank">elirum.me</a>`
- Style: small, accent-soft color на name

**SeoMeta.astro** (Worker 2, FR-011):
- Props: `title`, `description`, `lang`, `cover?`, `publishedAt`, `updatedAt?`, `translations?`, `Astro.url`
- Renders в `<head>`:
  - `<link rel="canonical" href={Astro.url.href}>`
  - `<link rel="alternate" hreflang="en" href={...}>` + `<link rel="alternate" hreflang="ru" href={...}>` (если есть translations)
  - `<link rel="alternate" hreflang="x-default" href={...}>`
  - `<meta property="og:title" content={title}>`
  - `<meta property="og:description" content={description}>`
  - `<meta property="og:image" content={cover ?? '/og-default.png'}>`
  - `<meta property="og:type" content="article">`
  - `<meta property="article:published_time" content={publishedAt.toISOString()}>`
  - `<meta name="twitter:card" content="summary_large_image">`
  - `<meta name="twitter:creator" content="@elirum">` (TBD: подтвердить handle, иначе fallback на site-level)
  - `<script type="application/ld+json">` с BlogPosting JSON-LD (headline, datePublished, dateModified, author.name=Mike Kubal, author.url=elirum.me, image, inLanguage)

**Reading-time plugin** (Worker 2, FR-005):
- Astro 6 path: remark plugin in `astro.config.mjs` markdown.remarkPlugins
- Plugin реализация в `src/lib/reading-time.mjs`: traverse MDX AST, extract text nodes, count chars, divide by 200wpm × language-aware factor (RU ~1.5× slower for non-native speakers, but для simplicity skip — use 200wpm для обоих)
- Result available как `frontmatter.readingTime` (auto-set if not provided)

### Worktree topology

```
PHASE 0 worktree:
  /Users/explosovebit/Work/forgeplan-blog-foundation/    feat/blog-foundation (off origin/dev, after PR-1 merge)

PHASE 1 sub-worktrees:
  /Users/explosovebit/Work/forgeplan-blog-w1-visual/     feat/blog-foundation-w1-visual (off feat/blog-foundation after schema commit)
  /Users/explosovebit/Work/forgeplan-blog-w2-discovery/  feat/blog-foundation-w2-discovery (same base)
```

После Workers commit и lead merge — оба sub-worktrees cleanup, остаётся только foundation worktree.

## Invariants — что НЕ должно сломаться

1. **PR-1 placeholder welcome.mdx остаётся функциональным** — extended schema backwards-compat (все новые поля optional).
2. **Existing `/`, `/docs/*`, `/ru/docs/*`, `/blog`, `/ru/blog`, `/blog/welcome`, `/ru/blog/welcome` resolve 200** после PR-2A.
3. **Шрифтовой стек НЕ меняется** — Space Grotesk + Geist Mono (NFR из PRD-079 переносится).
4. **Tokens scoped** — 6 новых topic-colors добавляются под `.blog-post, .blog-index`, не на `:root`.
5. **0 JS by default** — Hero, TOC, Callout, SeriesNav, TopicChip, AuthorBlock, SeoMeta — все Astro components без `client:*` directives.
6. **Build degradation ≤ 15%** — vs PR-1 baseline 15.38s → ≤ 17.7s acceptable.
7. **No regression in RFC-011 6 invariants** — Landing pixel-identical (Header не трогаем в PR-2A), docs работают, fonts stack тот же.

## Rollback Plan

Если PR-2A после merge ломает блог:

1. **Revert merge commit** — стандартный путь:
   ```bash
   git checkout dev && git revert -m 1 <sha> && git push origin dev
   ```
   Возвращает блог в PR-1 state. Зеро downtime.

2. **Partial rollback** — отключить только `<SeoMeta>` (если HEAD baking ломается):
   - Comment out `<SeoMeta>` import в BlogPost.astro
   - Build снова проходит, posts работают без SEO meta (regression к PR-1 SEO baseline = только title+description).

3. **Schema rollback** — если backwards-compat нарушился (welcome.mdx ломается):
   - Удалить `series?/seriesOrder?/seriesDescription?` поля → откат к PR-1 schema.
   - Удалить series landing routes.

Worst case downtime: 0 (static rebuild ~15-18s).

## Risks & mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| File overlap workers (blog-theme.css) | MEDIUM | Раздельные секции с маркерами; lead consolidate если diff overlap |
| Reading-time plugin breaks build на Astro 6 | LOW-MED | Известный pattern (reading-time для Astro есть в community); fallback — manual `readingTime` field; PR-1 schema уже её имеет |
| TOC sticky не работает с Astro hydration | LOW | Pure CSS sticky, без JS; testable в build preview |
| JSON-LD validation false-positive | LOW | Test через schema.org validator после build; fallback — remove если есть проблема |
| Series landing 404 для серии без posts | LOW | getStaticPaths возвращает только existing series → 404 not generated |
| og-default.png design | LOW | 1200×630, brand-coherent (dark, dotted bg, "Forgeplan" wordmark); создаётся вручную или CSS→image conversion |

## Phases

### Phase A — Lead, in main worktree (~20 min)

1. Worktree create: `git worktree add ../forgeplan-blog-foundation -b feat/blog-foundation origin/dev`.
2. Schema extension в `content.config.ts` (FR-001).
3. Build verify (welcome.mdx not broken).
4. Commit: `chore(blog): extend schema with series + seriesOrder + seriesDescription`.

### Phase B — 2 parallel workers in sub-worktrees (~1-1.5 hour each)

Worker 1 (Visual primitives) — branch `feat/blog-foundation-w1-visual` off foundation:
- TOC.astro, Hero.astro, Callout.astro, SeriesNav.astro
- BlogPost.astro integration (hero replacing inline title block, TOC slot, SeriesNav footer)
- blog-theme.css: visual primitives section
- npm run build → verify
- Final report

Worker 2 (Discovery + SEO + author) — branch `feat/blog-foundation-w2-discovery`:
- TopicChip.astro, AuthorBlock.astro, SeoMeta.astro
- Series landing routes (EN + RU)
- pages/blog/index.astro + RU mirror: topic chips + thumbnail rendering
- reading-time remark plugin
- og-default.png (placeholder OR ручной если есть бренд-asset)
- astro.config.mjs: remark plugin wire
- blog-theme.css: discovery section
- npm run build → verify
- Final report

### Phase C — Lead consolidate + integration (~30 min)

1. Lead merges w1 + w2 branches into `feat/blog-foundation`.
2. Wire SeoMeta + AuthorBlock в BlogPost.astro (post-worker integration).
3. Full `npm run build` test.
4. Visual smoke в `npm run dev`.

### Phase D — Audit (Step 6.5 forge-cycle, ~30 min)

2 parallel reviewers:
- `agents-core:code-reviewer` — quality, anti-patterns
- `agents-pro:architect-reviewer` — RFC fit + invariants compliance

Если HIGH findings — fix-coder round.

### Phase E — EVID + activate + commit + STOP

EVID linked to PRD-080. Activate PRD-080 + RFC-012. Commit. **NO push** без user approval.

## Acceptance — RFC-012

- [ ] Phase A 4 шагов выполнены (schema extension + welcome.mdx not broken).
- [ ] Phase B оба workers вернули success report.
- [ ] Phase C lead merge + integration без conflicts; full build green; visual smoke OK.
- [ ] Phase D audit: 0 HIGH unfixed.
- [ ] All 12 FRs реализованы.
- [ ] All 8 NFRs соблюдены.
- [ ] All 7 Invariants intact.
- [ ] Linked based_on PRD-080.

## Related Artifacts

- based_on PRD-080
- references RFC-011 (PR-1 architecture — extends)
- references PRD-079 (parent product req)
- FPF evaluate output: F-G-R на 22 кандидата (12 MUST selected)
- future: PRD-081 (PR-2B), PRD-082 (PR-3)

