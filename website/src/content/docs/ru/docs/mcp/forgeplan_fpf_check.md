---
title: forgeplan_fpf_check
description: "Оценивает набор правил FPF для конкретного артефакта. Возвращает каждое совпавшее правило, выигрышное правило (первое в порядке приоритета - как и во время выполнения), правила, которые не совпали, и рекомендуемую категорию действий. Используйте это, чтобы ответить на вопрос 'что мне делать дальше с этим артефактом и почему?'"
---

Запускает активные правила FPF для одного артефакта и возвращает всё, что обнаружил движок: какие правила совпали, какие нет, какое правило выиграло в порядке приоритета, и рекомендуемую категорию действий (`EXPLORE` / `INVESTIGATE` / `EXPLOIT`). Это инструмент агента «что мне делать дальше?» - он превращает абстрактный набор правил из `forgeplan_fpf_rules` в конкретную рекомендацию, привязанную к реальному артефакту.

**Категория**: База знаний FPF

## Когда агент вызывает это

- На вопрос «какое следующее действие по PRD-042?» изучите `winning.message`, если `winning` не равен `null`.
- При ревью смотрите условия совпавших правил и их источник, а не принимайте метку действия за порог оценки.
- Для понимания рекомендаций правил: `fpf_check` показывает выбранный набор правил, но не трассировку каждого решения health или LLM.
- Перед активацией проверяйте рекомендацию отдельно от валидации, доказательств и происхождения изменений. `EXPLOIT` не является разрешением на активацию.

## Как это работает

1. Загружает действующий набор правил (переопределения конфигурации > значения по умолчанию), тот же путь, что и у `forgeplan_fpf_rules`.
2. Извлекает артефакт по ID (frontmatter + R_eff + ссылки на доказательства).
3. Оценивает дерево условий каждого правила относительно состояния артефакта.
4. Собирает совпадения и сортирует по `priority` - **первое совпадение выигрывает**, точно так же, как это делает движок во время выполнения.
5. Сообщает `action` выигрышного правила как рекомендуемую категорию вместе с его `message`.

Этот инструмент применяет `fpf.rules` либо встроенные правила, а не универсальную таблицу порогов с поправкой на глубину. У стандартных правил есть буквальные условия: draft `r_eff < 0.01`, active/stale `r_eff < 0.5`, а также `r_eff >= 0.7` вместе с `overall >= 0.6`. Пользовательские правила могут проверять глубину. Автоматической поправки `+0.10` для Deep в этой проверке нет.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта, например `PRD-042`. Обработчик отклоняет ID длиннее 128 байт; схема этот предел не показывает. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::FpfCheckParams`_

## Возвращает

Учебный фрагмент ответа для черновика без совпадений со стандартными правилами. `unmatched` содержит строки имён правил; поле победителя — `winning`, оно может быть `null`.

```json
{
  "artifact_id": "PRD-042",
  "kind": "prd",
  "status": "draft",
  "matched": [],
  "unmatched": ["blind-spot", "weak-evidence", "orphan-active", "medium-quality", "ready-to-build"],
  "winning": null
}
```

Если ID не существует, инструмент возвращает ошибку, чтобы агент мог вернуться к `forgeplan_search`.

## Пример вызова

```json
{ "id": "PRD-042" }
```

## Типовая последовательность

```
forgeplan_list (status=draft)       ← найти кандидатов
forgeplan_fpf_check { id: "PRD-X" } ← какая рекомендуемая категория?
  → EXPLORE:     forgeplan_new evidence + forgeplan_link
  → INVESTIGATE: forgeplan_reason + добавить измерения
  → EXPLOIT:     forgeplan_review → forgeplan_activate
```

## Эквивалент CLI

- [`forgeplan fpf check <ID>`](/ru/docs/cli/fpf-check/) - идентичный вывод, отображаемый в терминале.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_fpf_rules`](/ru/docs/mcp/forgeplan_fpf_rules/) - перечень правил, оцениваемых здесь
- [`forgeplan_score`](/ru/docs/mcp/forgeplan_score/) - вычисляет R_eff, с которым сопоставляются правила
- [`forgeplan_reason`](/ru/docs/mcp/forgeplan_reason/) - рассуждения ADI, дополняющие проверки на основе правил
- [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) - переход жизненного цикла со своими гейтами, независимо от метки действия FPF

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Check which FPF rules match a given artifact, showing all matched rules, the winning rule (first in priority order, same as runtime), and rules that did not match. Use this to understand FPF engine behavior for a specific artifact before acting on it.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Artifact ID to check (e.g. "PRD-041") |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to check (e.g. \"PRD-041\")",
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
    "id"
  ],
  "title": "FpfCheckParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
