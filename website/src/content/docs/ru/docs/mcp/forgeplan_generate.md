---
title: forgeplan_generate
description: "Генерирует полный текст артефакта из описания на естественном языке, используя настроенный провайдер LLM."
---

Создаёт новый черновик с телом от LLM по описанию на естественном языке через настроенный HTTP- или поддержанный локальный CLI-провайдер. Записывает канонический Markdown; генерация не доказывает полноту и правильность обязательных разделов. Прочитайте результат через `forgeplan_get`, проверьте и отредактируйте его, затем выполните валидацию.

**Категория**: Создание артефактов

## Когда агент вызывает это

- Пользователь предоставляет подробное описание и хочет получить готовый к ревью первый черновик, а не пустую заглушку.
- Миграция решения из истории чата в формальный ADR - используйте сводку чата в качестве описания.
- Массовая инициализация: превращение неформальной дорожной карты в 5 черновиков PRD за одну сессию.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `kind` | `string` | yes | Тип артефакта: `prd`, `epic`, `spec`, `rfc`, `adr`, `problem`, `solution`, `evidence`, `note`, `refresh`. |
| `description` | `string` | yes | Описание на естественном языке того, что нужно сгенерировать. |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::GenerateParams`_

## Возвращает

Разобранный MCP payload содержит `id`, `kind`, `title`, `filepath`,
`provider`, `model`, `_next_action`, а также поля выбора workspace
и публикации индекса. Полей `body`, `status`, вложенного объекта `llm`
и измерений расхода токенов нет. Получите созданное тело через
`forgeplan_get`. В мигрированном workspace проверяйте `index_pending`,
не предполагая одновременность канонической записи и производной публикации.

## Пример вызова

```json
{ "kind": "prd", "description": "OAuth2 login flow" }
```

В типичном контексте агента:

> Пользователь вставляет описание функции в один абзац. Агент генерирует полный черновик PRD вместо пустой заглушки.

```json
{ "kind": "prd", "description": "Add OAuth2 login with Google and GitHub, support PKCE, 15m token TTL." }
```

## Типичная последовательность

`forgeplan_route` → `forgeplan_search` (проверка дубликатов) → `forgeplan_generate` → `forgeplan_get` → review/правка → `forgeplan_validate` → reasoning по выбранной глубине → реализация и evidence → review → `forgeplan_activate`. Генерация сама по себе не заменяет условия активации.

## Эквивалент CLI

- [`forgeplan generate`](/ru/docs/cli/generate/) - та же операция

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) - альтернатива с пустой заглушкой
- [`forgeplan_capture`](/ru/docs/mcp/forgeplan_capture/) - захват решений из беседы
- [`forgeplan_validate`](/ru/docs/mcp/forgeplan_validate/) - всегда валидировать сгенерированный вывод

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Generate an artifact using AI from a natural language description. Requires an LLM provider in .forgeplan/config.yaml. Supports OpenAI-compatible HTTP providers, Claude, Gemini, Ollama, and explicit subscription-authenticated local CLI providers.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `description` | да | string | Natural language description of what to generate |
| `kind` | да | ArtifactKindArg | Artifact kind to generate via LLM |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "ArtifactKindArg": {
      "enum": [
        "prd",
        "epic",
        "spec",
        "rfc",
        "adr",
        "problem",
        "solution",
        "evidence",
        "note",
        "refresh"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "description": {
      "description": "Natural language description of what to generate",
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/ArtifactKindArg",
      "description": "Artifact kind to generate via LLM"
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
    "kind",
    "description"
  ],
  "title": "GenerateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
