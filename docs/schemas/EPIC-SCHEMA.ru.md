[English](EPIC-SCHEMA.md) · [Русский](EPIC-SCHEMA.ru.md)

# EPIC Schema — Strategic Initiative

## Когда создавать Epic

- Инициатива > 2 RFC
- Кросс-командная работа
- Roadmap item на квартал
- Decompose monolith (несколько сервисов)
- Миграция (framework, DB, cloud)

**Правило**: Epic = контейнер. Если у тебя 1 PRD и 1 RFC — Epic не нужен.

## Обязательные секции

| # | Секция | Обязательно? | Валидация |
|---|--------|-------------|-----------|
| 1 | **Meta Header** | ✅ MUST | Status, Owner, Created, Target (quarter) |
| 2 | **Vision** | ✅ MUST | 1 предложение — что хотим достичь |
| 3 | **Outcomes** | ✅ MUST | ≥ 2 measurable outcomes |
| 4 | **Children Table** | ✅ MUST | PRD/RFC/ADR list with status |
| 5 | **Phases** | ✅ MUST | ≥ 1 фаза с артефактами |
| 6 | **Progress** | ✅ MUST | Aggregated progress bars |
| 7 | **Dependency Graph** | SHOULD | Mermaid diagram |
| 8 | **Risks** | SHOULD | ≥ 1 risk |

## Aggregated Progress

Epic progress = сумма progress из children (PRD, RFC):

```
PRD-001  ████████████████████████  8/8   (100%) DONE
RFC-042  ██████████████░░░░░░░░░░  7/12  ( 58%)
RFC-043  ░░░░░░░░░░░░░░░░░░░░░░░░  0/6   (  0%)
─────────────────────────────────────────────────
TOTAL                              15/26 (57.7%)
```

## Status Lifecycle

```
Draft → Active → Done → Archived
          ↓
      Cancelled (with reason)
```

## Numbering

| Format | Example |
|--------|---------|
| ID | `EPIC-NNN` |
| File | `EPIC-{NNN}-{kebab-case}.md` |
| Path | `docs/epics/EPIC-003-auth-system.md` |
