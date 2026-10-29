---
title: forgeplan untag
description: "Удалить теги из артефакта"
---

Удаляет один или несколько тегов из артефакта. Обратная операция для
[`forgeplan tag`](/docs/cli/tag/). Используйте это для удаления устаревших меток (`legacy`,
которая больше не является устаревшей, `wip` для того, что уже выпущено) или для исправления ошибок,
возникших в результате массового тегирования.

## Использование

```text
forgeplan untag <ID> <TAGS>...
```

## Аргументы

```text
  <ID>       ID артефакта
  <TAGS>...  Теги для удаления
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Как это работает

1. Загружает фронтматтер артефакта.
2. Канонизирует каждый входящий тег так же, как это делает `tag` (приводит к нижнему регистру,
   обрезает пробелы), чтобы `Legacy`, `legacy` и `  legacy  ` соответствовали сохранённому значению.
3. Удаляет каждое совпадение из списка `tags:`.
4. Перезаписывает канонический frontmatter и обновляет обслуживающий индекс; на Constellation запрашивает reconciliation, не открывая legacy storage.

Неизвестные теги игнорируются без уведомления - `untag` идемпотентен и безопасен для использования в скриптах.

## Примеры

Удалить один тег:

```bash
forgeplan untag PRD-001 legacy
```

Удалить несколько тегов за один вызов:

```bash
forgeplan untag PRD-018 wip experimental source=draft
```

Исправить ошибку массового тегирования (сначала тегировать, затем сразу удалить тег):

```bash
forgeplan tag PRD-001 securiti   # опечатка
forgeplan untag PRD-001 securiti
forgeplan tag PRD-001 security   # правильно
```

## Массовая очистка

Сначала просмотрите ID:

```bash
forgeplan list --tag wip --json | jq -r '.[].id'
# После проверки списка удалите тег у каждого выбранного артефакта:
forgeplan untag PRD-001 wip
```

`index` обслуживает производное состояние, но не гарантирует перезапись тегов в исходниках.

## Примечания

- `untag` нечувствителен к регистру при вводе: `forgeplan untag PRD-001 Legacy`
  удалит `legacy`.
- Если вы хотите переместить артефакт из одного тега в другой, выполните `untag`, а затем
  `tag` - вспомогательной команды `retag` нет.
- Удаление тега происхождения не удаляет отдельное поле тела `source_tier`, используемое скорингом доказательств. Теги и объявленные поля скоринга — разные данные.

## См. также

- [Обзор CLI](/docs/cli/)
- [`forgeplan tag`](/docs/cli/tag/) - добавить теги
- [`forgeplan index`](/ru/docs/cli/index-command/) - обслуживание производного индекса Constellation
- [`forgeplan list`](/docs/cli/list/) - фильтровать артефакты по тегу
- [Руководство по методологии](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan untag --help`.

```text
Remove tags from an artifact

Usage: forgeplan untag <ID> <TAGS>...

Arguments:
  <ID>       Artifact ID
  <TAGS>...  Tags to remove

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
