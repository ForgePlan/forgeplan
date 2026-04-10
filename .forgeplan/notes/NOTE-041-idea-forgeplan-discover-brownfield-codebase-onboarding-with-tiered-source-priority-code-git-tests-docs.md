---
depth: tactical
id: NOTE-041
kind: note
links:
- target: EPIC-002
  relation: informs
status: deprecated
title: 'Idea: forgeplan discover — brownfield codebase onboarding with tiered source priority (code > git > tests > docs)'
---

## Brownfield Discovery — forgeplan discover

### Проблема
При установке forgeplan на legacy проект, агент идёт в docs/ и строит knowledge base из существующей документации. Это неверно — документация может быть устаревшей, неполной, или описывать будущие планы (как миграция JS→TS), а не текущее состояние.

### Решение: Tiered Source Priority

```
Tier 1 (Source of Truth):
  - Код: файловая структура, модули, exports, типы
  - git log: история, авторы, частота изменений
  - package.json / Cargo.toml: зависимости, scripts

Tier 2 (Extracted Knowledge):
  - JSDoc / rustdoc / docstrings → API documentation
  - Тесты → поведенческая спецификация
  - CI/CD конфиги → deployment model

Tier 3 (Supplementary — legacy, may be outdated):
  - docs/ → legacy documentation
  - README → обзорная информация
  - CHANGELOG → history
```

### Команды
- `forgeplan discover` — full analysis (Tier 1 + 2 + 3)
- `forgeplan discover --source code` — only code analysis
- `forgeplan discover --source docs --tag legacy-doc` — import docs as legacy refs

### Артефакты из discover
| Source | → Artifact Kind | Tags |
|--------|----------------|------|
| File structure | PRD or Note (architecture) | auto-discovered |
| Module deps | RFC (component diagram) | auto-discovered |
| git hot spots | Problem (risk areas) | auto-discovered |
| JSDoc API | Spec | auto-discovered |
| docs/ files | Note | legacy-doc |
| Test coverage | Evidence | auto-discovered |

### Контекст
Обнаружено при использовании forgeplan на legacy JS/TS проекте. Агент выбрал путь наименьшего сопротивления (готовые docs вместо анализа кода).

### Effort: Deep — требует code parsing, git integration, language-aware analysis
### Related: RFC-002 (Graph Intelligence), forgeplan scan-import, forgeplan git-sync



