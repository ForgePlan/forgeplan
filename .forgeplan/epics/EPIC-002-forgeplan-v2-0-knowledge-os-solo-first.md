---
depth: standard
id: EPIC-002
kind: epic
status: active
title: ForgePlan v2.0 — Knowledge OS Solo-first
---

# EPIC-002: ForgePlan v2.0 — Knowledge OS (Solo-first)

## Vision

ForgePlan становится единым источником структурированных инженерных знаний — граф отвечает на вопросы, сессия восстанавливается одной командой, документация генерируется из артефактов.

## Outcomes

| ID | Outcome | Metric | Target |
|---|---|---|---|
| O1 | Граф отвечает на вопросы про зависимости | forgeplan impact X возвращает chain | Chain >= 3 уровней |
| O2 | Контекст восстанавливается мгновенно | forgeplan session время выполнения | < 5 секунд |
| O3 | Проект генерирует отчёты из артефактов | forgeplan generate-docs создаёт STATUS.md | Содержит все active |
| O4 | Reasoning использует FPF полноценно | forgeplan reason output содержит bounded contexts | Есть в каждом reason |
| O5 | CI защищает pipeline | GitHub Actions gate на forgeplan health | Blind spots = 0 для merge |

## Children

| Artifact | Title | Status | Phase |
|---|---|---|---|
| RFC-001 | FPF Engine v2 | draft | Phase 1 |
| RFC-002 | Graph Intelligence | draft | Phase 2 |
| NOTE-026 | CI/CD Linter | draft | Phase 1 |
| PRD-new | Session Command | planned | Phase 2 |
| PRD-new | Generate-docs | planned | Phase 2 |
| PRD-new | Built-in Memory | planned | Phase 3 |
| PRD-new | Diff command | planned | Phase 3 |
| PRD-new | CLI UX Polish | planned | Phase 3 |

Already done (v1.0):

| Artifact | Title | Status |
|---|---|---|
| PRD-022 | AI Estimation Engine | active |
| PRD-024 | Website and Docs Portal | active |
| PRD-026 | Docs Reorg and Files-First Tracking | active |
| ADR-003 | Markdown as source of truth | active |

## Phases

### Phase 1: Architecture Foundation (Sprint 11)

- RFC-001 FPF Engine v2
- NOTE-026 to PRD CI Linter
- EPIC-002 shaped and activated

### Phase 2: Graph and Session (Sprint 12)

- RFC-002 Graph Intelligence
- PRD Session Command
- PRD Generate-docs
- Website deploy to forgeplan.dev

### Phase 3: Polish and Release (Sprint 13-14)

- Built-in Memory fallback
- Diff command
- CLI UX Polish (links, doctor, --ci)
- Release v2.0.0

## Progress

```
Phase 1      ░░░░░░░░░░░░░░░░░░░░░░░░  0/3   (  0%)
Phase 2      ░░░░░░░░░░░░░░░░░░░░░░░░  0/4   (  0%)
Phase 3      ░░░░░░░░░░░░░░░░░░░░░░░░  0/4   (  0%)
─────────────────────────────────────────────────
TOTAL                                   0/11  (  0%)
```

## Non-Goals (v2.0)

- Multi-agent orchestration (Claude Code already does this)
- Task management and Linear/Jira sync (SPRINTS.md works)
- Replace Hindsight/Mem0 (complement, not replace)
- VS Code extension
- Dashboard TUI
- Watch v2 (file watcher)
- Nx Monorepo (not blocking)

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Scope creep via v2.0 label | High | Strict Non-Goals list |
| Graph queries slow on 1000+ artifacts | Medium | Benchmark before merge |
| FPF Engine v2 too abstract | Medium | Concrete examples in RFC-001 |

## Evidence

ADI reasoning (2026-04-06): H3 Solo-first (confidence 0.8) selected over H2 Hub-and-spoke (0.5) and H1 All-in-one (0.2). Memory research: 22 alternatives reviewed. ForgePlan is the structured memory, external providers handle chronicle.

## Related Artifacts

| Artifact | Relation | Status |
|---|---|---|
| EPIC-001 | Predecessor | active |
| ADR-003 | Informs | active |
| RFC-003 | Informs | active |

