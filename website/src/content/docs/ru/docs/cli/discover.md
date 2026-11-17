---
title: forgeplan discover
description: "Управляет исследованием существующего проекта по протоколу. start создаёт сессию и печатает инструкции; человек или агент должен действительно изучить код, Git, тесты и документацию и сообщить находки. CLI не исследует и не реализует проект самостоятельно."
---

Управляет исследованием существующего проекта по протоколу. `start` создаёт сессию и печатает инструкции; человек или агент должен действительно изучить код, Git, тесты и документацию и сообщить находки. CLI не исследует и не реализует проект самостоятельно.

## Использование

```text
forgeplan discover <COMMAND>
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Подкоманды

```text
  start     Start a new discovery session — prints protocol for AI agent
  list      List all discovery sessions in the workspace
  show      Show status of a discovery session
  complete  Mark a discovery session as completed
  help      Print this message or the help of the given subcommand(s)
```

## Работа с сессией

`start NAME` начинает сессию, `list` перечисляет сохранённые сессии, `show SESSION_ID` показывает находки, `complete SESSION_ID` сохраняет завершение. Родителю нужна подкоманда. MCP дополнительно предоставляет `forgeplan_discover_finding`; соответствующей команды CLI для сообщения находки нет.

## Пример

```bash
forgeplan init -y
forgeplan discover start "Legacy API"
# Follow the printed protocol and report findings through MCP.
forgeplan discover list
# Replace SESSION_ID with the actual identifier:
forgeplan discover show SESSION_ID
forgeplan discover complete SESSION_ID
forgeplan health
```

## Протокол и дальнейшая работа

Семь фаз протокола: detect, structure, code, git, tests, docs, synthesize. Уровни источников разделяют авторитетные, извлечённые и вспомогательные сведения. Проверяйте противоречия по реальному поведению, а не считайте старые документы истиной. Завершение CLI только сохраняет статус и печатает итоги; не запускает health и не генерирует предложения артефактов. Разберите находки и явно создайте нужные Problems, PRDs, ADRs или Notes через `new`.

## См. также

-   [`forgeplan discover start`](/ru/docs/cli/discover-start/) - начать сессию + вывести протокол
-   [`forgeplan discover list`](/ru/docs/cli/discover-list/) - все сессии в рабочем пространстве
-   [`forgeplan discover show`](/ru/docs/cli/discover-show/) - проверить находки сессии
- [`forgeplan discover complete`](/ru/docs/cli/discover-complete/) - сохранить завершение и вывести итоги
-   [Руководство по методологии](/ru/docs/methodology/overview/)
-   [`forgeplan health`](/ru/docs/cli/health/) - сравнение до/после
