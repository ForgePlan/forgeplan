---
depth: tactical
id: PROB-019
kind: problem
links:
- target: EPIC-001
  relation: informs
status: deprecated
title: Self-link allowed — potential graph cycles
---

## Signal

E2E тест 3.21: `forgeplan link PRD-001 PRD-001` — self-link разрешён (exit 0). Артефакт ссылается сам на себя. Потенциально ведёт к:
- Бесконечным циклам в graph traversal (order, blocked, tree)
- Некорректным подсчётам в R_eff recursive chain
- Путанице пользователя

## Constraints
- Не должен ломать существующие связи (backward compatible)
- graph/order/blocked должны быть cycle-safe уже сейчас (petgraph)

## Optimization Targets
- Предотвратить бессмысленные self-links при создании

## Observation Indicators (Anti-Goodhart)
- Количество реальных self-links в dogfood workspace (вероятно 0)

## Acceptance Criteria
- [ ] `forgeplan link PRD-001 PRD-001` → exit 1 + 'Self-link not allowed'
- [ ] Или: warning + разрешить (если есть use case)
- [ ] Тест для выбранного поведения

## Blast Radius
- link.rs (add_link), link CLI command
- Минимальный

## Reversibility
High





