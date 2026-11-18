---
title: forgeplan claim
description: "Claim an artifact for exclusive work - TTL-based advisory lock for multi-agent dispatch (PRD-057)."
---

`forgeplan claim` резервирует артефакт за конкретным агентом, чтобы другие агенты знали - туда не лезть. Команда пишет небольшой файл `.forgeplan/claims/<id>.yaml` с идентичностью агента и временем истечения (TTL - time-to-live, после этого клейм считается заброшенным и другой агент может взять артефакт). По умолчанию TTL - 30 минут, максимум - 24 часа.

Это **рекомендательная блокировка** работы: она не запрещает изменения артефактов. Сами `claim` и `release` проверяют владельца. Перед работой агенты проверяют [`forgeplan claims`](/ru/docs/cli/claims/); [`forgeplan dispatch`](/ru/docs/cli/dispatch/) исключает живые клеймы.

Две гарантии безопасности:

- **Двойного клейма не будет** - если живой клейм уже держит другой агент, вызов завершится понятной ошибкой.
- **Продление тем же владельцем** - повторный `claim` сохраняет владельца, но перезаписывает `claimed_at`, срок и заметку. Передайте `--note` повторно, чтобы сохранить её.

Аналог [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) на MCP-стороне.

## Когда использовать

- Саб-агент получил корзину от [`forgeplan dispatch`](/ru/docs/cli/dispatch/) - клеймите артефакт перед редактированием файлов.
- Долгая работа (большой рефакторинг, multi-PR фича) - вызывайте `claim` каждые 20 минут или около того, чтобы продлевать срок до истечения.
- Оркестратор-скрипт хочет заклеймить от имени саб-агента, который не говорит по MCP - передайте `--agent worker-1` явно.

## Когда НЕ использовать

- Однопоточный workflow - гонки нет, клейм не нужен.
- Ожидаете жёсткий лок, который заблокирует другие команды - клеймы рекомендательные, агенты должны проверять их сами.
- Не подумав о TTL - дефолтные 30 минут защищают от заброшенных клеймов, которые блокируют воркспейс навсегда. Поднимайте TTL только если планируете продлевать до истечения.

## Использование

```text
forgeplan claim [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Artifact ID to claim (e.g. PRD-057)
```

## Опции

```text
      --agent <AGENT>              Явное печатное имя без символов пути, например worker-1; default cli-<version>
      --ttl-minutes <TTL_MINUTES>  Time-to-live in minutes (default 30, max 1440 = 24h, min 1) [default: 30]
      --note <NOTE>                Optional free-form note surfaced by `forgeplan claims`
      --json                       Output as JSON for machine consumption
  -h, --help                       Print help
  -V, --version                    Print version
```

## Примеры

### Пример 1: Саб-агент клеймит с дефолтным TTL

```bash
forgeplan claim PRD-057 --note "implementing FR-003"
```

Резервирует `PRD-057` на 30 минут за `cli-<version>`. Сохранённый help ниже ошибочно указывает `cli/<version>`; явное CLI-имя не может содержать `/`. У `release` в этой сборке другой default: смотрите владельца в `claims` и передавайте точное имя через `release --agent`, а не `--force`.

### Пример 2: Оркестратор клеймит от имени агента

```bash
forgeplan claim RFC-012 --agent worker-2 --ttl-minutes 60
```

Полезно, когда оркестратор - это shell-скрипт, гоняющий саб-агентов, не говорящих напрямую по MCP. 60-минутный TTL даёт `worker-2` больше времени до необходимости продления.

### Пример 3: Продление существующего клейма

```bash
forgeplan claim PRD-057 --ttl-minutes 30
```

Тот же владелец может обновить клейм. Помимо срока перезаписываются время захвата и заметка, поэтому при необходимости повторите `--note`.

## Место в рабочем процессе

Это шаг 2 multi-agent цикла: `dispatch` → **`claim`** → работа → `release`. После того как [`forgeplan dispatch`](/ru/docs/cli/dispatch/) раздал корзины, каждый саб-агент клеймит свой артефакт перед тем, как трогать файлы. Когда работа закончена (или агент упал), слот освобождается через [`forgeplan release`](/ru/docs/cli/release/) - используйте `--force` на `release`, чтобы убрать клейм упавшего агента.

## См. также

- [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) - MCP-эквивалент
- [`forgeplan release`](/ru/docs/cli/release/) - снять активный клейм
- [`forgeplan claims`](/ru/docs/cli/claims/) - список активных клеймов
- [`forgeplan dispatch`](/ru/docs/cli/dispatch/) - продуцирует план работы, который защищают клеймы

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan claim --help`.

```text
Claim an artifact (PRD-057 multi-agent coordination — soft signal "I'm working on this")

Usage: forgeplan claim [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID to claim (e.g. PRD-057)

Options:
      --agent <AGENT>              Agent identity ("name/version"). Defaults to `cli/<version>`
      --ttl-minutes <TTL_MINUTES>  Time-to-live in minutes (default 30, max 1440 = 24h, min 1) [default: 30]
      --note <NOTE>                Optional free-form note surfaced by `forgeplan claims`
      --json                       Output as JSON for machine consumption
  -h, --help                       Print help
  -V, --version                    Print version
```
<!-- forgeplan-runtime:end -->
