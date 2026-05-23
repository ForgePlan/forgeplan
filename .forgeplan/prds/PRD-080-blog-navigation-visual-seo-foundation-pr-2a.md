---
depth: standard
id: PRD-080
kind: prd
last_modified_at: 2026-05-23T00:10:11.135439+00:00
last_modified_by: claude-code/2.1.149
links:
- target: PRD-079
  relation: based_on
status: draft
title: Blog navigation + visual + SEO foundation (PR-2A)
---

# PRD-080: Blog navigation + visual + SEO foundation (PR-2A)

## Problem Statement

PR-1 (PRD-079, merged 2026-05-22) поставил блог-scaffold: Content Collection, mirror routes, BlogPost layout, scoped theme. Это **минимально жизнеспособная инфраструктура**, но не готовый блог — не хватает **навигационных, визуальных и SEO primitive'ов**, без которых:

1. **Контент-launch ломается**: 5 циклов формируют серию (trust → decision → spec → bmad → forgeplan); без `series` поддержки читатель не понимает порядок чтения.
2. **Длинные посты не сканируются**: каждый цикл = 4-6 секций с интерактивными визуализациями; без Table of Contents UX страдает на постах 2000+ слов.
3. **Listing невзрачный**: 10 постов без cover/thumbnail — «лысый» список.
4. **Bilingual SEO рискует duplicate-content penalty**: 5 циклов × 2 языка без canonical + hreflang Google спутает RU/EN версии.
5. **Социальный шеринг = naked link**: без og:image + twitter:card preview shareable surface страдает.
6. **Атрибуция автору отсутствует**: одно-авторный блог требует чёткой ссылки на elirum.me для credibility.
7. **Контент-форматы ограничены**: long-form методологические посты требуют callout-блоков (info/warn/tip/danger).

PR-2A заполняет эти 12 primitive'ов **до того как** PR-2B/3 заливают 10 постов. Иначе контент шипится через черновой UX, что создаёт rework-долг.

## Target Audience

| Audience | Need | How PR-2A serves |
|----------|------|------------------|
| Methodology-curious dev (Forgeplan power users) | Сканировать длинный пост по секциям, понять контекст серии, оценить authority автора | TOC, series prev/next, author block с elirum.me |
| Casual reader из X/LinkedIn share | Preview-картинка в социалке, заголовок, описание | og:image + twitter:card |
| Russian-speaking team из РФ | Найти русскую версию из английской и наоборот, читать на родном языке без duplicate-content наказания Google | hreflang на странице + canonical |
| Forgeplan team (long-tail) | Discover старые посты по теме (r-eff, adi, methodology), переключаться между сериями | Topic chips + 6 цветов, series support |
| Author (я сам) | Минимум ручной работы при публикации поста: reading-time auto, cover опционально, hero без обязательного дизайна | Hybrid hero (CSS fallback), auto reading time |

## Goals

1. **Series-aware блог**: каждый пост может принадлежать серии; серия имеет landing page; в посте есть prev/next в серии.
2. **Scan-friendly long-form**: Table of Contents в sticky right rail; auto reading time.
3. **Brand-coherent visual**: каждая страница (пост + listing card) имеет hero/thumbnail — либо frontmatter `cover`, либо CSS fallback с topic-цветом.
4. **SEO baseline для bilingual launch**: canonical, hreflang, og:image, JSON-LD BlogPosting на каждой странице.
5. **Минимальная атрибуция**: author block с одним автором (Mike Kubal → elirum.me) — без author pages.
6. **Content authoring primitives**: Callout components (info/warn/tip/danger), topic chips для discovery.

## Functional Requirements

- [ ] FR-001: Schema extension — `series?: string`, `seriesOrder?: number`, `cover?: string` поля в `content.config.ts`. Все optional, backwards-compatible с PR-1 frontmatter.
- [ ] FR-002: Series landing page — dynamic route `/blog/series/[name]` + `/ru/blog/series/[name]`; рендерит все посты данной серии в seriesOrder asc; шапка серии = название + описание + posts count.
- [ ] FR-003: Series prev/next-in-series footer block в `BlogPost.astro` — рендерится только если у поста есть `series`; показывает predecessor + successor с их title.
- [ ] FR-004: Table of Contents component — sticky right rail (≥1024px viewport); derives from MDX headings h2/h3; clicking scrollst к секции с smooth scroll; на mobile — collapse в expandable details.
- [ ] FR-005: Reading time auto-calc — remark plugin counts characters / 200wpm равноязычно; результат доступен через `data.readingTime` (auto-set если в frontmatter не задан).
- [ ] FR-006: Hybrid hero block — `BlogPost.astro` рендерит: если `frontmatter.cover` set → `<img src={cover}>` поверх content; иначе → CSS hero (eyebrow + h1 + dotted bg + accent stripe в topic color).
- [ ] FR-007: Card thumbnail rendering — `pages/blog/index.astro` и RU mirror: если `data.cover` set → `<img>`; иначе → CSS gradient placeholder с topic color + topic name overlay.
- [ ] FR-008: Topic chips — каждая карточка в listing показывает chip с topic name, окрашенный в topic-specific цвет.
- [ ] FR-009: 6 topic colors в `blog-theme.css` — `--t-r-eff: #ff5a1f`, `--t-adi: #60a5fa`, `--t-fpf: #22c55e`, `--t-mcp: #a855f7`, `--t-methodology: #06b6d4`, `--t-release: #737373`. Под scope `.blog-post, .blog-index`.
- [ ] FR-010: Author meta block — компактный footer-блок в `BlogPost.astro`: name «Mike Kubal», ссылка на https://elirum.me, descriptor «Author». Static, не configurable в frontmatter (single author).
- [ ] FR-011: SEO meta on BlogPost — `<link rel="canonical">` + `<link rel="alternate" hreflang>` для cross-lang counterpart (если в `translations`) + `<meta property="og:image">` (per-post cover OR `/og-default.png` fallback) + twitter:card + JSON-LD BlogPosting structured data.
- [ ] FR-012: Callout components — `Callout.astro` с props `type: "info" | "warn" | "tip" | "danger"`; четыре варианта стилизации с соответствующими цветами (info=R blue, warn=accent orange, tip=G green, danger=err red); MDX-usable as `<Callout type="warn">...</Callout>`.

## Non-Functional Requirements

| ID | Requirement | Acceptance Criteria |
|----|-------------|--------------------|
| NFR-001 | Build perf | Полный build в worktree exits 0 без degradation > 15% от PR-1 baseline (15.38s). |
| NFR-002 | Bundle size | TOC и Callout — Astro components без client-side JS (`client:` directives отсутствуют). Hero без images = 0 KB images. |
| NFR-003 | No regression | Existing `/`, `/docs/*`, `/ru/docs/*`, `/blog`, `/ru/blog`, `/blog/welcome`, `/ru/blog/welcome` resolve 200 после изменений. PR-1 placeholder welcome.mdx remains functional с extended schema (cover/series optional). |
| NFR-004 | Type safety | Расширенная schema type-checked; existing PR-1 файлы (welcome.mdx) не нарушаются. |
| NFR-005 | SEO validity | hreflang pair правильный (en↔ru bidirectional); canonical = self URL; JSON-LD validates по schema.org BlogPosting. |
| NFR-006 | Accessibility | TOC keyboard-navigable; Callout uses `role="note"` или подходящий semantic. |
| NFR-007 | i18n consistency | Series landing page и topic colors работают одинаково на EN и RU routes. |
| NFR-008 | A scale | Все features работают при 10 постах (текущий target launch) и при 100 (хотя pagination = future-scope). |

## Non-Goals

- **NOT** topic landing pages (`/blog/topics/[topic]`) — chips есть, отдельные dynamic routes отложены в PR-2B.
- **NOT** author pages (`/blog/author/mike-kubal`) — single author, бессмысленно.
- **NOT** pagination — <30 posts, не нужно.
- **NOT** search — <30 posts, Cmd-F + RSS достаточны.
- **NOT** newsletter — нет email infra.
- **NOT** comments — Non-Goal из PRD-079.
- **NOT** share buttons — elirum.me централизует социалки + dev audience сами шарят.
- **NOT** related posts — algorithm-dependent, requires similarity model.
- **NOT** date archive — single year of content.
- **NOT** chronological prev/next — series covers ordering при текущем launch.
- **NOT** content в этом PR — 0 контентных постов мутируется; placeholder welcome.mdx остаётся.

## Related Artifacts

- based_on PRD-079 (parent — blog scaffold, PR-1 merged)
- based_on RFC-011 (parent architecture — referenced for scope continuity)
- references FPF evaluate output (F-G-R на 22 кандидата → 12 MUST PR-2A)
- references `ForgePlanMarketing/teaching-assets/*.html` (5 cycles — content source для PR-2B/3, не для PR-2A)
- future: PRD-081 (PR-2B — 6 MDX components + 2 cycles), PRD-082 (PR-3 — remaining 3 cycles)

## Open Questions

- **Q1: TOC scroll behavior** — smooth-scroll или instant? Recommendation: smooth (CSS `scroll-behavior: smooth` на html).
- **Q2: Series description** — где хранится? В первом посте серии как `seriesDescription` поле или в отдельном `.forgeplan/series/[name].yaml`? Recommendation: первый пост с `seriesOrder=1` имеет optional `seriesDescription` поле, остальные посты наследуют. Если не задано — fallback на список постов без descriptive header.
- **Q3: og:image fallback** — `/og-default.png` единая на все посты или per-topic 6 разных? Recommendation: 1 default + ручная per-post через cover. Per-topic = over-engineering для launch.

## Acceptance Criteria

- [ ] FR-001..FR-012 реализованы, проверены build success в worktree.
- [ ] NFR-001..NFR-008 соблюдены.
- [ ] PR-1 placeholder welcome.mdx остаётся live (extended schema backwards-compat).
- [ ] Series landing `/blog/series/<name>` рендерит posts (тестируется на ad-hoc series для testing).
- [ ] EvidencePack с verdict/CL/type linked informs PRD-080.
- [ ] R_eff(PRD-080) > 0 после link.
- [ ] Audit ≥ 2 agents (code-reviewer + architect-reviewer) — HIGH/CRITICAL = 0.
- [ ] PRD activated (draft → active).
- [ ] Commit на `feat/blog-foundation` с правильными Refs.
- [ ] User-approved push (RED LINE #2).



