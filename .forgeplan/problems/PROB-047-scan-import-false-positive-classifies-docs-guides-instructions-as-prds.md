---
depth: standard
id: PROB-047
kind: problem
last_modified_at: 2026-04-28T12:11:53.158435+00:00
last_modified_by: claude-code/2.1.121
links:
- target: PRD-058
  relation: based_on
- target: PRD-035
  relation: informs
- target: ADR-003
  relation: informs
status: draft
title: scan-import false-positive — classifies docs/guides/instructions as PRDs
---

---
created: 2026-04-28
id: PROB-047
kind: problem
title: scan-import false-positive — classifies docs/guides/instructions as PRDs
status: draft
---

# PROB-047: scan-import false-positive — classifies docs/guides/instructions as PRDs

## Context

`forgeplan scan-import` (PRD-058 brownfield migration) сканирует workspace и автоматически создаёт forge artifacts из обнаруженных markdown файлов. Текущая heuristic ошибочно классифицирует **product guides, methodology docs, и instruction files** как PRD-артефакты.

## Observed symptoms

В workspace (state на 2026-04-28) накопилось **28 PRD стабов + 1 SPEC** — все ложные срабатывания scan-import:

| Source file (real role) | Misclassified as | Duplicate count |
|---|---|---|
| `docs/methodology/FORGEPLAN-GUIDE.md` (user guide) | PRD | 7 (PRD-001/029/033/059/063/074/078) |
| `docs/methodology/FORGEPLAN-GUIDE.ru.md` (RU user guide) | PRD | 7 (PRD-021/030/036/060/064/075/079) |
| `BROWNFIELD-ORCHESTRATOR-HANDOFF-2026-04-21.ru.md` (session handoff) | PRD | 7 (PRD-027/031/037/061/072/076/080) |
| `CLAUDE.md` (agent instructions) | PRD | 7 (PRD-028/032/038/062/073/077/081) |
| `SPEC-SCHEMA.md` (schema definition doc) | SPEC | 1 (SPEC-001) |
| **Total** | | **29 false-positive artifacts** |

Health показывает их как **29 orphans + 10 duplicate pairs**. Каждый повторный запуск `scan-import` (без `--update`) создаёт новые copies — duplicate count растёт.

## Root cause hypothesis

scan-import классификатор смотрит на:
- Filename pattern (e.g., содержит `prd`, `feature`, `requirements`)
- Markdown headings (e.g., `## Goals`, `## Problem`, `## Functional Requirements`)
- YAML frontmatter `kind:` field

Файлы вроде `FORGEPLAN-GUIDE.md` содержат секции **`## Goals`** (как часть guide на тему "что такое forgeplan"), но **сами по себе** не являются PRD. Heuristic не различает:
- "документ описывает PRD-структуру" (guide / schema)
- "документ ЯВЛЯЕТСЯ PRD"

Аналогично, CLAUDE.md содержит `## Problem` как часть инструкций — попадает под heuristic.

## Impact

- **Workspace pollution**: 29 false drafts в health, blind-spot для real artifacts.
- **Idempotency violation**: повторный run множит duplicates, **противоречит ADR-003** (markdown — source of truth, scan-import должен быть idempotent).
- **Adoption blocker**: brownfield user открывает forgeplan, делает `init --scan`, получает 30+ false PRDs — сигнал "tool сломан".
- **Methodology dilution**: orphan stubs в graph traversal — noise при `forgeplan search`, `forgeplan health`, ADI reasoning.

## Proposed mitigations (separate follow-up sprint)

1. **Filename + content hybrid heuristic**: file под `docs/`, `marketplace/`, root-level (CLAUDE.md, AGENTS.md, README.md) — **never PRD**, regardless of headings.
2. **Frontmatter precedence**: если `kind:` явно указан — use it; only when absent, fall back to heuristic.
3. **scan-import default to `--dry-run` + report**: показать что будет создано, требует opt-in `--apply`.
4. **Idempotency via content_hash**: повторный run с тем же source — update existing, не create new.
5. **Test fixtures**: brownfield_test/ workspace с typical guides → assert scan-import создаёт `0` PRDs.

## Related Artifacts

- PRD-058: scan-import brownfield migration (this is the affected feature)
- PRD-035: Brownfield Discovery Engine (related architectural intent)
- ADR-003: markdown source of truth, idempotency invariant
- This sprint deprecates 29 orphan artifacts as workspace hygiene chore.




