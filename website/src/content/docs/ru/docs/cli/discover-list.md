---
title: forgeplan discover list
description: "Перечисляет сохранённые сессии для продолжения работы, изучения прежних исследований или предотвращения дубликатов."
---

Перечисляет сохранённые сессии для продолжения работы, изучения прежних исследований или предотвращения дубликатов.

## Использование

```text
forgeplan discover list
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Пример

```bash
forgeplan discover list
# Copy a session identifier from the table:
forgeplan discover show SESSION_ID
```

## Вывод

Текстовая таблица содержит Session ID, Project, Status, Phase и число Findings. Столбцов времени создания и охвата уровней источников нет; флага JSON тоже нет. Время начала, уровни и последние находки показывает `show`. Копируйте реальные ID, не рассчитывая на последовательность disc-NNN.

## См. также

- [`forgeplan discover`](/ru/docs/cli/discover/) - родительская команда
- [`forgeplan discover start`](/ru/docs/cli/discover-start/) - создать новую сессию
- [`forgeplan discover show`](/ru/docs/cli/discover-show/) - инспектировать конкретную сессию
- [`forgeplan discover complete`](/ru/docs/cli/discover-complete/) - финализировать сессию