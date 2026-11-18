---
title: forgeplan_calibrate
description: "Предлагает глубину Tactical, Standard или Deep по содержимому, связям и структуре артефакта."
---

Предлагает **глубину** для одного или всех артефактов на основе эвристик. Выбранная реализация возвращает `Tactical`, `Standard` или `Deep`. Описание из каталога ниже упоминает также `Critical`, но алгоритм калибровки его не выдаёт. Рекомендация не меняет метаданные и не доказывает готовность реализации.

**Категория**: Качество

## Когда агент вызывает его

- **После создания заглушки** - позвольте калибровке предложить правильный уровень глубины перед заполнением разделов.
- **Во время ревью** - проверьте, что автор выбрал разумный уровень глубины.
- **Аудиторская проверка** - найдите артефакты, помеченные как `Tactical`, которые на самом деле требуют `Deep` на основе содержимого.
- **При обучении агентов** - научите их, какие факторы повышают уровень глубины (безопасность, критические изменения, межкомандный охват).

Учитываемые эвристики: наличие разделов безопасности, маркеры критических изменений, количество исходящих ссылок, длина содержимого и сложность разделов.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | no | ID артефакта для калибровки. Опустите, чтобы откалибровать все артефакты в рабочем пространстве. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::CalibrateParams`_

## Возвращает

Учебный фрагмент ответа. Обработчик возвращает `results`, `signals` и `total_escalations`, а не массив `suggestions`.

```json
{
  "results": [
    {
      "artifact_id": "PRD-042",
      "artifact_title": "Система аутентификации",
      "current_depth": "tactical",
      "suggested_depth": "Deep",
      "escalation_needed": true,
      "signals": [
        { "name": "security_section", "value": "present", "minimum_depth": "Deep" }
      ]
    }
  ],
  "total_escalations": 1
}
```

## Пример вызова

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

1. `forgeplan_calibrate` - сканирует рабочее пространство.
2. Если изменение обосновано, используйте CLI для метаданных, например `forgeplan update PRD-042 --depth deep`. В MCP-схеме `forgeplan_update` параметра `depth` нет.
3. `forgeplan_validate` - повторно запустите валидацию с новой глубиной (более строгие правила MUST).
4. Выполните применимое требование ADI для выбранной глубины: сама калибровка рассуждение не выполняет.

## Эквивалент CLI

```bash
forgeplan calibrate
forgeplan calibrate PRD-042
```

## См. также

- [`forgeplan_route`](/ru/docs/mcp/forgeplan_route/) - выбор глубины при создании задачи.
- [`forgeplan_validate`](/ru/docs/mcp/forgeplan_validate/) - валидация с учётом глубины.
- [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Suggest depth level (Tactical/Standard/Deep/Critical) for artifacts based on content analysis. Detects security sections, breaking changes, link count, body complexity.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | нет | string \| null | Artifact ID (checks all if omitted) |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "default": null,
      "description": "Artifact ID (checks all if omitted)",
      "type": [
        "string",
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
  "title": "CalibrateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
