---
title: forgeplan_route
description: "Рекомендует глубину и цепочку артефактов. MCP и CLI могут использовать настроенный LLM с возвратом к ключевым словам; CLI auto может повысить Deep до Level 2."
---

Рекомендует глубину и цепочку артефактов перед нетривиальной работой. Сверяйте рекомендацию с реальным риском и объёмом задачи: это не доказательство правильности дизайна или трудоёмкости. MCP пробует Level 1 при наличии конфигурации LLM, иначе возвращается к ключевым словам Level 0. Настроенный локальный CLI-провайдер не обязательно требует API-ключа.

**Категория**: Рассуждения и ИИ

## Когда агент вызывает это

- Пользователь говорит «пожалуйста, добавьте X в проект» - агент выполняет роутинг, прежде чем решить, создавать ли артефакты.
- Перед `forgeplan_new`: глубина из роутинга определяет, какие разделы шаблона являются обязательными.
- Когда глубина рефакторинга неясна — использовать рекомендацию как основание для решения, а не безошибочный ответ.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `description` | `string` | yes | Описание задачи на естественном языке. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::RouteParams`_

## Возвращает

Возвращает `depth`, `pipeline`, `triggers`, `confidence`, `level`, `explanation`, `display`, `_alternatives`, `_next_action`. У альтернативы есть `depth`, `pipeline`, `reasoning`. Уверенность — оценка классификатора, не измеренная вероятность успеха.

Иллюстративный фрагмент:

```json
{
  "depth": "Standard",
  "pipeline": ["prd", "rfc"],
  "level": 0,
  "explanation": "Учебное объяснение классификации",
  "_alternatives": []
}
```

## Пример вызова

```json
{ "description": "add rate limiting to API" }
```

В типичном контексте агента:

> Пользователь запрашивает функцию. Агент сначала выполняет роутинг, чтобы остальная часть рабочего процесса соответствовала правильной глубине.

```json
{ "description": "rewrite the LanceDB storage layer to use SQLite" }
```

## Типичная последовательность

`forgeplan_route` → проверить глубину → для Standard+ найти существующие решения и заполнить нужные артефакты → `forgeplan_validate` → ADI при необходимости (обязательно для Deep/Critical) → код. Tactical не отменяет тесты и применимую политику проекта.

## Эквивалент CLI

- [`forgeplan route`](/ru/docs/cli/route/) — выбранный CLI по умолчанию работает в auto: пробует настроенный LLM Level 1 с возвратом к ключевым словам и может повысить успешный ответ Deep Level 1 до Level 2, если `--level` не задан. В снимке help по-прежнему указан Level 0 по умолчанию, но обработчик ведёт себя иначе. Для классификации только по ключевым словам явно передавайте `--level 0` **без `--explain`**: добавление `--explain` может вызвать настроенного провайдера даже при Level 0.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [Руководство по калибровке глубины](/ru/docs/methodology/routing/)
- [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) - следующий шаг после роутинга
- [`forgeplan_reason`](/ru/docs/mcp/forgeplan_reason/) — ADI, обязательный для Deep/Critical

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Suggest depth level (Tactical/Standard/Deep/Critical) and artifact pipeline for a task description. Uses LLM classification (Level 1) when API key is configured, falls back to rule-based keywords (Level 0).

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `description` | да | string | Task description in natural language |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "description": {
      "description": "Task description in natural language",
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
    "description"
  ],
  "title": "RouteParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
