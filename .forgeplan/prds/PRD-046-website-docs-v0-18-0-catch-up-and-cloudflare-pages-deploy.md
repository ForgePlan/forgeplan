---
depth: standard
id: PRD-046
kind: prd
links:
- target: PROB-035
  relation: based_on
- target: PRD-024
  relation: refines
status: active
title: Website docs v0.18.0 catch-up and Cloudflare Pages deploy
---

# PRD-046: Website docs v0.18.0 catch-up and Cloudflare Pages deploy

## Progress

```
Phase 1 Shape    ████████████████████████  2/2  (100%)
Phase 2 Build    ░░░░░░░░░░░░░░░░░░░░░░░░  0/6  (  0%)
Phase 3 Deploy   ░░░░░░░░░░░░░░░░░░░░░░░░  0/2  (  0%)
─────────────────────────────────────────────────
TOTAL                                      2/10 ( 20%)
```

---

## Executive Summary

### Vision

Привести docs portal Forgeplan в соответствие с реальным состоянием v0.18.0 и опубликовать сайт на Cloudflare Pages, чтобы внешние пользователи и AI-агенты могли найти актуальную reference по всем 58 CLI командам и 47 MCP tools.

### Problem

**Кому плохо**: разработчики, AI-агенты и студенты курса, которые ищут документацию Forgeplan. **Что происходит**: docs portal застрял на v0.15.0 (2026-04-06) — после этого были выпущены v0.16, v0.17.x, v0.18.0 с production BM25, Russian morphology, lifecycle v2, health-debt cleanup, tag canonicalization, reindex rebuild. В результате **CLI reference покрывает 1.7% команд** (1 из 58), **MCP reference — 2.1%** (1 из 47), **CHANGELOG и migration guide отсутствуют**. Сайт не задеплоен на публичный URL.

**Impact**:
- Внешний пользователь установит v0.18.0 binary → откроет docs → не найдёт 57 CLI команд → уйдёт к конкурентам (Quint-code, BMAD, OpenSpec).
- AI-агент не сможет цитировать docs для команд типа `forgeplan reindex`, `forgeplan renew`, `forgeplan reopen` → будет галлюцинировать.
- Студенты курса (см. `.local/course-material/forgeplan-course-brief.md`) получат сломанные ссылки на docs.
- Полная деталь пробелов — `PROB-035`.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| External developer | Оценивает Forgeplan для своего проекта | Хочет увидеть полный CLI reference + installation — иначе не поверит что продукт зрелый |
| AI coding agent (Claude Code, Cursor) | Автоматизирует работу с Forgeplan через MCP | Нужен `docs/mcp/*` reference для корректного вызова tools |
| Course student | Изучает методологию по курсу | Нужен completeness + примеры, чтобы выполнить lab-задания |
| Contributor | Хочет внести PR | Нужна актуальная ARCHITECTURE/CLI documentation чтобы не дублировать существующее |

### Differentiators

- **Generated from source of truth** — CLI reference из `forgeplan help <cmd>`, MCP из Rust source. На следующем релизе достаточно перезапустить генератор, не переписывать вручную.
- **Starlight autogenerate sidebar** — новые страницы автоматически появляются в навигации.
- **Integrated CHANGELOG** — единственный источник release notes (`CHANGELOG.md`) импортируется в docs через MDX.

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | CLI reference coverage | страниц в `docs/cli/` | 1 | ≥ 58 | 2026-04-11 | `ls website/src/content/docs/cli/*.md \| wc -l` |
| SC-2 | MCP reference coverage | страниц в `docs/mcp/` | 1 | ≥ 47 | 2026-04-11 | `ls website/src/content/docs/mcp/*.md \| wc -l` |
| SC-3 | Build cleanliness | warnings при `astro build` | 2 | 0 | 2026-04-11 | `npm run build` exit 0, stderr grep `WARN` = 0 |
| SC-4 | Build speed | seconds для full build | 3.45s | < 12s | 2026-04-11 | `time npm run build` |
| SC-5 | Total pages | `dist/` HTML count | 24 | ≥ 110 | 2026-04-11 | `find dist -name 'index.html' \| wc -l` |
| SC-6 | Pagefind recall | keyword→page@top3 для 10 command queries | unmeasured | ≥ 8/10 | 2026-04-11 | ручной smoke test |
| SC-7 | Deploy live | HTTP 200 на production URL | 404/none | 200 | 2026-04-11 | `curl -I https://forgeplan.dev` (или beta pages.dev) |
| SC-8 | Evidence + R_eff | R_eff(PRD-046) > 0 | 0.00 | ≥ 0.80 | 2026-04-11 | `forgeplan score PRD-046` |

---

## Product Scope

### MVP (In-Scope)

- **CLI reference generator** — Rust/bash script, парсящий `forgeplan help <cmd>` для 58 команд → `.md` с frontmatter `title` + `description`.
- **MCP reference generator** — парсер Rust source (`crates/forgeplan-mcp/src/tools/*.rs`, `#[tool]` макросы) → 47 `.md` страниц с input schema и примером.
- **CHANGELOG import** — `src/content/docs/changelog.mdx` импортирует корневой `CHANGELOG.md` (Starlight MDX + markdown loader).
- **v0.18 feature guides** — `guides/search-v2.md` (BM25 + Russian morphology), `guides/lifecycle-v2.md` (stale/renew/reopen с примерами CLI).
- **Build fixes** — `404.md` страница, `site: 'https://forgeplan.dev'` в `astro.config.mjs`, sitemap.xml генерируется.
- **Cloudflare Pages deploy** — проект в CF Pages, build cmd `cd website && npm run build`, output `website/dist`, auto-deploy на push в `dev` и `main`. Минимум — beta subdomain или `*.pages.dev`.

### Out of Scope

- Переписывание landing page, GSAP анимаций, Hero section.
- Redesign Trust v2, Pipeline v2, Graph section.
- Миграция на Nx монорепо (PRD-025).
- Multi-language docs (RU/EN) — пока только английский + русские примеры в коде.
- SSR / dynamic pages — остаётся static output.
- Custom Starlight theme rewrite — используем существующий forge-theme.
- Commerce / paywall.

### Growth Vision

- **Auto-regen в CI** — GitHub Action запускает генераторы при каждом релизе, открывает PR с diff.
- **API reference из Rust docs** — `cargo doc` → импорт в Starlight для contributor-facing internals.
- **Search analytics** — Pagefind + privacy-friendly analytics (Plausible) чтобы видеть какие запросы не находят результата.

---

## User Journeys

### Journey 1: External developer evaluates Forgeplan

**Цель**: понять подходит ли Forgeplan для своего workflow за 10 минут.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | Открывает `https://forgeplan.dev` | Видит landing (Hero + Trust + Pipeline + Install) | Уже работает из PRD-024 |
| 2 | Кликает "Docs" в nav | Попадает на `/docs/getting-started/installation` | Sidebar показывает все разделы |
| 3 | Ищет `validate` в Pagefind | Получает `docs/cli/validate` как top result | FR-001, FR-006 |
| 4 | Открывает CLI reference → validate | Видит usage, args, examples, exit codes | FR-001 |
| 5 | Возвращается в sidebar → "CHANGELOG" | Видит v0.18.0 features с датами | FR-003 |
| 6 | Устанавливает binary по Installation | Binary v0.18.0 установлен | FR-008 (installation актуален) |

**Результат**: разработчик доверяет продукту и ставит binary.

### Journey 2: AI agent calls MCP tool

**Цель**: Claude Code вызывает `forgeplan_health` через MCP и хочет убедиться в input schema.

| Шаг | Действие | Ответ системы | Заметки |
|-----|----------|---------------|---------|
| 1 | Агент запрашивает `docs/mcp/forgeplan_health` | Получает markdown с schema + example | FR-002 |
| 2 | Парсит frontmatter `description` | Использует как tooltip для пользователя | FR-002 |
| 3 | Формирует вызов с args из примера | Успешный MCP response | FR-002 |

**Результат**: MCP интеграция работает без галлюцинаций.

### Journey 3: Course student follows tutorial

**Цель**: студент курса проходит "Полный цикл Shape→Validate→ADI→Code".

| Шаг | Действие | Ответ системы | Заметки |
|-----|----------|---------------|---------|
| 1 | Открывает `docs/methodology/lifecycle` | Видит state machine с примерами | FR-005 |
| 2 | Переходит по ссылке на `docs/cli/new` | Видит пример `forgeplan new prd` | FR-001 |
| 3 | Студент спрашивает про lifecycle v2 (renew) | Читает `docs/guides/lifecycle-v2` | FR-005 |
| 4 | Выполняет lab → смотрит `docs/cli/renew` | Видит examples, exit codes | FR-001, FR-005 |

**Результат**: студент прошёл lab без вопросов в чат.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | Reader can find every CLI command on its own reference page with usage, args, examples | Journey 1, 3 |
| FR-002 | Core | Must | AI agent can retrieve every MCP tool schema, description and example from docs portal | Journey 2 |
| FR-003 | Core | Must | Reader can browse full CHANGELOG from docs sidebar with entries for v0.10..v0.18 | Journey 1 |
| FR-004 | Core | Must | Reader can read dedicated v0.18 guides explaining BM25 search and Russian morphology | Journey 1 |
| FR-005 | Core | Must | Reader can read lifecycle v2 guide covering stale/renew/reopen transitions | Journey 3 |
| FR-006 | UX | Must | Reader can search any CLI command or MCP tool by keyword and reach its page in top-3 Pagefind results | Journey 1 |
| FR-007 | Build | Must | Build produces a custom 404 page instead of Starlight default warning | Journey 1 |
| FR-008 | Build | Must | Build generates a sitemap.xml reflecting the production site URL | Journey 1 |
| FR-009 | Deploy | Must | External visitor can reach the site on a public HTTPS URL via Cloudflare Pages | Journey 1, 2, 3 |
| FR-010 | Ops | Should | Maintainer can regenerate CLI and MCP reference pages with a single command when Forgeplan is rebuilt | Journey 1 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Build performance | Build shall complete | < 12s | 110+ static pages, cold cache | `time npm run build` |
| NFR-002 | Pagefind index | Search index shall cover | 100% pages | All `src/content/docs/**` | `dist/pagefind/pagefind-entry.json` |
| NFR-003 | Bundle size | dist shall stay under | 15 MB | Excluding pagefind shards | `du -sh dist` |
| NFR-004 | Accessibility | All new pages shall pass | Lighthouse a11y ≥ 90 | CLI + MCP generated pages | Lighthouse CI on sample |
| NFR-005 | Deploy time | CF Pages build shall complete | < 5 min | Fresh install, no cache | CF Pages build log |
| NFR-006 | Uptime | Site shall maintain | 99.9% | Monthly | CF Pages built-in monitoring |

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| PRD-024 Website | Internal | Active (R_eff 1.00) | Forgeplan core |
| Forgeplan v0.18.0 binary | Internal | Released | Forgeplan core |
| Cloudflare account with Pages | External | Needed (user action) | gogocat |
| Domain forgeplan.dev | External | Unknown — fallback `*.pages.dev` | gogocat |
| Astro 6.0.1 / Starlight 0.38.2 | External | Installed | npm |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | `forgeplan help <cmd>` output format changes in future release → generator breaks silently | Medium | High | Snapshot tests on generator output; regen in CI on every release | Core |
| R-2 | MCP tools parser misses new tools added outside `crates/forgeplan-mcp/src/tools/` | Low | High | Cross-check count against `grep -r '#\[tool' crates/forgeplan-mcp` in generator | Core |
| R-3 | Build exceeds 12s with 110+ pages → SC-4 fails | Low | Medium | Profile with `ASTRO_TELEMETRY_DISABLED=1`; split long pages | Core |
| R-4 | Cloudflare account access missing → deploy blocked | Medium | High | Fall back to `*.pages.dev` URL; document manual deploy via wrangler | User |
| R-5 | Starlight autogenerate sorts pages alphabetically → UX bad (59 CLI pages unordered) | High | Low | Group frequent commands via manual sidebar override for top-10 | Core |
| R-6 | v0.18 feature guide references wrong API → misleads users | Low | High | Validate examples by running them against built binary before commit | Core |

---

## Acceptance Criteria

### AC-1: CLI reference generator produces all pages

```gherkin
Given forgeplan v0.18.0 binary is built
When the CLI docs generator is executed
Then website/src/content/docs/cli/ contains ≥ 58 .md files
And each file has frontmatter {title, description}
And npm run build passes with exit 0
```

### AC-2: MCP reference generator produces all pages

```gherkin
Given crates/forgeplan-mcp/src/tools/ exists
When the MCP docs generator is executed
Then website/src/content/docs/mcp/ contains ≥ 47 .md files
And each file documents input schema and returns one runnable example
```

### AC-3: CHANGELOG mdx renders

```gherkin
Given CHANGELOG.md exists at repository root with entries for v0.10..v0.18
When npm run build is executed
Then dist/docs/changelog/index.html exists
And it contains the text "v0.18.0"
And it contains the text "BM25"
```

### AC-4: Build clean

```gherkin
Given all previous ACs pass
When npm run build is executed from clean state
Then exit code is 0
And stdout contains no "[WARN]" substring
And dist contains ≥ 110 index.html files
And dist/pagefind/pagefind-entry.json exists
```

### AC-5: Deploy live

```gherkin
Given CF Pages project is configured
When a commit is pushed to feat/prd-046-docs-v0.18-deploy branch
Then a preview URL becomes HTTP 200 within 5 minutes
And homepage shows "F●RGEPLAN" and Crystallization animation
And docs/cli/init responds with 200
And docs/mcp/forgeplan_health responds with 200
```

---

## Timeline

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-11 | Requirements locked (this PRD validated) |
| Wave 1 CLI gen | 2026-04-11 | 58 CLI pages committed |
| Wave 2 MCP gen | 2026-04-11 | 47 MCP pages committed |
| Wave 3 CHANGELOG + fixes | 2026-04-11 | 404, sitemap, changelog.mdx |
| Wave 4 v0.18 guides | 2026-04-11 | Search + lifecycle guides |
| Wave 5 Audit + build | 2026-04-11 | 0 warnings, audit PASS |
| Wave 6 Deploy | 2026-04-11 | CF Pages live |
| PR merged | 2026-04-11 | feat/prd-046-docs-v0.18-deploy → dev |

---

## Stakeholders

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | gogocat | [ ] |
| Engineering Lead | Claude Code | [ ] |
| Design | N/A (reuses PRD-024 design) | [x] |
| QA | `/audit` agents | [ ] |

---

## Affected Files

- `website/src/content/docs/cli/**` — new directory, 58 files
- `website/src/content/docs/mcp/**` — expanded, 47 files
- `website/src/content/docs/changelog.mdx` — new
- `website/src/content/docs/404.md` — new
- `website/src/content/docs/guides/search-v2.md` — new
- `website/src/content/docs/guides/lifecycle-v2.md` — new
- `website/astro.config.mjs` — add `site`, possibly sidebar overrides
- `website/scripts/generate-cli-docs.mjs` — new (CLI generator)
- `website/scripts/generate-mcp-docs.mjs` — new (MCP generator)
- `website/package.json` — add `docs:regen` script

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| PROB-035 | solves | draft |
| PRD-024 | extends | active |
| EPIC-001 | parent | active |
| CHANGELOG.md | source | file |

---

> **Next step**: validate → reason (ADI) → branch already created → execute Waves 1..6.



