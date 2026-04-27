---
title: forgeplan_restore
description: "Restore a soft-deleted artifact from the most recent non-consumed receipt in the trash."
---

Откатывает деструктивную операцию (delete / supersede / deprecate) для конкретного
ID артефакта, читая `.forgeplan/trash/`. Пересоздаёт строку в LanceDB, возвращает
проекцию на место, восстанавливает связи там, где цели всё ещё существуют, и
переключает статус обратно с `superseded` / `deprecated`. Отказывает, если другой
артефакт с тем же ID существует на текущий момент (требуется ручное разрешение).
Receipts старше TTL (30 дней по умолчанию, ленивая чистка) восстановлению не
подлежат.

**Категория**: Lifecycle / Recovery

## Когда агент вызывает

- «Восстанови PRD-042» — пользователь заметил, что вчера был удалён не тот артефакт.
- После `forgeplan_supersede`, который агент осознаёт как ошибочный: восстановить
  оригинал.
- Точечное восстановление, когда [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/)
  откатил бы не ту операцию — передача ID точнее, чем «самое последнее».
- Аудит receipt в trash перед фиксацией реального восстановления.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для восстановления из самого свежего не-consumed receipt. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::RestoreParams`_

## Возвращает

```json
{
  "restored": "PRD-042",
  "op_reversed": "delete",
  "relations_restored": 3,
  "relations_skipped": [],
  "projection_restored": true,
  "warnings": [],
  "_next_action": "Restored `PRD-042` (reversed delete). 3 relation(s) restored. Verify with `forgeplan_get PRD-042`."
}
```

Когда некоторых целей связей больше нет:

```json
{
  "restored": "PRD-042",
  "op_reversed": "delete",
  "relations_restored": 2,
  "relations_skipped": ["EVID-099", "RFC-007"],
  "projection_restored": true,
  "warnings": [],
  "_next_action": "Restored `PRD-042` (reversed delete). 2 relation(s) restored, 2 skipped because targets no longer exist. Review with `forgeplan_get PRD-042` and re-link manually if needed."
}
```

Когда receipt не найден:

```json
{
  "ok": false,
  "error": "No non-consumed receipt found for `PRD-042`.",
  "_next_action": "Check `.forgeplan/trash/` contents or use `forgeplan_activity --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate --since 720h` to see recent destructive ops. Receipts older than 30 days are purged."
}
```

## Пример вызова

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

1. [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) — найти, когда произошла деструктивная операция.
2. `forgeplan_restore` — восстановить конкретный артефакт.
3. [`forgeplan_get`](/ru/docs/mcp/forgeplan_get/) — проверить тело, статус, связи.
4. При необходимости — вручную перевосстановить пропущенные связи.

## CLI эквивалент

[`forgeplan restore <id>`](/ru/docs/cli/) — та же семантика восстановления.

## См. также

- [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/) — откатить самую последнюю деструктивную операцию (без указания ID)
- [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) — найти receipt перед восстановлением
- [`forgeplan_delete`](/ru/docs/mcp/forgeplan_delete/) — мягкое удаление, которое это откатывает
- [Обзор MCP](/ru/docs/mcp/)
