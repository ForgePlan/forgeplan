---
title: forgeplan_estimate
description: "Оценить трудозатраты для артефакта на основе пунктов функциональных требований (FR) и этапов реализации (Phase). Возвращает разбивку по грейдам (Junior/Middle/Senior/Principal/AI) с оценкой достоверности."
---

Оценивает трудозатраты (в часах) для артефакта путем анализа его чекбоксов функциональных требований и этапов реализации, затем применяя профиль с несколькими грейдами (Junior / Middle / Senior / Principal / AI). Возвращает разбивку по грейдам с оценкой достоверности, опционально используя оценку сложности на основе LLM или ручные переопределения.

**Категория**: Качество

## Когда агент вызывает её

-   **Планирование спринта** - получить часы для каждого PRD / RFC до фиксации объема работ.
-   **Сопоставление грейдов** - посмотреть, сколько часов занимает задача для вашего собственного грейда с помощью `my_grade`.
- **Планы с помощью ИИ** - `grade: "ai"` выделяет AI-грейд в ответе, а не переключает расчёт на другой алгоритм. Настраиваемые множители грейда и типа задач — допущения модели, а не измеренная гарантия производительности.
-   **Цикл обратной связи для калибровки** - сравнить фактические и оценочные значения после спринта, чтобы настроить профиль.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для оценки. |
| `grade` | `string` | no | Выделить грейд: `junior`, `middle`, `senior`, `principal`, `ai`. Итоги по всем грейдам всё равно рассчитываются. |
| `my_grade` | `bool` | no | Автоматически определить грейд из конфигурации `grade_profile` + вывод домена артефакта. |
| `llm_score` | `bool` | no | Использовать оценку сложности на основе LLM вместо эвристики, основанной на правилах. |
| `complexity` | `string` | no | Ручные переопределения сложности, например `"FR-001=5,FR-002=3"`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::EstimateParams`_

## Возвращает

Учебный фрагмент ответа без рабочих пунктов, не измеренная оценка. Непустой ответ использует `items`, `totals`, `total_score`, `confidence`, `confidence_reasons`; достоверность относится ко всему отчёту, не к отдельным грейдам. Без настроенной LLM запрос `llm_score: true` использует эвристики. Обработчик ограничивает строку `complexity` 4096 байтами, хотя схема этого ограничения не показывает.

```json
{
  "artifact_id": "PRD-039",
  "artifact_title": "Система аутентификации",
  "items": [],
  "totals": {},
  "total_score": 0.0,
  "confidence": 0.0,
  "confidence_reasons": []
}
```

## Пример вызова

```json
{ "id": "PRD-039", "my_grade": true }
```

## Типичная последовательность

1.  `forgeplan_estimate` с `my_grade: true` - получить личные часы.
2.  Сравнить с пропускной способностью спринта - при необходимости уменьшить объем работ.
3.  После спринта: внести фактические данные и оценки обратно в конфигурацию `grade_profile`.

## Эквивалент CLI

```bash
forgeplan estimate PRD-039 --my-grade
forgeplan estimate PRD-039 --grade ai
```

## См. также

-   [`forgeplan_calibrate`](/ru/docs/mcp/forgeplan_calibrate/) - предложение глубины (используется для оценки).
-   [`forgeplan_progress`](/ru/docs/mcp/forgeplan_progress/) - фактический процент выполнения по сравнению с оценкой.
-   [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Estimate effort for an artifact based on FR and Phase items. Returns multi-grade breakdown (Junior/Middle/Senior/Principal/AI) with confidence scoring.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `complexity` | нет | string \| null | Manual complexity overrides: "FR-001=5,FR-002=3" |
| `grade` | нет | GradeKind \| null | Override grade for all items |
| `id` | да | string | Artifact ID to estimate |
| `llm_score` | нет | boolean \| null | Use LLM-based complexity scoring instead of rule-based heuristics |
| `my_grade` | нет | boolean \| null | Auto-detect grade from config grade_profile + artifact domain inference |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "GradeKind": {
      "enum": [
        "junior",
        "middle",
        "senior",
        "principal",
        "ai"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "complexity": {
      "default": null,
      "description": "Manual complexity overrides: \"FR-001=5,FR-002=3\"",
      "type": [
        "string",
        "null"
      ]
    },
    "grade": {
      "anyOf": [
        {
          "$ref": "#/$defs/GradeKind"
        },
        {
          "type": "null"
        }
      ],
      "description": "Override grade for all items"
    },
    "id": {
      "description": "Artifact ID to estimate",
      "type": "string"
    },
    "llm_score": {
      "default": null,
      "description": "Use LLM-based complexity scoring instead of rule-based heuristics",
      "type": [
        "boolean",
        "null"
      ]
    },
    "my_grade": {
      "default": null,
      "description": "Auto-detect grade from config grade_profile + artifact domain inference",
      "type": [
        "boolean",
        "null"
      ]
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "EstimateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
