---
title: forgeplan discover list
description: "Список всех сессий brownfield-обнаружения в рабочем пространстве"
---

`forgeplan discover list` выводит все сессии обнаружения, сохранённые в рабочем пространстве - активные и завершённые - с ID, статусом, временем создания и кратким резюме покрытия. Это индексное представление всех сессий.

## Когда использовать

- **Чтобы продолжить остановленную сессию** - найти ID для передачи в `discover show` или `discover complete`.
- **Для аудита истории обнаружения** - увидеть, сколько проходов онбординга / обновления было у проекта.
- **Перед началом новой сессии** - избежать дублирования активной.
- **В CI или скриптах** - перечислить сессии для автоматизации.

## Когда НЕ использовать

- Для инспекции находок одной сессии - используйте [`discover show`](/ru/docs/cli/discover-show/).
- Для запуска новой сессии - используйте [`discover start`](/ru/docs/cli/discover-start/).

## Использование

```text
forgeplan discover list
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

```bash
# Все сессии
forgeplan discover list

# Типичный флоу восстановления "где я остановился"
forgeplan discover list
forgeplan discover show disc-002
forgeplan discover complete disc-002
```

## Что вы увидите

Таблицу (или список) с одной строкой на сессию, обычно показывающую:

- **ID сессии** (`disc-NNN`)
- **Статус** - активная / завершённая
- **Создана** - ISO-метка времени
- **Количество находок** - сколько вызовов `discover_finding` попало в эту сессию
- **Покрытие** - краткое резюме по уровням (какие из code/git/tests/docs были затронуты)

## Как это вписывается

`discover list` - это примитив перечисления для подсистемы обнаружения. Всё остальное (`show`, `complete`) работает с конкретным ID сессии, который вы обычно выбираете из этого списка.

```
discover list → выбрать ID → discover show / complete
```

## См. также

- [`forgeplan discover`](/ru/docs/cli/discover/) - родительская команда
- [`forgeplan discover start`](/ru/docs/cli/discover-start/) - создать новую сессию
- [`forgeplan discover show`](/ru/docs/cli/discover-show/) - инспектировать конкретную сессию
- [`forgeplan discover complete`](/ru/docs/cli/discover-complete/) - финализировать сессию