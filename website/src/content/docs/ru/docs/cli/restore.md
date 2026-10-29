---
title: forgeplan restore
description: "Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Surgical recovery by explicit ID."
---

`forgeplan restore` возвращает конкретный артефакт, ранее удалённый, замещённый (`superseded`) или выведенный из эксплуатации (`deprecated`). По ID (например, `PRD-042`) команда находит последний подходящий receipt в `.forgeplan/trash/` и отменяет записанную операцию. Статус возвращается к состоянию, зафиксированному в receipt. На Constellation сначала восстанавливается канонический Markdown, затем reconciliation публикует производный индекс; изучите предупреждения и `index_pending`, не предполагая успешную публикацию. Связи с отсутствующими целями могут быть пропущены.

При восстановлении delete конфликтующая живая identity вызывает отказ.
Проверьте оба артефакта и receipt до изменений. Не переименовывайте,
не замещайте и не удаляйте живой артефакт автоматически ради восстановления.
Нужен действительный неиспользованный receipt; срок хранения по умолчанию - 30 дней.

Это точечный вариант [`forgeplan undo-last`](/ru/docs/cli/undo-last/) - `undo-last` откатывает самую свежую деструктивную операцию независимо от артефакта, а `restore` нацеливается на один конкретный артефакт по ID. Это аналог [`forgeplan_restore`](/ru/docs/mcp/forgeplan_restore/) на MCP-стороне, с одним отличием: MCP-инструмент дополнительно принимает необязательный параметр `target_status`, а CLI всегда восстанавливает статус, зафиксированный в receipt.

## Когда использовать

- Поняли, что вчерашний `forgeplan delete PRD-042` был ошибкой - восстанавливайте по точному ID.
- Решение `forgeplan supersede` оказалось неверным, и нужен именно оригинальный артефакт, а не та операция, которая случайно оказалась самой свежей.
- Нужно точное восстановление без риска откатить случайно ещё какую-то операцию, произошедшую позже.

## Когда НЕ использовать

- Не помните ID - [`forgeplan undo-last`](/ru/docs/cli/undo-last/) сам пройдёт по логу деструктивных операций от свежих к старым.
- Есть конфликтующий живой артефакт - изучите и осознанно разрешите identity-конфликт до повтора.
- Действительного receipt нет - изучите историю Git; не придумывайте lifecycle-переход для обхода recovery.

## Использование

```text
forgeplan restore [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Artifact ID to recover from the most recent non-consumed receipt
```

## Опции

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

### Пример 1: Восстановление удалённого PRD

```bash
forgeplan restore PRD-042
```

Находит самую свежую неиспользованную запись для `PRD-042`, проигрывает оригинальную операцию в обратную сторону и печатает, сколько связей восстановлено. Если какие-то связи указывали на артефакты, которых больше нет, они попадают в `relations_skipped` - будете знать, что чинить руками.

### Пример 2: Проверка после восстановления

```bash
forgeplan restore PRD-042
forgeplan get PRD-042
```

После восстановления всегда проверьте тело, статус и связи - убедитесь, что всё корректно. Если в `relations_skipped` что-то важное, пересоздайте вручную.

### Пример 3: Машинно-читаемый вывод

```bash
forgeplan restore PRD-042 --json | jq '.relations_skipped'
```

Возвращает JSON и вытаскивает только список связей, которые не удалось восстановить. Полезно в скрипте восстановления, который должен попытаться пересоздать эти связи из другого источника.

## Место в рабочем процессе

Изучите историю деструктивных операций и подходящий receipt, выполните
`forgeplan restore <ID>`, затем проверьте `forgeplan get <ID>` и связи.
Recovery отменяет записанную операцию, а не возвращает terminal-статус
обычным переходом. Наличие команды не доказывает поддержку мигрированного
workspace каждым recovery tool; см. [Constellation](/ru/docs/guides/constellation/).

## См. также

- [`forgeplan_restore`](/ru/docs/mcp/forgeplan_restore/) - MCP-эквивалент
- [`forgeplan undo-last`](/ru/docs/cli/undo-last/) - откатить самую свежую деструктивную операцию (без ID)
- [`forgeplan activity`](/ru/docs/cli/activity/) - найти receipt перед восстановлением
- [`forgeplan delete`](/ru/docs/cli/delete/) - soft-delete, который это откатывает
- [Обзор CLI](/ru/docs/cli/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan restore --help`.

```text
Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Works for delete (recreates row + moves projection back), supersede (resets status + drops link), and deprecate (resets status). Refuses if a different artifact with the same ID currently exists. TTL default: 30 days from the destructive op

Usage: forgeplan restore [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID to recover from the most recent non-consumed receipt

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
