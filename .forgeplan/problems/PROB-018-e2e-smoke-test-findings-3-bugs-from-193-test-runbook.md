---
depth: tactical
id: PROB-018
kind: problem
links:
- target: EPIC-001
  relation: refines
status: active
title: E2E Smoke Test Findings — 3 bugs from 193-test runbook
---

# PROB-018: E2E Smoke Test Findings — 3 bugs from 193-test runbook

## Signal

E2E smoke test (193 теста, 11 волн) на PetAdoptionApi выявил 3 бага при 92.7% pass rate:

1. **BUG-001 (P1 Security):** `forgeplan scan --path /tmp` сканирует произвольные директории без валидации границ проекта. При этом `scan-import --path /etc` корректно блокирует path traversal ("Path traversal rejected"). Несогласованность между `scan` и `scan-import`.

2. **BUG-002 (P2 Functional):** `forgeplan unlink A B --relation X` не проверяет существование связи перед удалением. Возвращает "Unlinked" + exit 0 даже для несуществующей связи. Скрытые ошибки в автоматизации.

3. **BUG-003 (P3 Display):** При `deprecated → active` transition, сообщение выводит "draft → active" вместо "deprecated → active". Display bug в lifecycle transition message.

Дополнительно: 5 расхождений runbook vs реальность (не баги — design decisions).

## Constraints

- BUG-001 MUST быть исправлен до v1.0 (security blocker)
- Фиксы не должны менять public API (backward compatible)
- Каждый фикс = тест, подтверждающий исправление

## Optimization Targets (1-3 макс)

- Консистентность path validation: `scan` и `scan-import` должны вести себя одинаково
- Честные exit codes: операция не выполнена → exit 1, не exit 0

## Observation Indicators (Anti-Goodhart)

- Общий pass rate E2E runbook (текущий: 92.7%, цель: >98%)
- Количество inconsistencies между похожими командами
- Время прогона smoke test (не жертвовать скоростью ради проверок)

## Acceptance Criteria

- [ ] `forgeplan scan --path /tmp` → exit 1 + "Path outside project root"
- [ ] `forgeplan scan --path /etc` → exit 1 + "Path outside project root"
- [ ] `forgeplan unlink A B --relation X` (несуществующая) → exit 1 + "Relation not found"
- [ ] `forgeplan activate ADR-001` (deprecated→active) → "deprecated → active" в сообщении
- [ ] Unit тесты для каждого фикса
- [ ] Re-run affected E2E tests: PASS

## Blast Radius

- `scan` command (scan.rs) — path validation
- `unlink` logic (link/mod.rs) — existence check
- Lifecycle transitions (lifecycle/transitions.rs) — message formatting
- Минимальный blast radius: 3 изолированных модуля

## Reversibility

High — все фиксы точечные, обратимые через git revert

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| EPIC-001 | refines |
| EVID-037 | informs (E2E verification) |


