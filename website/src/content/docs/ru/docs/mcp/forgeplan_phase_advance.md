---
title: forgeplan_phase_advance
description: "Manually advance (or set) the advisory phase marker for an artifact and record a transition."
---

Записывает рекомендательную фазу артефакта в `.forgeplan/state/<id>.yaml`. Явное исправление допускает движение вперёд, назад и сразу к `done`; оно не подтверждает выполненную работу и не обходит политику. Смена фазы добавляет временную метку и причину в ограниченную историю; повтор той же фазы не добавляет запись.

Это не автомат фаз **сессии** методологии, который проверяет `forgeplan_guard`.

**Категория**: Lifecycle (рекомендательный)

## Когда агент вызывает

- Авто-продвижение пропустило: инструмент отработал, но трекинг фаз был выключен,
  теперь его включают.
- Переклассификация: артефакт повышен из `code` в `audit` после волны PR-ревью.
- Бэкфилл: legacy-артефакт старше PRD-056, агент проводит его до `done`.
- Запись осознанного пропуска: прыжок сразу в `done` для тривиальной правки с `reason`.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для продвижения. |
| `to` | `string` | yes | Целевая фаза. Одно из `shape`, `validate`, `adi`, `code`, `test`, `audit`, `evidence`, `done`. |
| `reason` | `string` | no | Опциональное обоснование, записываемое в историю. Жёсткий лимит 4096 байт (отклоняется на границе для предотвращения DoS). |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::PhaseAdvanceParams`_

## Возвращает

Иллюстративный успешный ответ:

```json
{
  "artifact_id": "PRD-057",
  "current_phase": "test",
  "workflow_type": "greenfield",
  "advanced_at": "2026-04-26T11:00:00Z",
  "history_entries": 4,
  "reason": "FR tests green",
  "_next_action": "`PRD-057` advanced to `test`. Suggested next: `audit`."
}
```

При повреждённом state-файле или недоступной записи MCP возвращает ошибочный результат с текстом, не JSON-оболочку `ok`/`error`. Иллюстративный текст:

```text
Failed to advance phase: ...

_next_action: Check `.forgeplan/state/` is writable; verify phase tracking is enabled in config.yaml (`phase.enabled: true`).
```

Подсказка упоминает config, но явное продвижение не проверяет `phase.enabled`.

## Пример вызова

Стандартный переход:

```json
{ "id": "PRD-057", "to": "test", "reason": "FR tests green" }
```

Прыжок вперёд (рекомендательный, без validation gate):

```json
{ "id": "NOTE-019", "to": "done", "reason": "trivial typo fix" }
```

## Типичная последовательность

1. [`forgeplan_phase`](/ru/docs/mcp/forgeplan_phase/) - прочитать текущее состояние.
2. Сделать работу для рекомендуемой следующей фазы.
3. `forgeplan_phase_advance` - записать переход.
4. Повторять, пока `current_phase: "done"`.

## CLI эквивалент

[`forgeplan phase-advance <id> --to <phase>`](/ru/docs/cli/) — тот же путь явного исправления отметки.

## См. также

- [`forgeplan_phase`](/ru/docs/mcp/forgeplan_phase/) - прочитать текущее состояние + историю
- [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) - гейт активации методологии
- [Methodology guide](/ru/docs/methodology/overview/) - Shape → Validate → Code → Evidence → Activate

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Manually advance (or set) the advisory **artifact lifecycle phase** marker for an artifact (shape/validate/adi/code/test/audit/evidence/done). Appends a transition to the history. Forward and out-of-order jumps are allowed, including a direct `done` override, and so is moving BACKWARDS — this tool is the deliberate-correction path (PRD-086 FR-007). Automatic advancement triggered by other tools refuses to move a phase backwards. Full phase enforcement lands in a later PRD under EPIC-005. Use when auto-advancement missed a transition or when reclassifying workflow state. NOTE: this targets the artifact lifecycle phase machine, NOT the methodology session phase machine (idle/routing/shaping/coding/evidence/pr) — for that use `forgeplan_guard` (PROB-065).

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Artifact ID to advance |
| `reason` | нет | string \| null | Optional reason / justification (recorded in history) |
| `to` | да | PhaseArg | Target phase to advance to |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "PhaseArg": {
      "description": "JSON-Schema enum mirroring `forgeplan_core::phase::Phase`.\nLLM clients constrain-sample against this so `advance` tools\nget exact values (\"shape\", \"validate\", etc.) rather than paraphrases.",
      "enum": [
        "shape",
        "validate",
        "adi",
        "code",
        "test",
        "audit",
        "evidence",
        "done"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to advance",
      "type": "string"
    },
    "reason": {
      "default": null,
      "description": "Optional reason / justification (recorded in history)",
      "type": [
        "string",
        "null"
      ]
    },
    "to": {
      "$ref": "#/$defs/PhaseArg",
      "description": "Target phase to advance to"
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
    "id",
    "to"
  ],
  "title": "PhaseAdvanceParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
