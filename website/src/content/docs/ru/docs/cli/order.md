---
title: forgeplan order
description: "Показывает граф структурных связей, готовность, препятствия и циклы. Включает based_on, refines, supersedes и contradicts; обход 0.38.0 не задаёт очередь выполнения предпосылок."
---

Показывает граф по структурным отношениям `based_on`, `refines`, `supersedes`, `contradicts`. `informs` не задаёт порядок работ. Включаются только узлы структурных связей; несвязанные артефакты отсутствуют.

## Использование

```text
forgeplan order [OPTIONS]
```

## Опции

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

```bash
forgeplan order
forgeplan order --json | jq -r '.order[] | "- [ ] \(.)"'
forgeplan order --json | jq '.cycles, .blocked, .ready'
```

## Вывод и код завершения

JSON содержит `order` (массив ID), `cycles`, `blocked` (id и blocked_by), `ready`, `_next_action`. Текст отмечает active, ready или blocked с реальным статусом. При обнаружении циклов команда не возвращает ошибку; для политики CI явно проверяйте `.cycles`. Зависимости active, deprecated, superseded считаются разрешёнными.

В 0.38.0 обход Кана добавляет источник связи раньше цели, хотя готовность считает источник зависимым от цели. Поэтому не используйте `.order` как очередь «сначала предпосылки». Изучайте `blocked_by` и `ready` для неразрешённых зависимостей. Узлы, не вошедшие в обход из-за циклов, могут отсутствовать в этих классификациях; проверяйте и `cycles`.

## Планирование

Используйте `blocked` для текущих препятствий, `tree` для иерархии, `graph` для всех связей. Готовность зависимостей не означает прохождение валидации и проверок доказательств.

## См. также

-   [`forgeplan blocked`](/ru/docs/cli/blocked/) - неразрешённые блокировщики
-   [`forgeplan tree`](/ru/docs/cli/tree/) - иерархическое представление
-   [`forgeplan graph`](/ru/docs/cli/graph/) - полный граф границ