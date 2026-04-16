---
depth: standard
id: NOTE-014
kind: note
links:
- target: PROB-012
  relation: informs
status: deprecated
title: Usability testing findings — route gaps, hook bug, agent compliance
---

## Usability Testing Results (2026-03-25)

### Тесты проведены
1. Tactical task flow — PASS
2. Standard task flow — PARTIAL (route gap)
3. All 5 hooks — PASS (1 bug fixed: pr-todo integer parse)
4. Full cycle from шпаргалка — PASS
5. Documentation coverage — PASS (Chapter 8 added)

### Проблемы найдены
1. Route не триггерит Standard для 'new command/feature' — нужны новые keywords
2. pr-todo-check.sh имел баг: пустой grep давал '0\n0' → integer expression error (пофиксил)
3. Агент НЕ соблюдает методологию автоматически — hooks ловят только commit/PR/edit, не решение пропустить PRD
4. Duplicate notes: NOTE-004 = NOTE-005

### Вывод
Системой можно пользоваться. Основной цикл работает. Главный gap — enforcement на уровне phase transitions (PRD-019 Layer 3).


