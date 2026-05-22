---
depth: standard
id: PRD-079
kind: prd
last_modified_at: 2026-05-22T22:59:53.611938+00:00
last_modified_by: claude-code/2.1.149
status: active
title: Bilingual blog as third mode in forgeplan.dev website
---

# PRD-079: Bilingual blog as third mode in forgeplan.dev website

## Problem Statement

`forgeplan.dev` сейчас имеет 2 mode внутри одного веб-приложения (`ForgePlan/website`):
1. **Landing** (`/`) — кастомная страница с интерактивными секциями.
2. **Docs** (`/docs/*` + `/ru/docs/*`) — developer reference.

Что есть в экосистеме, но **некуда публиковать**:
- `ForgePlanMarketing/posts/` — готовые drafts (10+ файлов в воронке).
- `ForgePlanMarketing/teaching-assets/trust-calculus.html` — высококачественные learning materials с интерактивной визуализацией (3D F/G/R chart, evidence таблицы, hypothesis cards, winner-card паттерн).
- `CONTENT-CALENDAR-V2.md` — расписание контента на квартал.
- Знания, накопленные из live ForgePlan-разработки (release retrospectives, decision case studies, методологические разборы).

**Проблема**: dev-audience не имеет канала для long-form контента вне маркетинговых каналов. Landing — sales surface, docs — reference. Между ними gap для **storytelling about decisions, releases, и методологии**. Этот gap = упущенная воронка top-of-funnel и упущенная opportunity для community building.

Дополнительно: visual identity из teaching-assets (тёмная палитра, F/G/R color code, dotted bg, card pattern) уже **выработана и валидирована** — нужен permanent home, иначе она тонет в репо.

## Target Audience

| Audience | Need | Why blog |
|----------|------|----------|
| **Forgeplan power users / contributors** | Понимать ход мысли мейнтейнеров: почему выбран weakest-link R_eff, почему ADR-003 markdown=truth, как ADI разрешает архитектурные конфликты | Docs объясняют **что** и **как**, блог объясняет **почему** — недостающий слой |
| **Methodology-curious developers** (BMAD/OpenSpec/Quint-code audience) | Discover Forgeplan через сравнения, case studies, разборы decisions | Top-of-funnel канал параллельный docs |
| **AI agent operators / prompt engineers** | Узнавать про hint contract, multi-agent dispatch patterns, sub-agent worktree tips | Тематические teaching posts + release notes deep-dives |
| **Forgeplan team itself** | Долговременная память — что зачем сделано в каком релизе, какие audit findings закрыты как | Замена ephemeral `HANDOFF-*.md` и memory snapshots на структурированные публичные records |
| **Bilingual constraint** | Русскоязычные команды (российский рынок) + английская developer community | EN root + RU mirror routes, симметрия как у docs |

## Goals

1. **Публикационный канал** — bilingual (EN/RU) блог по адресу `forgeplan.dev/blog` и `forgeplan.dev/ru/blog`, доступный с лендинга через Header link, с поддержкой расширенного Markdown (с возможностью inline interactive components в будущих постах).
2. **Архитектурная чистота** — блог как **третий mode** в существующем веб-приложении, без отдельного репо/сабдомена/деплоя. Visual continuity с landing (тот же шрифтовой стек и tokens).
3. **Bilingual symmetry** — каждый пост имеет EN и RU версии по mirror routes (`/blog/<slug>` ↔ `/ru/blog/<slug>`), как уже делает docs.
4. **Type-safe content** — все frontmatter валидируется через схему на build time; неполный пост = build-fail, не silent publication.

## Functional Requirements

- [ ] FR-001: Content Collection `blog` — экспорт `blog` collection с schema (title, description, slug, lang en/ru, publishedAt, updatedAt?, kind explainer/case-study/teaching/release-notes/deep-dive, topic r-eff/adi/fpf/mcp/methodology/release, artifacts?, cover?, draft default false, readingTime?, translations?). Build fails при invalid frontmatter.
- [ ] FR-002: Extended Markdown integration — поддержка inline interactive components в `.mdx` файлах из `src/content/blog/{en,ru}/`. Author can renderable component inline (e.g. `<Chart />` рядом с текстом).
- [ ] FR-003: Routes EN — `src/pages/blog/index.astro` показывает список постов с `lang=en`, отсортированный по publishedAt desc, draft=false. `src/pages/blog/[...slug].astro` рендерит индивидуальный пост через blog layout. URL: `forgeplan.dev/blog` и `forgeplan.dev/blog/<slug>`.
- [ ] FR-004: Routes RU — `src/pages/ru/blog/index.astro` + `src/pages/ru/blog/[...slug].astro` — mirror EN, фильтрует `lang=ru`. URL: `forgeplan.dev/ru/blog` и `forgeplan.dev/ru/blog/<slug>`.
- [ ] FR-005: RSS feeds — `src/pages/blog/rss.xml.ts` + `src/pages/ru/blog/rss.xml.ts`. Каждый язык — отдельный feed. URL: `forgeplan.dev/blog/rss.xml` и `forgeplan.dev/ru/blog/rss.xml`.
- [ ] FR-006: Blog post layout — переиспользует существующий `Header.astro` и существующий шрифтовой стек. Включает: title h1, meta (publishedAt, readingTime, topic, kind), content slot, footer с LangSwitcher (cross-language link для текущего поста).
- [ ] FR-007: Header Blog link — `src/components/Header.astro` имеет ссылку «Blog» (en) / «Блог» (ru) между Docs и GitHub. Active state когда current path начинается с `/blog` или `/ru/blog`.
- [ ] FR-008: Blog theme tokens — `src/styles/blog-theme.css` — палитра + типографика портированы из `ForgePlanMarketing/teaching-assets/trust-calculus.html`: --bg/--bg-1/--bg-2/--bg-3, --text/--text-1/--text-2/--text-3/--text-4, --line/--line-2, --accent (#ff5a1f) + --accent-soft + --accent-bg, --ok/--err/--warn, --f-color (#ff5a1f) / --g-color (#22c55e) / --r-color (#60a5fa), card style, dotted background. **--font-sans = существующий sans-шрифт сайта** (НЕ Inter), **--font-mono = существующий mono-шрифт сайта** (НЕ JetBrains Mono) — visual continuity с website. Scoped к `.blog-post` чтобы не аффектить landing/docs.
- [ ] FR-009: Placeholder content — `src/content/blog/en/_intro.mdx` + `src/content/blog/ru/_intro.mdx` — single placeholder per locale с draft=false, kind=explainer, чтобы `/blog` index был не пустой и build не падал. Удаляются/заменяются в PR-2.

## Non-Functional Requirements

| ID | Requirement | Acceptance Criteria |
|----|-------------|--------------------|
| NFR-001 | Build perf | Полный `build` команды в `website/` завершается БЕЗ degradation > 10% от baseline (current ~30-60s). |
| NFR-002 | Bundle size | Blog routes ship 0 JS by default (islands лениво). RSS endpoints — server-only, не в client bundle. |
| NFR-003 | No regression | Existing landing (`/`) и docs (`/docs/*`, `/ru/docs/*`) — НЕ ломаются. Все existing routes resolve 200 после merge. |
| NFR-004 | Type safety | Schema валидируется на build. Type-checker в `website/` — 0 errors. |
| NFR-005 | Sitemap inclusion | Если sitemap integration присутствует (TBD при audit) — блог routes автоматически попадают в sitemap.xml. Если не присутствует — отдельный follow-up, не блокер PR-1. |
| NFR-006 | Accessibility | Blog post layout: skip-to-content link, semantic html (article, header, footer, time с datetime attribute), language attribute на html. |
| NFR-007 | i18n consistency | URL pattern блога **идентичен** docs: `/blog` = EN root, `/ru/blog` = RU. Никакого top-level i18n middleware — mirror routes. |

## Non-Goals

- **NOT** отдельный subdomain `blog.forgeplan.dev` — единый сайт.
- **NOT** CMS (Decap/Tina/Strapi) — авторы пишут extended Markdown в репо через PR.
- **NOT** комментарии/реакции/auth — read-only публикация в этой итерации.
- **NOT** автоматический cross-post в соцсети — отдельный pipeline в `ForgePlanMarketing/outbox/`.
- **NOT** редизайн landing или docs — блог использует существующие primitives.
- **NOT** контент в этом PR — только инфраструктура. Контент (pilot trust-calculus) идёт в PR-2.
- **NOT** новые шрифты — keep existing sans + mono stack.
- **NOT** Mermaid в блоге в PR-1 — оценим необходимость в PR-2.
- **NOT** OG image generator в PR-1 — отдельный follow-up.

## Related Artifacts

- `ForgePlanMarketing/teaching-assets/trust-calculus.html` — source-of-truth для visual tokens (FR-008), портируется в `blog-theme.css`.
- `src/layouts/Landing.astro` — existing landing layout, шрифты и meta-pattern (preconnect, dark class) переиспользуем (FR-006).
- `src/components/Header.astro` — modify для Blog link (FR-007).
- `src/content.config.ts` — extend для blog collection (FR-001) без ломания existing docs/i18n collections (NFR-003).
- `astro.config.mjs` — add extended-markdown integration (FR-002).
- Future: PRD-080 (PR-2 — pilot post trust-calculus + 6 MDX-компонентов) — depends on this PRD.

## Open Questions (surface during ADI)

1. **Sitemap presence** — есть ли уже sitemap integration в `package.json`? Если нет — block PR-1 или follow-up?
2. **Header link copy** — «Blog» (EN) / «Блог» (RU) или «Журнал» (RU)? Решение в PR-1 review.
3. **Placeholder content visibility** — `_intro.mdx` (FR-009): draft=true (скрыт из index) или draft=false (виден)? Default: draft=false но minimal content.

## Acceptance Criteria

- [ ] FR-001..FR-007: реализованы и проверены build success в worktree.
- [ ] FR-008: blog-theme.css содержит все 4 группы tokens (bg/text/line/accent + F/G/R + card + dotted-bg) портированные из teaching-asset.
- [ ] FR-009: placeholder _intro.mdx в обоих локалях; build не падает на пустом index.
- [ ] All 7 NFR соблюдены (build green, 0 regression, type-safe, sitemap-aware).
- [ ] EvidencePack с verdict/CL/type linked informs PRD-079.
- [ ] R_eff(PRD-079) > 0 после link.
- [ ] Audit ≥ 2 agents (code-reviewer + architect-reviewer) — HIGH/CRITICAL = 0.
- [ ] PRD activated (draft → active).
- [ ] Commit on `feat/blog-scaffold` с `Refs: prd-bilingual-blog-as-third-mode-in-forgeplan-dev-website`.
- [ ] User-approved push (RED LINE #2).





