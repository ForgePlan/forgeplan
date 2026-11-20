---
title: forgeplan claims
description: "List live claims in the workspace - who is working on what right now. Sorted by expiry, soonest first."
---

`forgeplan claims` показывает все активные клеймы в воркспейсе - какой агент сейчас работает над каким артефактом и через сколько истекает его клейм. Клеймы отсортированы по времени до истечения, ближайшие - сверху, чтобы самые срочные слоты бросались в глаза. Истёкшие клеймы пропускаются (считаются освобождёнными).

Команда только читает и не берёт блокировку записи клеймов. Это не гарантирует отсутствие затрат при частом опросе. Повреждённые или слишком большие файлы пропускаются и учитываются в `skipped`; при ненулевом значении изучите каталог клеймов.

Аналог [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/) на MCP-стороне.

## Когда использовать

- Оркестратор мониторит на каждом раунде планирования: «какая работа сейчас в процессе?».
- Саб-агент перед собственным клеймом: «не опередил ли меня кто-то по этому артефакту?».
- Health-проверка - ненулевой `skipped` сигнализирует о повреждённых файлах клеймов, требующих внимания.
- Восстановление после краша - посмотреть клеймы, которые никто живой не держит, и принудительно их освободить через [`forgeplan release --force`](/ru/docs/cli/release/).

## Когда НЕ использовать

- Нужен lifecycle-статус (`draft` / `active` / `superseded`) - используйте [`forgeplan list`](/ru/docs/cli/list/) или `forgeplan get <id>`.
- Хотите изменить состояние - эта команда только читает. Для записи используйте [`forgeplan claim`](/ru/docs/cli/claim/) и [`forgeplan release`](/ru/docs/cli/release/).
- Нужна долгосрочная история - клеймы эфемерны (максимум 24 часа TTL). Для истории запросов используйте [`forgeplan activity`](/ru/docs/cli/activity/).

## Использование

```text
forgeplan claims [OPTIONS]
```

## Опции

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

### Пример 1: Дефолтный text-режим

```bash
forgeplan claims
```

Иллюстрация таблицы, не записанный runtime transcript:

```text
ID        agent      expires_in   note
PRD-057   worker-1   12m          implementing FR-003
RFC-012   worker-2   58m          -
```

### Пример 2: JSON-вывод для опроса оркестратором

```bash
forgeplan claims --json | jq '.claims[] | select(.agent_id == "worker-1")'
```

Фильтрует активные клеймы до конкретного агента. Полезно в скрипте диспетчера, которому нужно решить - перепланировать новую работу на этого агента или нет.

### Пример 3: Найти повреждённые файлы клеймов

```bash
forgeplan claims --json | jq '.skipped'
```

Ненулевой `skipped` означает, что хотя бы один файл не разобрался или превысил лимит размера. Изучите `.forgeplan/claims/`: сам счётчик не указывает повреждённый файл.

## Место в рабочем процессе

Это слой мониторинга в multi-agent цикле. Между раундами `dispatch` оркестратор вызывает `claims`, чтобы видеть живую работу; каждый саб-агент вызывает её перед собственным `claim`, чтобы не столкнуться. Совмещайте с [`forgeplan dispatch`](/ru/docs/cli/dispatch/) - планировщик сам читает клеймы внутри себя и исключает уже заклеймённые артефакты из плана.

## См. также

- [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/) - MCP-эквивалент
- [`forgeplan claim`](/ru/docs/cli/claim/) - взять клейм
- [`forgeplan release`](/ru/docs/cli/release/) - снять клейм
- [`forgeplan dispatch`](/ru/docs/cli/dispatch/) - план multi-agent работы

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan claims --help`.

```text
List active claims (sorted by expiry, soonest first)

Usage: forgeplan claims [OPTIONS]

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
