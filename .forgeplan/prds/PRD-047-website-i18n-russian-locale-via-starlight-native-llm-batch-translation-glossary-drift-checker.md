---
depth: standard
id: PRD-047
kind: prd
links:
- target: PROB-036
  relation: based_on
- target: PRD-046
  relation: based_on
status: active
title: Website i18n — Russian locale via Starlight native, LLM batch translation, glossary, drift checker
---

# PRD-047: Website i18n — Russian locale

## Progress

```
Phase 1 Shape    ░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)
Phase 2 Infra    ░░░░░░░░░░░░░░░░░░░░░░░░  0/4  (  0%)
Phase 3 Translate░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)
Phase 4 Landing  ░░░░░░░░░░░░░░░░░░░░░░░░  0/2  (  0%)
Phase 5 QA       ░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)
─────────────────────────────────────────────────
TOTAL                                      0/15 (  0%)
```

---

## Problem

**Кому плохо**: RU-говорящие разработчики и студенты курса. **Что происходит**: docs portal (147 pages, PRD-046) полностью на EN. Primary audience — CIS developers и студенты курса по Forgeplan/BMAD/FPF — должен читать техническую методологию на втором языке. CLAUDE.md в repo на русском, CLI output и docs на EN — disconnect.

**Impact**: студенты курса (`.local/course-material/forgeplan-course-brief.md`) получат EN docs, хотя весь курс ведётся на RU. Conversion drop при EN-only docs для RU audience estimated 30-50% (industry benchmark для developer tools).

Полная деталь: `PROB-036`.

## Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Course student | Изучает Forgeplan по курсу на RU | EN docs = cognitive overhead при lab assignments |
| CIS developer | Оценивает Forgeplan для проекта | RU docs повышает trust и adoption |
| Contributor | RU-speaking, хочет внести PR | RU methodology docs = lower barrier |

## Goals

| ID | Criterion | Metric | Target |
|----|-----------|--------|--------|
| SC-1 | RU pages count | files in `ru/docs/` | ≥ 147 (1:1 с EN) |
| SC-2 | Build passes | `npm run build` exit 0 | ~294 pages, <15s |
| SC-3 | EN URLs unchanged | `/docs/cli/init/` resolves | 200 OK |
| SC-4 | RU URLs work | `/ru/docs/cli/init/` resolves | 200 OK |
| SC-5 | Language switcher | Starlight built-in visible | click toggles EN↔RU |
| SC-6 | Pagefind | search indexes both locales | RU query returns RU page |
| SC-7 | Glossary consistency | 30+ terms canonicalized | glossary file exists |
| SC-8 | P0 manual review | getting-started + methodology pages | human-verified |

## Non-Goals

- Multi-language beyond EN+RU (no Chinese, Spanish, etc. — future)
- Translating CLI output (stays English — Rust binary)
- Translating CHANGELOG (EN-only, technical release notes)
- Machine-translated disclaimer badges (user explicitly declined)
- SSR / dynamic language detection (static site, manual switcher)
- Translating code blocks inside docs (they stay English)

---

## Functional Requirements

| ID | Priority | Requirement | Journey |
|----|----------|-------------|---------|
| FR-001 | Must | Reader can switch between EN and RU via Starlight language selector | Student browses docs |
| FR-002 | Must | EN pages served at root `/docs/...` without prefix (root locale) | Existing user visits bookmark |
| FR-003 | Must | RU pages served at `/ru/docs/...` with full sidebar | Student reads in RU |
| FR-004 | Must | Sidebar labels translated to Russian for all sections | Student navigates |
| FR-005 | Must | Glossary file maps 30+ technical terms to canonical RU equivalents | Translation consistency |
| FR-006 | Must | Translation script can batch-translate .md files using LLM API preserving frontmatter/code/tables | Maintainer runs regen |
| FR-007 | Must | Generator scripts (CLI/MCP/CHANGELOG) support `--lang ru` flag | Maintainer runs regen |
| FR-008 | Should | Landing page strings extracted into locale JSON files with RU translation | RU visitor sees landing in RU |
| FR-009 | Should | Drift checker script reports EN pages modified since last RU translation | Maintainer checks freshness |
| FR-010 | Could | Starlight i18n collection configured for custom UI label overrides | Future UX polish |

---

## Technical Approach (from ADI)

**Starlight root locale pattern** (Context7 verified):
```js
locales: {
  root: { label: 'English', lang: 'en' },
  ru: { label: 'Русский', lang: 'ru' },
}
```
- EN files stay in `src/content/docs/docs/` (current location, zero restructuring)
- RU files go to `src/content/docs/ru/docs/`
- `content.config.ts` adds `i18nLoader()` + `i18nSchema()`
- URLs: EN = `/docs/cli/init/`, RU = `/ru/docs/cli/init/`
- Fallback: missing RU page → shows EN with Starlight translation notice

**Translation**: Claude Haiku 4.5 batch via API. Glossary injected into system prompt. ~$3-5 for 147 pages.

**No disclaimer badges** per user decision — translated pages look native.

---

## Dependencies

| Dependency | Type | Status |
|-----------|------|--------|
| PRD-046 (147 EN pages) | Internal | Merged (PR #170) |
| Starlight 0.38.2 i18n | External | Supported |
| Claude API key (Haiku) | External | Available |

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| LLM breaks markdown structure | High | Post-translate build verify + markdown lint |
| Glossary drift between pages | Medium | Glossary injected in every translate call |
| Build time doubles (294 pages) | Medium | Profile; Starlight handles well per docs |
| RU content goes stale after EN update | High | Drift checker script + CI integration |

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-036 | based_on |
| PRD-046 | based_on |
| PRD-024 | based_on |



