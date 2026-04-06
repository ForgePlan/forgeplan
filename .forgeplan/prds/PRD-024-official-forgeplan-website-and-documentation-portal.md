---
depth: standard
id: PRD-024
kind: prd
links:
- target: EPIC-001
  relation: refines
status: active
title: Official Forgeplan Website and Documentation Portal
---

---
id: PRD-024
title: "Official Forgeplan Website and Documentation Portal"
status: Draft
author: gogocat
created: 2026-04-05
updated: 2026-04-05
epic: EPIC-001
priority: P1
depth: deep
domain: general
projectType: web_app
stepsCompleted: []
---

# PRD-024: Official Forgeplan Website and Documentation Portal

## Progress

```
Phase 1  ░░░░░░░░░░░░░░░░░░░░░░░░  0/8   (  0%)  Landing
Phase 2  ░░░░░░░░░░░░░░░░░░░░░░░░  0/6   (  0%)  Docs Portal
Phase 3  ░░░░░░░░░░░░░░░░░░░░░░░░  0/4   (  0%)  Animations & Polish
─────────────────────────────────────────────────
TOTAL                               0/18  (  0%)
```

---

## Executive Summary

### Vision

Официальный сайт Forgeplan — единая точка входа (landing + docs) в forge-эстетике (ч/б, геометрические линии, полигональность, минимализм), который одновременно служит базой визуального языка для TUI (ratatui) desktop-приложения.

### Problem

Forgeplan v0.7.0 — зрелый CLI-инструмент с 33 командами, 28 MCP tools и более чем 225 тестами, но у него полностью отсутствует публичное присутствие в интернете. Потенциальный пользователь, узнавший о Forgeplan из рекомендации, Twitter или Hacker News, сталкивается с тремя критическими барьерами: невозможно понять что это за инструмент за 10 секунд без landing page, невозможно найти структурированную документацию для установки и использования без docs portal, и невозможно увидеть CLI в действии без демо или скриншотов. README на GitHub — единственная точка входа, и этого категорически недостаточно для инструмента со своей собственной методологией, включающей 10 типов артефактов, lifecycle state machine, quality scoring и evidence tracking.

**Impact**: 0 external users, 0 community contributions — продукт невидим без сайта. Конкуренты (astral.sh, charm.sh) имеют сайты с docs, что даёт им преимущество в discoverability.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Developer (потенциальный пользователь) | Разработчик, узнавший о Forgeplan из Twitter/HN/рекомендации | Не может за 30 сек понять что это и стоит ли пробовать |
| Current user | Уже установил Forgeplan, использует CLI | Нет structured docs — только CLAUDE.md и README |
| AI Agent operator | Подключает Forgeplan как MCP server к Claude/GPT | Нет reference по 28 MCP tools и их параметрам |
| Contributor | Хочет контрибьютить в проект | Нет architecture docs и contribution guide |

### Differentiators

- **Единый визуальный язык** — сайт, docs и TUI разделяют одну дизайн-систему (forge palette)
- **Methodology-first docs** — документация описывает не только CLI, но и методологию (Shape→Validate→Code→Evidence)
- **Interactive CLI demo** — встроенный терминал-демо на лендинге (не скриншот)
- **Dark-first** — эстетика "кузницы": чёрный фон, геометрические линии, минимализм

---

## Success Criteria

<!-- BMAD: Каждый критерий MUST быть SMART — Specific, Measurable, Achievable, Relevant, Time-bound. -->
<!-- Запрещены формулировки: "улучшить", "повысить", "ускорить" без конкретных чисел. -->

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Landing page load | Time to First Contentful Paint | N/A | < 1.5s | Launch | Lighthouse |
| SC-2 | Docs coverage | % CLI commands documented | 0% | 100% (33/33) | Launch +2w | Manual count |
| SC-3 | Install conversion | Visitor → install attempt | 0% | > 5% | Launch +1m | Analytics |
| SC-4 | Lighthouse score | Performance + Accessibility | N/A | > 90 each | Launch | Lighthouse CI |
| SC-5 | MCP docs | % MCP tools documented | 0% | 100% (28/28) | Launch +2w | Manual count |

---

## Product Scope

### MVP (In-Scope)

- Landing page: hero + features + CLI demo + install + footer
- Docs portal: getting started, guides, CLI reference, MCP reference, methodology
- Forge design system: palette, typography, components (shared web + TUI tokens)
- SVG animations: hero morph (forge → DAG), scroll-pinned sections
- Dark/light mode toggle (dark default)
- Search in docs (Pagefind)
- Mobile responsive
- Deploy to GitHub Pages or Vercel

### Out of Scope

- Blog / changelog page (future)
- User accounts / auth
- Interactive playground (live CLI in browser)
- i18n (only Russian + English for MVP)
- E-commerce / pricing (Forgeplan is free/open-source)
- Community forum

### Growth Vision

- Blog with release notes, methodology deep-dives
- Interactive artifact editor (try Forgeplan in browser)
- Video tutorials / screencasts
- Community showcase (projects using Forgeplan)
- Localization (zh, ja, de)

---

## User Journeys

<!-- BMAD: Для каждого типа пользователя (из Target Users) описать минимум один journey. -->
<!-- Каждый journey должен иметь хотя бы один FR в секции Functional Requirements. -->

### Journey 1: Developer — First Discovery

**Цель пользователя**: Понять что такое Forgeplan и стоит ли его попробовать.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | Переходит на forgeplan.dev | Hero: bold "FORGEPLAN" + SVG animation + subtitle | 10-sec comprehension |
| 2 | Скроллит вниз | Scroll-pinned секции: What → Features → CLI Demo | Progressive disclosure |
| 3 | Видит CLI demo | Terminal window с animated typing: health → route → new prd | "Aha" moment |
| 4 | Кликает [Install] | Install section: cargo/brew/curl one-liners с copy button | Minimal friction |
| 5 | Кликает [Documentation] | Redirect to /docs/getting-started | Smooth transition |

**Результат**: Установил Forgeplan и перешёл к getting started guide.

### Journey 2: Current User — Finding CLI Reference

**Цель пользователя**: Найти документацию по конкретной команде.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | Открывает /docs | Docs sidebar: Getting Started, Guides, CLI Ref, MCP Ref | Structured nav |
| 2 | Кликает CLI Reference | List: 33 команды с кратким описанием | Searchable |
| 3 | Ищет "validate" | Pagefind: мгновенный результат → /docs/cli/validate | Client-side search |
| 4 | Читает страницу | Usage, flags, examples, related commands | Complete reference |

**Результат**: Нашёл нужную информацию за <30 секунд.

### Journey 3: AI Agent — MCP Tool Reference

**Цель пользователя**: Понять параметры MCP tool для интеграции.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | Открывает /docs/mcp-reference | List: 28 MCP tools с описанием | Structured |
| 2 | Находит нужный tool | Страница: name, description, parameters (JSON Schema), examples | API-grade docs |

**Результат**: Корректно вызывает MCP tool из AI агента.

---

## Functional Requirements

<!-- ============================================================ -->
<!-- BMAD QUALITY REMINDERS (НЕ УДАЛЯТЬ):                        -->
<!--                                                              -->
<!-- FORMAT: "[Actor] can [capability]"                            -->
<!--   OK:    "User can filter projects by status"                -->
<!--   BAD:   "Filter component renders project list"             -->
<!--                                                              -->
<!-- NO IMPLEMENTATION LEAKAGE:                                   -->
<!--   Запрещены названия технологий (React, Django, PostgreSQL,  -->
<!--   Redis, AWS, Docker, etc.) ЕСЛИ они не являются частью      -->
<!--   capability. PRD описывает ЧТО, не КАК.                    -->
<!--   OK:    "API consumer can retrieve data via REST endpoint"  -->
<!--   BAD:   "React component fetches data using Redux store"    -->
<!--                                                              -->
<!-- NO SUBJECTIVE ADJECTIVES:                                    -->
<!--   Запрещены: "быстро", "удобно", "интуитивно", "легко",     -->
<!--   "просто", "эффективно" — без конкретных метрик.            -->
<!--                                                              -->
<!-- TRACEABILITY:                                                -->
<!--   Каждый FR MUST traceably link to a User Journey.           -->
<!--   Orphan FR (без связи с journey) = validation failure.      -->
<!-- ============================================================ -->

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Landing | Must | [ ] Visitor can understand what Forgeplan is within 10 seconds of landing | J1 |
| FR-002 | Landing | Must | [ ] Visitor can see a live CLI demo showing core workflow (health, route, new) | J1 |
| FR-003 | Landing | Must | [ ] Visitor can copy a one-line install command (cargo/brew/curl) | J1 |
| FR-004 | Landing | Must | [ ] Visitor can navigate to documentation from landing page | J1 |
| FR-005 | Landing | Should | [ ] Visitor can see SVG animation (forge morph) on hero section | J1 |
| FR-006 | Landing | Should | [ ] Visitor can toggle between dark and light mode | J1 |
| FR-007 | Docs | Must | [ ] User can browse all 33 CLI commands with usage, flags, and examples | J2 |
| FR-008 | Docs | Must | [ ] User can search docs content and find results in <1 second | J2 |
| FR-009 | Docs | Must | [ ] User can navigate docs via sidebar with hierarchical structure | J2 |
| FR-010 | Docs | Must | [ ] User can read Getting Started guide (install → init → first artifact) | J2 |
| FR-011 | Docs | Must | [ ] User can read methodology guides (routing, lifecycle, scoring, evidence) | J2 |
| FR-012 | Docs | Should | [ ] User can browse all 28 MCP tools with parameters and examples | J3 |
| FR-013 | Design | Must | [ ] Site uses forge design system (palette, typography, grid, components) | J1,J2,J3 |
| FR-014 | Design | Must | [ ] Site is responsive (mobile, tablet, desktop) | J1,J2 |
| FR-015 | Perf | Must | [ ] Landing page achieves Lighthouse Performance score > 90 | J1 |
| FR-016 | Perf | Should | [ ] Docs pages load with 0 JS by default (static HTML) | J2 |
| FR-017 | Deploy | Must | [ ] Site deploys automatically on git push to main | J1,J2 |
| FR-018 | Design | Should | [ ] Design tokens are exportable for TUI (ratatui) consumption | J1 |

---

## Non-Functional Requirements

<!-- ============================================================ -->
<!-- BMAD QUALITY REMINDERS (НЕ УДАЛЯТЬ):                        -->
<!--                                                              -->
<!-- FORMAT: "System shall [metric] [condition] [measurement]"    -->
<!--   OK:    "System shall respond within 200ms at p95 under     -->
<!--           1000 concurrent users, measured by APM"            -->
<!--   BAD:   "System should be fast and responsive"              -->
<!--                                                              -->
<!-- MEASURABILITY:                                               -->
<!--   Каждый NFR MUST содержать конкретное число и метод         -->
<!--   измерения. Запрещены: "быстрый", "отзывчивый",            -->
<!--   "масштабируемый", "надёжный" без цифр.                     -->
<!--                                                              -->
<!-- TEMPLATE: criterion + metric + condition + measurement       -->
<!-- ============================================================ -->

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Landing FCP | < 1.5s | 3G throttled | Lighthouse |
| NFR-002 | Performance | Docs page size | < 100KB HTML+CSS | Per page | Build stats |
| NFR-003 | Accessibility | WCAG compliance | AA level | All pages | Lighthouse a11y > 90 |
| NFR-004 | SEO | Meta tags + OG | Complete | All public pages | Lighthouse SEO > 90 |
| NFR-005 | Bundle | Landing JS | < 150KB gzipped | Total JS payload | Build stats |

---

## Acceptance Criteria

<!-- Обязательно для depth: deep / critical. Опционально для standard. -->
<!-- Формат: Given / When / Then (Gherkin-style) -->

### AC-1: {Scenario Name}

```gherkin
Given [предусловие / начальное состояние]
When  [действие пользователя]
Then  [ожидаемый результат]
And   [дополнительный результат, если есть]
```

### AC-2: {Scenario Name}

```gherkin
Given [предусловие]
When  [действие]
Then  [результат]
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| Forgeplan CLI v0.7.0+ | Technical | Ready | gogocat |
| Design Research (DESIGN-RESEARCH-WEBSITE.md) | Reference | Done | gogocat |
| Domain name (forgeplan.dev/sh/rs) | External | Not started | gogocat |
| Astro + Starlight | Technical | Available | OSS |
| GSAP (community edition) | Technical | Available | OSS |
| Space Grotesk font | Technical | Available | Google Fonts |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | GSAP MorphSVG is paid ($199), free alternative may lack quality | Medium | Medium | Use anime.js or GSAP community morph, test visual quality first | gogocat |
| R-2 | Docs content creation is labor-intensive (33 cmds + 28 tools) | High | High | Auto-generate CLI reference from `forgeplan --help` output, manual polish | gogocat |
| R-3 | Design system may not translate well to TUI (ratatui) | Low | Medium | Define design tokens abstractly (not CSS-specific), validate with TUI prototype | gogocat |
| R-4 | Custom animations hurt Lighthouse Performance score | Medium | Medium | Use Astro islands — GSAP only on landing, 0 JS on docs pages | gogocat |

---

## Timeline

<!-- Обязательно для depth: deep / critical. -->

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-04 | Requirements locked |
| Spec Complete | 2026-04-04 | API contracts defined |
| RFC Approved | 2026-04-04 | Architecture decided |
| MVP | 2026-04-04 | Core features shipped |
| GA | 2026-04-04 | Full release |

---

## Stakeholders

<!-- Обязательно для depth: deep / critical. -->

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | | [ ] |
| Engineering Lead | | [ ] |
| Design | | [ ] |
| QA | | [ ] |

---

## Affected Files

- NEW: `website/` directory (Astro project)
- NEW: `website/src/content/docs/` (documentation content)
- REF: `docs/references/DESIGN-RESEARCH-WEBSITE.md` (design research)
- REF: `docs/guides/` (source content for docs migration)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic | active |
| DESIGN-RESEARCH-WEBSITE.md | Research input | done |
| RFC-TBD | Website architecture | pending |

---

<!-- ============================================================ -->
<!-- BMAD VALIDATION CHECKLIST (для автора и ревьюера):           -->
<!--                                                              -->
<!-- [ ] Executive Summary содержит vision + problem + users      -->
<!-- [ ] Success Criteria — все SMART с числами                   -->
<!-- [ ] Product Scope — MVP чётко отделён от out-of-scope        -->
<!-- [ ] User Journeys — минимум 1 на каждую персону              -->
<!-- [ ] FR — формат "[Actor] can [capability]", нет impl leakage -->
<!-- [ ] NFR — конкретные метрики, метод измерения                -->
<!-- [ ] Traceability — каждый FR ссылается на journey            -->
<!-- [ ] Acceptance Criteria — Given/When/Then (deep/critical)    -->
<!-- [ ] Risks — минимум 1 риск с mitigation                      -->
<!-- [ ] Related Artifacts — ссылки на SPEC/RFC/ADR если есть     -->
<!--                                                              -->
<!-- ADVERSARIAL REVIEW (BMAD):                                   -->
<!-- Ревьюер ОБЯЗАН найти минимум 1 проблему.                     -->
<!-- 0 найденных проблем = недостаточно тщательный review.        -->
<!-- ============================================================ -->

> **Next step**: После approve → создать SPEC (контракты) и/или RFC (архитектура).



