---
depth: tactical
id: NOTE-026
kind: note
links:
- target: EPIC-001
  relation: informs
status: draft
title: 'Idea: CI/CD Architecture Linter — validate/health as pipeline gates'
---

## CI/CD Architecture Linter

**R_eff: MEDIUM-HIGH** — быстрая победа.

### Тезис
Forgeplan как обязательный шаг в CI pipeline — архитектурные решения проверяются как код.

### Что нужно
1. forgeplan validate --ci — exit 1 если MUST-ошибки
2. forgeplan health --fail-on blind_spots>5 — configurable thresholds
3. GitHub Action: uses: forgeplan/action@v1

### Effort: 1-2 дня на --ci flag, 1 неделя на GitHub Action

