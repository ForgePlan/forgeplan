---
title: forgeplan undo-last
description: "Reverse the most recent destructive operation (delete, supersede, or deprecate) - the undo button for AI agents. Never guesses; errors with guidance when no receipt is found."
---

`forgeplan undo-last` откатывает последнюю деструктивную операцию - `delete`, `supersede` или `deprecate` - без необходимости помнить ID артефакта. Это «кнопка отмены» после того, как агент (или вы) сделал что-то не то: артефакт возвращается, его связи восстанавливаются там, где цели ещё существуют, статус переключается обратно с `superseded` / `deprecated`.

Как это работает: каждая деструктивная операция оставляет небольшую запись («квитанцию» - receipt) в `.forgeplan/trash/`. `undo-last` находит самую свежую неиспользованную квитанцию, проигрывает её в обратную сторону и помечает использованной - поэтому повторный вызов перейдёт к следующей по свежести операции. Если в указанном окне ничего подходящего нет, команда возвращает ошибку с подсказкой - она никогда не угадывает.

Это CLI-вариант [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/) на MCP-стороне.

## Когда использовать

- Сразу после ошибочного `forgeplan delete` / `supersede` / `deprecate` - отменить и повторить правильно.
- Пользователь говорит «отмени» без ID — команда выбирает receipt, а не запись журнала; сначала убедитесь, что это нужная операция.
- Откат галлюцинации LLM, которая запустила деструктивное действие.
- Для инспекции вызовов используйте [`forgeplan activity`](/ru/docs/cli/activity/); агрегат `activity-stats` не является журналом отдельных операций.

## Когда НЕ использовать

- Знаете точный ID - [`forgeplan restore <ID>`](/ru/docs/cli/restore/) точнее (нет риска откатить не ту операцию).
- Истёк TTL квитанции (по умолчанию 30 дней): восстановление может быть отклонено. Это не гарантия физического удаления файла ровно через 30 дней.
- Опечатку или неверный заголовок исправляйте через `update` либо разрешённую правку Markdown с последующим `index`/`reindex`, не через устаревший `scan-import`.

## Использование

```text
forgeplan undo-last [OPTIONS]
```

## Опции

```text
      --within-hours <WITHIN_HOURS>  Time window (hours) to search for the last destructive op (1..=720, default 24) [default: 24]
      --json                         Output as JSON for machine consumption
  -h, --help                         Print help
  -V, --version                      Print version
```

## Примеры

### Пример 1: Откат за 24 часа по умолчанию

```bash
forgeplan undo-last
```

Откатывает самую свежую деструктивную операцию из последних 24 часов. Каждый вызов использует одну квитанцию - поэтому три вызова подряд откатят последние три операции в обратном порядке (от самой свежей).

### Пример 2: Расширенный поиск после паузы

```bash
forgeplan undo-last --within-hours 720
```

Ищет по всему 30-дневному окну. Используйте, когда возвращаетесь в воркспейс через несколько дней и поиск по умолчанию (24 часа) ничего не находит.

### Пример 3: Машинно-читаемый вывод для скриптов

```bash
forgeplan undo-last --json | jq '.restored, .op_reversed'
```

Возвращает JSON, потом вытаскивает ID восстановленного артефакта и тип откатанной операции. Полезно, когда `undo-last` - часть скрипта восстановления, которому нужно залогировать что именно сделано.

Проверяйте также предупреждения, `index_pending` и `relations_skipped`: восстановление источника не гарантирует немедленную публикацию индекса или восстановление каждой связи.

## Место в рабочем процессе

Используется после ошибочной деструктивной операции (`delete`, `supersede` или `deprecate`). Запустите `undo-last`, чтобы откатить самую свежую; повторите вызов, чтобы откатить более ранние по порядку. Как только знаете конкретный ID, переключайтесь на [`forgeplan restore <ID>`](/ru/docs/cli/restore/) - он точечный, не идёт по стеку от свежих к старым.

## См. также

- [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/) - MCP-эквивалент
- [`forgeplan restore`](/ru/docs/cli/restore/) - восстановить конкретный артефакт по ID
- [`forgeplan activity`](/ru/docs/cli/activity/) - инспекция таймлайна деструктивных операций
- [`forgeplan delete`](/ru/docs/cli/delete/) - soft-delete, который это откатывает
- [Обзор CLI](/ru/docs/cli/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan undo-last --help`.

```text
Reverse the most recent destructive operation (delete, supersede, or deprecate) by reading the soft-delete trash and applying restore to the most recently written non-consumed receipt. If no matching receipt is found, returns an error with guidance; the tool never guesses

Usage: forgeplan undo-last [OPTIONS]

Options:
      --within-hours <WITHIN_HOURS>  Time window (hours) to search for the last destructive op (1..=720, default 24) [default: 24]
      --json                         Output as JSON for machine consumption
  -h, --help                         Print help
  -V, --version                      Print version
```
<!-- forgeplan-runtime:end -->
