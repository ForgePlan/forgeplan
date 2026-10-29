---
title: forgeplan_journal
description: "Показать журнал решений - хронологическую временную шкалу артефактов ADR, Note, Problem, Solution с оценками R_eff и статусом доказательств."
---

Показать журнал решений - хронологическую временную шкалу артефактов ADR, Note, Problem и Solution, каждый из которых аннотирован текущей оценкой `R_eff` и связанными доказательствами. Используйте его, чтобы восстановить, **почему** что-то было решено и **насколько сильным** это решение является в настоящее время.

**Категория**: Панели мониторинга и графики

## Когда агент вызывает его

- **Начало сессии после долгого отсутствия** - просмотр последних N решений для восстановления контекста.
- **Перед принятием связанного решения** - проверка, охватывает ли предыдущий Note / ADR / Problem уже эту тему.
- **Аудит рисков** - вызов с `risk: true` для отображения только решений со слабыми или просроченными доказательствами.
- **Онбординг** - создание повествования об истории решений проекта.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `kind` | `string` | no | Фильтр по типу: `adr`, `note`, `problem` или `solution`. Пропустите для всех. |
| `risk` | `bool` | нет | При `true` остаются записи без связанных доказательств, с `R_eff < 0.3` или просроченными учитываемыми доказательствами. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::JournalParams`_

## Возвращает

Записи отсортированы от новых к старым. Ответ содержит `entries`, `total` и подсказку. Поля записи: `id`, `kind`, `title`, `created_at`, `r_eff`, `evidence_count`, `has_stale_evidence`. ID доказательств, статус артефакта и поле `at_risk` не выдаются.

Иллюстративный фрагмент:

```json
{
  "entries": [
    {
      "id": "ADR-005",
      "kind": "adr",
      "title": "Учебное решение",
      "created_at": "2026-04-03",
      "r_eff": 0.82,
      "evidence_count": 2,
      "has_stale_evidence": false
    }
  ],
  "total": 1
}
```

Сводная подсказка считает риск по `R_eff < 0.5`, а фильтр `risk: true` использует `< 0.3`. Не подменяйте определение фильтра числом из подсказки.

## Пример вызова

```json
{ "kind": "adr", "risk": true }
```

## Типичная последовательность

1. `forgeplan_journal` с `risk: true` - список рискованных решений.
2. `forgeplan_get` для каждой записи - чтение полного содержимого.
3. `forgeplan_new` доказательство → `forgeplan_link` → `forgeplan_score` - обновление R_eff.

## Эквивалент CLI

```bash
forgeplan journal --type adr --risk
```

## См. также

- [`forgeplan_blindspots`](/ru/docs/mcp/forgeplan_blindspots/) - решения без каких-либо доказательств.
- [`forgeplan_decay`](/ru/docs/mcp/forgeplan_decay/) - деградация R_eff из-за старения доказательств.
- [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Show decision journal — chronological timeline of ADR, Note, Problem, Solution artifacts with R_eff scores and evidence status.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `kind` | нет | JournalKind \| null | Filter by artifact kind (decision-kinds only) |
| `risk` | нет | boolean \| null | Show only at-risk decisions |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "JournalKind": {
      "enum": [
        "adr",
        "note",
        "problem",
        "solution"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "kind": {
      "anyOf": [
        {
          "$ref": "#/$defs/JournalKind"
        },
        {
          "type": "null"
        }
      ],
      "description": "Filter by artifact kind (decision-kinds only)"
    },
    "risk": {
      "default": null,
      "description": "Show only at-risk decisions",
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
  "title": "JournalParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
