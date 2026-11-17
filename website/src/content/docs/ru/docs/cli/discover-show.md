---
title: forgeplan discover show
description: "Показывает существующую сессию перед продолжением или завершением. Читает сохранённое состояние сессии, а не таблицу LanceDB или поток работы другого агента."
---

Показывает существующую сессию перед продолжением или завершением. Читает сохранённое состояние сессии, а не таблицу LanceDB или поток работы другого агента.

## Использование

```text
forgeplan discover show <SESSION_ID>
```

## Аргументы

```text
  <SESSION_ID>  Session ID (e.g. disc-20260407-abc)
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Пример

```bash
forgeplan discover list
# Replace SESSION_ID with a listed identifier:
forgeplan discover show SESSION_ID
```

## Вывод

Показывает ID, проект, статус, время начала, текущую фазу, количество находок по фазам и уровням источников, до десяти последних находок. Поля владельца нет; достаточность охвата не определяется автоматически. Перед завершением проверьте, что находки покрывают нужную область.

## См. также

- [`forgeplan discover`](/ru/docs/cli/discover/) - родительская команда
- [`forgeplan discover start`](/ru/docs/cli/discover-start/) - начать сессию
- [`forgeplan discover list`](/ru/docs/cli/discover-list/) - все сессии
- [`forgeplan discover complete`](/ru/docs/cli/discover-complete/) - сохранить завершение и вывести итоги