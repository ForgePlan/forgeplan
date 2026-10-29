---
title: forgeplan_discover_complete
description: "Завершает сессию исследования и возвращает счётчики сохранённых наблюдений и ID артефактов. Health и синтез предложений — отдельные действия агента."
---

Завершает сессию, созданную через `forgeplan_discover_start`. Выбранная реализация загружает сессию, помечает её завершённой, сохраняет и возвращает счётчики и ID созданных артефактов. Она **не** запускает health и не генерирует предложения, несмотря на описание runtime ниже.

**Категория**: Исследование существующей системы

## Когда агент вызывает эту функцию

- После прохождения всех семи фаз протокола и выдачи всех результатов.
- Когда пользователь говорит «завершить исследование» / «предложить следующие шаги» / «завершить сканирование».
- Перед планированием следующих работ: перечитать наблюдения и самостоятельно подготовить предложения.

## Что она делает

1. Загружает сессию и сохранённые в ней наблюдения.
2. Устанавливает `session_status: completed` и сохраняет сессию.
3. Возвращает `total_findings`, `phase_counts`, `tier_counts`, `artifacts_created`, `completed_at`.

## Health и синтез предложений выполняются отдельно

При необходимости отдельно вызовите `forgeplan_health`. Затем агент может предложить PROB для риска, PRD для потребности или RFC для реализации. Создавайте через `forgeplan_new` только согласованные и заполненные артефакты.

Завершение сессии не активирует созданные черновики. Текущий обработчик наблюдений также не отклоняет запись лишь потому, что сессия завершена: это отметка процесса, а не запрет записи.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `session_id` | `string` | да | Идентификатор существующей сессии из `forgeplan_discover_start`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::DiscoverCompleteParams`_

## Возвращает

Сводка сохранённой сессии. Иллюстративный фрагмент:

```json
{
  "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z",
  "session_status": "completed",
  "total_findings": 2,
  "phase_counts": { "code": 1, "docs": 1 },
  "tier_counts": { "1": 1, "3": 1 },
  "artifacts_created": ["NOTE-087", "PROB-020"]
}
```

Устаревающее поле `status` также описывает сессию, не артефакты. Используйте `session_status`, а состояние каждого артефакта проверяйте отдельно.

## Пример вызова

```json
{ "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z" }
```

## Типичная последовательность

```
discover_start → …many discover_finding calls… → discover_complete
                                                    ↓ агент читает наблюдения; health при необходимости отдельно
                                                 forgeplan_new (согласованные последующие артефакты)
                                                    ↓
                                                 forgeplan_validate / forgeplan_reason …
```

## Эквивалент CLI

- [`forgeplan discover complete`](/ru/docs/cli/discover-complete/) - та же финализация из терминала.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_discover_start`](/ru/docs/mcp/forgeplan_discover_start/) - начать сессию
- [`forgeplan_discover_finding`](/ru/docs/mcp/forgeplan_discover_finding/) - сообщать наблюдения
- [`forgeplan_health`](/ru/docs/mcp/forgeplan_health/) — отдельная проверка после исследования
- [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) - перенести предложение в реальный артефакт

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Complete a discovery session. Generates a summary report with findings per phase/tier, runs forgeplan health, and marks the session as completed.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `session_id` | да | string | Session ID to complete |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "session_id": {
      "description": "Session ID to complete",
      "type": "string"
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
    "session_id"
  ],
  "title": "DiscoverCompleteParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
