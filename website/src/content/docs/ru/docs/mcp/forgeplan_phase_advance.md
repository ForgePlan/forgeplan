---
title: forgeplan_phase_advance
description: "Manually advance (or set) the advisory phase marker for an artifact and record a transition."
---

Записывает следующую фазу в `.forgeplan/state/<id>.yaml`, дописывая неизменяемую
запись в историю с timestamp и опциональной причиной. Рекомендательный слой — **не**
валидирует порядок фаз, поэтому скачки не по порядку (например, сразу к `done` для
однострочной правки) разрешены by design. Полное обеспечение фаз появится в более
позднем PRD в рамках EPIC-005. Используйте, когда авто-продвижение пропустило
переход или при переклассификации состояния workflow.

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

Сбой (config disabled, файловая система недоступна для записи):

```json
{
  "ok": false,
  "error": "Failed to advance phase: ...",
  "_next_action": "Check `.forgeplan/state/` is writable; verify phase tracking is enabled in config.yaml (`phase.enabled: true`)."
}
```

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

1. [`forgeplan_phase`](/ru/docs/mcp/forgeplan_phase/) — прочитать текущее состояние.
2. Сделать работу для рекомендуемой следующей фазы.
3. `forgeplan_phase_advance` — записать переход.
4. Повторять, пока `current_phase: "done"`.

## CLI эквивалент

[`forgeplan phase advance <id> --to <phase>`](/ru/docs/cli/) — та же запись, та же
рекомендательная семантика.

## См. также

- [`forgeplan_phase`](/ru/docs/mcp/forgeplan_phase/) — прочитать текущее состояние + историю
- [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) — гейт активации методологии
- [Methodology guide](/ru/docs/methodology/overview/) — Shape → Validate → Code → Evidence → Activate
