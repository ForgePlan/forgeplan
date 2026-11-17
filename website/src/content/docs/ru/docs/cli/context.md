---
title: forgeplan context
description: "Возвращает сводку CLI об артефакте: связи, количество замечаний валидации, качество F-G-R и вычисленный R_eff. Полное содержимое запрашивайте через get. В Forgeplan 0.38.0 инструмента MCP forgeplan_context нет."
---

Возвращает сводку CLI об артефакте: связи, количество замечаний валидации, качество F-G-R и вычисленный R_eff. Полное содержимое запрашивайте через `get`. В Forgeplan 0.38.0 инструмента MCP `forgeplan_context` нет.

## Использование

```text
forgeplan context [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Artifact ID
```

## Опции

```text
      --json     Output as JSON for machine consumption (primary mode for AI agents)
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

```bash
forgeplan context PRD-001
forgeplan context PRD-001 --json | jq '.artifact.r_eff, .validation, .graph.evidence'
```

## Контракт JSON

Поля: `artifact` (id, kind, status, title, depth, r_eff, created_at, updated_at), `derived_status`, `graph` (parent, evidence, depends_on, dependents, related), `validation` (passed, must_errors, should_warnings), `fgr` (formality, granularity, reliability, grade), `suggestions`, `_next_action`, `hints`. Тела артефакта, объекта `scoring` и массива `memory` нет.

## Как использовать результат

Проверьте `validation.passed` и счётчики ошибок, прочитайте доказательства через `get`, проверьте связи. Для пространства используйте `health`, для вкладов доказательств — `score --json`. Эта сводка CLI не доказывает, что прочитал другой инструмент или агент.

## См. также

- [`forgeplan get`](/ru/docs/cli/get/) - более лёгкое подмножество (только артефакт)
- [`forgeplan validate`](/ru/docs/cli/validate/) - подмножество валидации
- [`forgeplan score`](/ru/docs/cli/score/) - подмножество скоринга
- [`forgeplan reason`](/ru/docs/cli/reason/) - использует контекст для анализа ADI
- [`forgeplan health`](/ru/docs/cli/health/) - панель мониторинга по всему проекту
