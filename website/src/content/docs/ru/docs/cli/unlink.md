---
title: forgeplan unlink
description: "Удалить связь между двумя артефактами"
---

Удаляет типизированную связь между двумя артефактами, обратная операция к
[`forgeplan link`](/ru/docs/cli/link/). Исправляет неверное направление
или тип связи, оставляя сами артефакты на месте.

## Использование

```text
forgeplan unlink [OPTIONS] <SOURCE> <TARGET>
```

## Аргументы

```text
  <SOURCE>  ID исходного артефакта
  <TARGET>  ID целевого артефакта
```

## Опции

```text
      --relation <RELATION>  Тип связи для удаления [по умолчанию: informs]
  -h, --help                 Вывести справку
  -V, --version              Вывести версию
```

## Что делает команда

Команда удаляет совпавшее ребро `(source, target, relation)` и обновляет
каноническую проекцию. В workspace Constellation локальной
`0.39.0+constellation.s1.1` меняет `links:` frontmatter исходного файла и
запрашивает reconcile без открытия legacy storage.

Отсутствующее ребро - ошибка (`Relation '…' from … to … not found`), не
успешный no-op. Второй одинаковый вызов поэтому может завершиться ошибкой.

Отмена связи **не** удаляет ни один из артефактов. Она только удаляет ребро.

## Примеры

Исправление ссылки с неправильным направлением:

```bash
forgeplan unlink PRD-001 EVID-001 --relation informs
forgeplan link EVID-001 PRD-001 --relation informs
```

Удаление просроченной `based_on` после замещения родительского элемента:

```bash
forgeplan unlink RFC-006 PRD-025 --relation based_on
forgeplan link RFC-006 PRD-030 --relation based_on
```

Удаление ребра `contradicts`, которое было записано по ошибке:

```bash
forgeplan unlink EVID-017 ADR-004 --relation contradicts
```

## Тип связи должен совпадать

`--relation` должен совпадать с записанным типом. Если ребро имеет тип
`informs`, удаление `based_on` сообщит, что связь не найдена.

Если сомневаетесь, сначала проверьте граф:

```bash
forgeplan graph                 # граф workspace, без positional ID артефакта
forgeplan get PRD-001 --json     # проверить links в ответе артефакта
```

## Защита от самосвязей (PROB-019)

Самосвязи отклоняются при создании. Попытка удалить несуществующую
самосвязь поэтому сообщает об отсутствии связи, не об успешном no-op.

## Побочные эффекты

- [`forgeplan score`](/ru/docs/cli/score/) пересчитывает R_eff. Удаление
  единственного supporting `informs` может убрать поддержку решения.
- [`forgeplan blocked`](/ru/docs/cli/blocked/) и
  [`forgeplan order`](/ru/docs/cli/order/) перечитывают структуру зависимостей.
- [`forgeplan graph`](/ru/docs/cli/graph/) больше не показывает удалённое ребро.

## Примечания

- Отсутствующее ребро вызывает ошибку; скрипт должен обработать такой результат.
- Unlink удаляет ребро, не артефакт. Не удаляйте артефакт ради исправления связи.
- После осознанной правки frontmatter используйте
  [`forgeplan index`](/ru/docs/cli/index-command/) на Constellation или legacy `reindex` для подходящей раскладки,
  не `scan-import`.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan link`](/ru/docs/cli/link/) - создать связь
- [`forgeplan graph`](/ru/docs/cli/graph/) - граф связей
- [`forgeplan score`](/ru/docs/cli/score/) - R_eff
- [Методология](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan unlink --help`.

```text
Remove a relation between two artifacts

Usage: forgeplan unlink [OPTIONS] <SOURCE> <TARGET>

Arguments:
  <SOURCE>  Source artifact ID
  <TARGET>  Target artifact ID

Options:
      --relation <RELATION>  Relationship type to remove [default: informs]
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- forgeplan-runtime:end -->
