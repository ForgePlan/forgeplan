---
title: forgeplan_undo_last
description: "Reverse the most recent destructive operation (delete / supersede / deprecate) — undo button for AI agents."
---

Идёт по trash от свежего к старому, находит самый свежий не-consumed receipt в
пределах `within_hours` и применяет ту же логику восстановления, что и
[`forgeplan_restore`](/ru/docs/mcp/forgeplan_restore/). Используйте, когда агент
понимает «последнее, что я сделал, было неправильным», без необходимости знать ID
артефакта. Возвращает ошибку с подсказкой, если подходящего receipt нет — никогда
не угадывает.

**Категория**: Lifecycle / Recovery

## Когда агент вызывает

- Сразу после ошибочного `forgeplan_delete` / `_supersede` / `_deprecate`.
- Пользователь говорит «отмени это», не указывая, какой артефакт.
- Восстановление после галлюцинации LLM, которая совершила деструктивное действие.
- В паре с [`forgeplan_activity_stats`](/ru/docs/mcp/forgeplan_activity_stats/) —
  увидели неожиданный деструктивный вызов, откатываете его.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `within_hours` | `number` | no (default 24, max 720) | Временное окно для поиска последней деструктивной операции. Расширьте до 720 (30 дней) при сомнениях. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::UndoLastParams`_

## Возвращает

```json
{
  "restored": "PRD-042",
  "op_reversed": "delete",
  "receipt_id": "trash-2026-04-26T10-14-22-001",
  "relations_restored": 3,
  "relations_skipped": [],
  "projection_restored": true,
  "warnings": [],
  "_next_action": "Reversed most recent delete of `PRD-042`. To undo another, call `forgeplan_undo_last` again (finds the next newest non-consumed receipt). Or restore a specific ID: `forgeplan_restore <id>`."
}
```

Когда в окне нечего откатывать:

```json
{
  "ok": false,
  "error": "No non-consumed destructive op in the last 24 hour(s).",
  "_next_action": "Expand the window: `forgeplan_undo_last within_hours=720`. Or inspect the log: `forgeplan_activity --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate --since 720h`."
}
```

## Пример вызова

Дефолтное окно 24 ч:

```json
{}
```

Более широкий поиск после периода простоя:

```json
{ "within_hours": 720 }
```

## Типичная последовательность

1. Происходит осечка (`forgeplan_delete`, `_supersede` или `_deprecate`).
2. `forgeplan_undo_last` — откатить.
3. Повторить вызов, чтобы откатить предыдущую операцию (каждый вызов потребляет
   самый свежий не-consumed receipt).
4. Или переключиться на [`forgeplan_restore <id>`](/ru/docs/mcp/forgeplan_restore/),
   когда конкретный ID известен.

## CLI эквивалент

[`forgeplan undo`](/ru/docs/cli/) — та же логика обхода trash.

## См. также

- [`forgeplan_restore`](/ru/docs/mcp/forgeplan_restore/) — восстановить конкретный артефакт по ID
- [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) — изучить таймлайн деструктивных операций
- [`forgeplan_delete`](/ru/docs/mcp/forgeplan_delete/) — мягкое удаление, которое это откатывает
- [Обзор MCP](/ru/docs/mcp/)
