---
title: forgeplan_discover_start
description: "Запускает сессию обнаружения brownfield. Возвращает структурированный протокол (7 фаз: detect/structure/code/git/tests/docs/synthesize), которому следует ИИ-агент для составления карты существующей кодовой базы."
---

Запускает **сессию исследования существующей системы** и возвращает семифазный протокол (detect / structure / code / git / tests / docs / synthesize). Агент читает источники и фиксирует наблюдения через `forgeplan_discover_finding`. `forgeplan_discover_complete` отмечает завершение и возвращает счётчики; health и синтез последующих предложений остаются отдельной работой агента.

**Категория**: Обнаружение Brownfield

## Когда агент вызывает это

- Первая сессия в незнакомом репозитории - «что это за проект и в каком он состоянии?»
- Внедрение устаревшей кодовой базы в методологию Forgeplan без создания артефактов из ничего.
- После крупного слияния или поглощения, когда агенту необходимо перестроить карту кодовой базы, прежде чем предлагать новые решения.
- Перед PRD миграции - обнаружение делает состояние «как есть» явным, чтобы цели миграции были обоснованными.

Протокол обнаружения намеренно **управляется агентом**: ForgePlan никогда не читает исходные файлы самостоятельно. Он указывает агенту, *что* искать и в *каком порядке уровней*, а затем принимает результаты.

## Приоритет уровней источников

Протокол задаёт агенту приоритет источников, но не проверяет их содержимое и истинность:

| Уровень | Источники | Роль |
|---------|-----------|------|
| **1** | Код, история Git, манифесты пакетов | Реализация и записанные намерения |
| **2** | Тесты, JSDoc-комментарии, CI-конфигурация | Ожидания от поведения и контекст |
| **3** | Документация, README, wiki | Утверждения для сверки с более сильными основаниями |

Разрешайте противоречия по конкретным доказательствам. Номер уровня не делает утверждение верным автоматически.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `project_name` | `string` | yes | Человекочитаемое имя для сессии обнаружения (например, `"legacy-billing-service"`). Используется в качестве префикса идентификатора сессии и в итоговом отчёте. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::DiscoverStartParams`_

## Возвращает

Возвращает `session_id`, `project_name`, `session_status`, `current_phase` и вложенный `protocol` с полями `version`, `phases`, `source_tier_rules`. У фазы есть `phase`, `order`, `name`, `instructions`. Устаревающее поле `status` также относится к сессии, не к артефактам.

Например, правила уровней в возвращаемом протоколе:

```json
{
  "t1": ["code", "git", "package manifests"],
  "t2": ["tests", "JSDoc comments", "CI configs"],
  "t3": ["docs/ directory", "README files", "wiki"]
}
```

## Пример вызова

```json
{ "project_name": "legacy-billing-service" }
```

## Типичная последовательность

```
forgeplan_discover_start
    ↓ возвращает session_id + 7-фазный протокол
forgeplan_discover_finding (tier=1, phase=detect,    …)
forgeplan_discover_finding (tier=1, phase=structure, …)
forgeplan_discover_finding (tier=1, phase=code,      …)
forgeplan_discover_finding (tier=1, phase=git,       …)
forgeplan_discover_finding (tier=2, phase=tests,     …)
forgeplan_discover_finding (tier=3, phase=docs,      …)
forgeplan_discover_complete (session_id)
    ↓ агент читает наблюдения и отдельно готовит предложения следующих работ
```

Ожидается, что агент будет выполнять итерации по каждой фазе - он может выдавать множество результатов для каждой фазы, прежде чем перейти к следующей.

## Эквивалент CLI

- [`forgeplan discover start`](/ru/docs/cli/discover-start/) - тот же процесс из терминала, для оператора-человека.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_discover_finding`](/ru/docs/mcp/forgeplan_discover_finding/) - добавить одно наблюдение
- [`forgeplan_discover_complete`](/ru/docs/mcp/forgeplan_discover_complete/) — завершить сессию и получить сводку сохранённых наблюдений
- [`forgeplan_health`](/ru/docs/mcp/forgeplan_health/) - состояние проекта после обнаружения

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Start a brownfield discovery session. Returns a structured protocol (7 phases: detect/structure/code/git/tests/docs/synthesize) that the AI agent follows to map an existing codebase. ForgePlan provides the protocol; the agent parses code and reports findings via forgeplan_discover_finding.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `project_name` | да | string | Project name for the discovery session |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "project_name": {
      "description": "Project name for the discovery session",
      "type": "string"
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
  "required": [
    "project_name"
  ],
  "title": "DiscoverStartParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
