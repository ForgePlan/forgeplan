---
title: forgeplan update
description: "Обновляет метаданные артефакта или заменяет его содержимое целиком без повторного создания по шаблону. Команда записывает markdown и синхронизирует индекс LanceDB."
---

Обновляет метаданные артефакта или заменяет его содержимое целиком без повторного создания по шаблону. Команда записывает markdown и синхронизирует индекс LanceDB.

## Использование

```text
forgeplan update [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Artifact ID
```

## Опции

```text
      --status <STATUS>  New status (draft, active, superseded, deprecated)
      --title <TITLE>    New title
      --depth <DEPTH>    New depth (tactical, standard, deep)
      --body <BODY>      New body content (use @filepath to read from file)
  -h, --help             Print help
  -V, --version          Print version
```

## Ограничения жизненного цикла

`update --status` отклоняет `active`, `superseded` и `deprecated`, хотя справка 0.38.0 перечисляет их. Используйте `activate`, `supersede`, `deprecate`, а для устаревших решений — `renew` или `reopen`. Остальные записи статуса в 0.38.0 не проходят валидатор переходов жизненного цикла: это ограничение, не разрешение возвращать terminal-артефакты или придумывать статусы. Runtime принимает `--depth critical`, хотя справка его не перечисляет.

## Примеры

```bash
forgeplan update PRD-001 --title "Authentication and SSO"
forgeplan update PRD-001 --depth deep
forgeplan update NOTE-042 --body @./draft.md
forgeplan activate PRD-001
```

## Содержимое и прямые правки markdown

`--body` заменяет всё содержимое, а не раздел. `@filepath` читает файл; YAML frontmatter из него удаляется, а не вставляется в тело артефакта. Перед заменой проверьте передаваемый текст. После прямого изменения исходного markdown выполните `reindex`. `scan-import` предназначен для новых внешних документов, а не для обновления индекса.

## См. также

- [CLI](/ru/docs/cli/)
- [`forgeplan activate`](/ru/docs/cli/activate/)
- [`forgeplan reindex`](/ru/docs/cli/reindex/)
