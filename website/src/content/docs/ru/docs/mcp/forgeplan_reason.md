---
title: forgeplan_reason
description: "Анализирует артефакт с использованием цикла рассуждений FPF ADI: Абдукция (3+ гипотезы) → Дедукция (оценка каждой) → Индукция (синтез рекомендации). Требует LLM-провайдера."
---

Запускает FPF ADI (Абдукция → Дедукция → Индукция) для артефакта через настроенного LLM-провайдера. Промпт запрашивает не менее трёх конкурирующих гипотез, проверяемые выводы и индукцию по доказательствам; это не гарантия выполнения запроса моделью или измеренной уверенности. По методологии ADI остаётся обязательным для Deep/Critical. У MCP-вызова нет входа `fpf`; CLI `--fpf` — отдельная опция.

**Категория**: Рассуждения и ИИ

## Когда агент вызывает эту функцию

- После `forgeplan_validate` PASS, но до кодирования артефакта глубины Deep/Critical - обязательный гейт ADI.
- Когда пользователь колеблется между двумя подходами - ADI формирует сопоставимые предсказания.
- Во время состязательной ревью: переосмыслить с другими ограничениями для стресс-тестирования текущего решения.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для анализа с помощью цикла рассуждений ADI. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ReasonParams`_

## Возвращает

Возвращает `artifact_id`, `artifact_title`, строку `analysis`, `provider`, `model`, trace workspace и подсказку. Контракта на отдельные поля MCP `hypotheses` или `induction` нет.

Проверьте, есть ли в анализе действительно конкурирующие варианты, предсказания, основания и неопределённость. Уверенность модели не является измерением надёжности или разрешением на активацию.

Иллюстративный фрагмент:

```json
{
  "artifact_id": "PRD-042",
  "artifact_title": "Учебный проект",
  "analysis": "Абдукция: сравнить варианты. Дедукция: указать различающие предсказания. Индукция: выбрать лишь настолько уверенно, насколько позволяют доказательства."
}
```

## Пример вызова

```json
{ "id": "PRD-001" }
```

С типичным контекстом агента:

> PRD-042 имеет глубину Deep. Агент запускает ADI и проверяет фактический анализ до написания кода.

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

`forgeplan_validate` PASS → `forgeplan_reason` (обязательно для Deep/Critical) → обсуждение вариантов и доказательств с пользователем → выбор направления → код. Сообщайте о неопределённости, не превращая неподтверждённое число уверенности в гейт.

## Эквивалент CLI

- [`forgeplan reason`](/ru/docs/cli/reason/) - тот же конвейер, вывод в терминал

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [Методология FPF](/ru/docs/methodology/adi/)
- [`forgeplan_fpf_search`](/ru/docs/mcp/forgeplan_fpf_search/) - прямой поиск по базе знаний
- [`forgeplan_decompose`](/ru/docs/mcp/forgeplan_decompose/) - рассуждения, применённые к декомпозиции

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Analyze an artifact using FPF ADI reasoning cycle: Abduction (3+ hypotheses) → Deduction (evaluate each) → Induction (synthesize recommendation). Requires LLM provider.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Artifact ID to analyze with ADI reasoning cycle |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to analyze with ADI reasoning cycle",
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
  "title": "ReasonParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
