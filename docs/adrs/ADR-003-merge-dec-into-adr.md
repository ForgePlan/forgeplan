---
id: ADR-003
title: "DecisionRecord объединён в ADR"
status: Accepted
depth: deep
valid_until: 2027-03-21
problem_ref: ""
created: 2026-03-21
updated: 2026-03-21
---

# ADR-003: DecisionRecord объединён в ADR

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Context

В методологии было два типа артефактов для записи решений: ADR (Architecture Decision Record, лёгкий) и DecisionRecord/DDR (из Quint-code, тяжёлый с invariants, rollback, valid_until). Overlap между ними составляет ~70%. Adversarial review (BMAD) показал что пользователи не смогут определить когда какой использовать -- дополнительный decision point без ясных критериев.

## Decision

Объединить в один тип **ADR**. Depth определяет полноту:
- **Tactical**: Context + Decision + Alternatives + Consequences
- **Standard+**: + Invariants + Evidence Requirements + valid_until
- **Deep+**: + Pre/Post-conditions + Rollback Plan + Affected Files + Weakest Link

**Selected**: Единый ADR с depth calibration

**Why Selected**: Устраняет 70% overlap, убирает лишний decision point для пользователя, один шаблон и один parser в CLI. Depth calibration из Quint-code methodology естественно решает вопрос "сколько деталей писать".

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| Единый ADR с depth calibration | **Chosen** | 10 типов вместо 11, один шаблон, один parser, depth решает полноту |
| Разделить чётко: ADR для архитектуры, DEC для операционных контрактов | Rejected | 70% overlap, дополнительный decision point для пользователя, два parser'а в CLI |
| Убрать DEC, DDR-поля в RFC | Rejected | Category error (FPF A.7) -- rollback это про решение, не про план реализации |

## Consequences

### Positive
- 10 типов артефактов вместо 11 -- проще mental model
- Один шаблон `templates/adr/_TEMPLATE.md` вместо двух
- Один parser в forgeplan-core для ADR
- Depth calibration естественно масштабирует детализацию

### Negative (trade-offs)
- ADR template сложнее стандартного ADR в индустрии (секции deep+ непривычны)
- Документация и обучение должны объяснять depth levels

### Risks
- Пользователи привыкшие к лёгким ADR (adr-tools, MADR) могут быть удивлены rollback plan секциями

<!-- Depth: standard+ — обязательно для standard, deep, critical -->

## Invariants

- При depth deep+ секции Invariants и Rollback ОБЯЗАТЕЛЬНЫ
- При depth tactical секции deep+ не показываются в шаблоне
- DecisionRecord НЕ существует как отдельный ArtifactKind

## Evidence Requirements

- Пользователи корректно выбирают depth level в 80%+ случаев
- Время создания tactical ADR < 5 минут
- Время создания deep ADR < 15 минут

## Valid Until

**Дата**: 2027-03-21

**Обоснование TTL**: 1 год -- достаточно чтобы собрать обратную связь от пользователей по удобству depth calibration.

**Refresh Triggers** (когда пере-оценить досрочно):
- >50% пользователей путают depth levels
- Появятся use cases где ADR и DEC семантически не совместимы
- Community feedback потребует разделения

<!-- Depth: deep+ — обязательно для deep, critical -->

## Pre-conditions (чеклист ДО реализации)

- [ ] ADR template переписан с depth-conditional секциями
- [ ] DecisionRecord удалён из ArtifactKind enum
- [ ] templates/decision/ удалён

## Post-conditions (Definition of Done)

- [ ] `forgeplan new adr --depth deep` создаёт полный DDR
- [ ] `forgeplan new adr --depth tactical` создаёт лёгкий ADR
- [ ] Все тесты проходят с 10 artifact kinds
- [ ] CLAUDE.md обновлён: 10 типов вместо 11

## Admissibility

- NOT: нельзя создавать DecisionRecord как отдельный тип
- NOT: нельзя убирать DDR-поля (invariants, rollback, valid_until) из deep+ ADR
- NOT: нельзя показывать deep+ секции при depth tactical

## Rollback Plan

**Triggers** (когда откатывать):
- >50% пользователей жалуются что ADR слишком тяжёлый
- Depth calibration не решает проблему "сколько писать"

**Steps** (шаги отката):
1. Восстановить DecisionRecord в ArtifactKind enum
2. Создать templates/decision/_TEMPLATE.md из git history
3. Обновить CLAUDE.md: 11 типов
4. Добавить decision tree в PRD-RFC-ADR-FLOW.md: "когда ADR, когда DEC"

**Blast Radius**: все ADR шаблоны, ArtifactKind enum, CLAUDE.md, GLOSSARY.md

## Weakest Link

Пользователи привыкшие к лёгким ADR (adr-tools, MADR) будут путаться видя rollback plan и invariants секции. Mitigation: depth-conditional visibility -- tactical depth показывает только базовые секции, deep+ секции скрыты за HTML-комментариями или не генерируются в CLI.

## Affected Files

| File | Change |
|------|--------|
| crates/forgeplan-core/src/artifact/types.rs | DecisionRecord removed из ArtifactKind enum |
| templates/adr/_TEMPLATE.md | Переписан с depth-conditional секциями |
| templates/decision/ | Удалён полностью |
| CLAUDE.md | 11 -> 10 типов артефактов |
| docs/guides/GLOSSARY.md | Lifecycle updated, DecisionRecord убран |

<!-- /Depth: deep+ -->

## AI Guidance

> Правила для AI-агентов при работе с этим решением.

- DecisionRecord НЕ существует -- всегда используй ADR
- При создании ADR спрашивай depth level если не указан
- Для tactical ADR не генерируй секции deep+ (Pre/Post-conditions, Rollback, Affected Files, Weakest Link)
- Если задача конфликтует с этим ADR, явно указать на конфликт

## Implementation Plan

### Phase 0: Foundation
- [ ] **0.1** Удалить DecisionRecord из ArtifactKind enum в types.rs
- [ ] **0.2** Удалить templates/decision/ директорию

### Phase 1: Core
- [ ] **1.1** Обновить ADR template с depth-conditional секциями
- [ ] **1.2** Реализовать depth-aware ADR generator в CLI
- [ ] **1.3** Обновить CLAUDE.md и GLOSSARY.md

## Implementation Log

<!-- Add wave entries as sprints are completed:

### Wave 1 — YYYY-MM-DD
| Task | Teammate | Status | Files |
|------|----------|--------|-------|
| 0.1 | ... | Done | ... |
-->

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| EPIC-001 | Epic | based_on |
| ADR-001 | ADR | related |
| ADR-002 | ADR | related |
