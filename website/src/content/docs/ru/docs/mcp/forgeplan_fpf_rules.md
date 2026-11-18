---
title: forgeplan_fpf_rules
description: "Список действующих декларативных правил FPF по приоритету, с фильтрами действия и источника. Проверьте условия перед forgeplan_fpf_check для артефакта."
---

Возвращает действующие декларативные **правила FPF** по возрастанию числового приоритета. Непустой список `fpf.rules` в `.forgeplan/config.yaml` заменяет встроенные правила; отсутствующий или пустой список оставляет значения по умолчанию. У правила есть условия, действие (`EXPLORE` / `INVESTIGATE` / `EXPLOIT`), приоритет и сообщение.

**Категория**: База знаний FPF

## Когда агент вызывает это

- "Какие правила действуют в этом рабочем пространстве?" - агент хочет получить список перед отладкой неожиданного действия.
- "Покажи мне только правила EXPLORE" - сузить область просмотра при работе с артефактами с низким уровнем доверия.
- "Правило X из конфигурации или из значений по умолчанию?" - используйте `source`, чтобы определить, загружено ли пользовательское переопределение.
- "Дай мне однострочное резюме каждого правила" - передайте `summary: true`, чтобы пропустить полные деревья условий.

## Категории действий

Пять встроенных декларативных правил не сводятся к трём исчерпывающим интервалам R_eff:

| Правило | Условия | Действие |
|---------|---------|----------|
| `blind-spot` | draft и `R_eff < 0.01` | EXPLORE |
| `weak-evidence` | active/stale и `R_eff < 0.5` | INVESTIGATE |
| `orphan-active` | active и нет связей | EXPLORE |
| `medium-quality` | `0.5 ≤ R_eff < 0.7` | INVESTIGATE |
| `ready-to-build` | `R_eff ≥ 0.7` и общий F-G-R `≥ 0.6` | EXPLOIT |

Побеждает первое совпавшее правило по приоритету. Пользовательское условие может учитывать depth, но встроенные правила не повышают пороги автоматически для Deep/Critical. `fpf.thresholds` настраивает отдельный жёстко заданный путь рекомендаций: это не переписывает декларативный список и не разрешает активацию.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `action` | `string` | no | Фильтровать по одной категории: `"EXPLORE"` / `"INVESTIGATE"` / `"EXPLOIT"`. Опустите для всех. |
| `name` | `string` | no | Получить одно правило по точному имени. Вызывает ошибку, если не найдено. |
| `summary` | `bool` | no | Если `true`, возвращает только `{name, priority, action}` для каждого правила - без деревьев условий и сообщений. По умолчанию `false`. |
| `source` | `string` | no | Фильтровать по источнику: `"config"` (пользовательские переопределения) или `"default"` (встроенные). Полезно для отладки загрузки конфигурации. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::FpfRulesParams`_

## Возвращает

Полный режим возвращает `source` (`config` или `default`), `count`, `rules` и подсказку. У правила есть `name`, `priority`, `action`, `condition`, `condition_summary`, `message`. Условия — именованные поля, не массив `all` из объектов field/op; числовые выражения сериализуются строками, например `"< 0.01"`.

Сводный режим опускает условия и сообщения. Иллюстративный ответ с фильтром:

```json
{
  "source": "default",
  "count": 1,
  "rules": [
    { "name": "ready-to-build", "priority": 5, "action": "EXPLOIT" }
  ]
}
```

## Пример вызова

Перечислить все правила:

```json
{}
```

Только правила EXPLORE, в виде сводки:

```json
{ "action": "EXPLORE", "summary": true }
```

Получить конкретное правило по имени:

```json
{ "name": "ready-to-build" }
```

Отладка: действительно ли загружены мои переопределения конфигурации?

```json
{ "source": "config" }
```

## Типичная последовательность

```
forgeplan_fpf_rules                  ← инвентаризация
forgeplan_fpf_check { id: "PRD-019" } ← какое правило побеждает для артефакта
forgeplan_fpf_section { id: "<returned-section-id>" } ← прочитать соответствующий источник
```

## Эквивалент CLI

- [`forgeplan fpf rules`](/ru/docs/cli/fpf-rules/) - тот же список в терминале.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_fpf_check`](/ru/docs/mcp/forgeplan_fpf_check/) - оценивает правила для конкретного артефакта
- [`forgeplan_fpf_list`](/ru/docs/mcp/forgeplan_fpf_list/) - каталог базы знаний
- [`forgeplan_score`](/ru/docs/mcp/forgeplan_score/) - вычисляет R_eff, с которым сопоставляются правила

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

List active FPF rules from the workspace. By default returns all rules with full condition trees and messages. Parameters allow filtering: `action` (EXPLORE/INVESTIGATE/EXPLOIT) to show only rules for that action category; `name` to fetch a single rule by name; `summary: true` to return only name/priority/action without condition details (useful for quick overviews); `source` (config/default) for debugging which rule source is active. If workspace has user-defined rules in .forgeplan/config.yaml under fpf.rules, those take precedence; otherwise built-in defaults are returned.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `action` | нет | string \| null | Filter by action: "EXPLORE", "INVESTIGATE", or "EXPLOIT". Omit for all. |
| `name` | нет | string \| null | Fetch single rule by name. Returns 1 rule or error if not found. |
| `source` | нет | string \| null | Filter by source: "config" or "default". For debugging workspace state. |
| `summary` | нет | boolean \| null | If true, return only {name, priority, action} without condition details. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "action": {
      "default": null,
      "description": "Filter by action: \"EXPLORE\", \"INVESTIGATE\", or \"EXPLOIT\". Omit for all.",
      "type": [
        "string",
        "null"
      ]
    },
    "name": {
      "default": null,
      "description": "Fetch single rule by name. Returns 1 rule or error if not found.",
      "type": [
        "string",
        "null"
      ]
    },
    "source": {
      "default": null,
      "description": "Filter by source: \"config\" or \"default\". For debugging workspace state.",
      "type": [
        "string",
        "null"
      ]
    },
    "summary": {
      "default": null,
      "description": "If true, return only {name, priority, action} without condition details.",
      "type": [
        "boolean",
        "null"
      ]
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "FpfRulesParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
