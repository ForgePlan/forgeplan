---
title: forgeplan_fpf_search
description: "Поиск в импортированной базе FPF по ключевым словам или кэшированным векторам BGE-M3 с явным предупреждением о fallback."
---

Поиск в импортированной **базе знаний FPF (First Principles Framework)**. Набор разделов зависит от корпуса, явно импортированного в workspace; для списка используйте `forgeplan_fpf_list`. `semantic: true` запрашивает векторный поиск BGE-M3 при подходящей сборке и кэшированной модели; иначе отвечает поиск по ключевым словам с предупреждением.

**Категория**: База знаний FPF

## Когда агент вызывает эту функцию

- **Поиск принципа** - "что FPF говорит о Trust Calculus?" → агенты часто обращаются к B.3.
- **Обоснование рассуждений ADI** - перед генерацией гипотез, получите соответствующий контекст FPF.
- **Поддержка принятия решений** - `fpf_search "explore exploit"` выводит правила баланса исследования/эксплуатации.
- **Онбординг** - помогите новому агенту быстро освоить терминологию FPF.

Инструмент читает вспомогательную таблицу FPF, не артефакты workspace и не тот же конвейер, что `forgeplan_search`. В выбранной локальной сборке чтение не скачивает веса. Захваченный каталог ниже ещё упоминает загрузку при первом вызове; handler использует проверенный кэшированный снапшот и при его отсутствии возвращается к ключевым словам. Для векторного поиска получите веса заранее отдельной явной операцией.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `query` | `string` | yes | После обрезки для проверки должен быть непустым; реализация ограничивает длину UTF-8 8192 байтами, несмотря на слово “chars” в каталоге. |
| `limit` | `integer \| null` | no (по умолчанию: `5`, предел: `50`) | Максимум возвращаемых совпадений; handler допускает ноль. |
| `semantic` | `boolean \| null` | no (по умолчанию: `false`) | Запрос кэшированного векторного поиска; `warning` сообщает о fallback. |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::FpfSearchParams`_

## Возвращает

Разобранный payload содержит `query`, `semantic`, `count`, `results`,
`warning`, `_next_action`. У совпадения есть `id`, `section_id`, `title`,
`snippet`, `line_count`; полей `score` и `path` нет.
Для `forgeplan_fpf_section` используйте `section_id`.

В этом handler `semantic` повторяет запрошенный флаг, в том числе при
fallback. Ненулевой `warning` сообщает о fallback; одного `semantic: true`
недостаточно для вывода о выполненном векторном поиске. Полей `mode`
и `total` нет. `_next_action` сообщает, если совпадений больше, чем вернул
лимит. Отсутствие таблицы FPF — ошибка с требованием явного `fpf ingest`,
а не успешный поиск без совпадений.

## Пример вызова

```json
{ "query": "trust calculus", "limit": 5 }
```

Семантический вариант:

```json
{ "query": "how do agents handle uncertainty", "semantic": true }
```

## Типичная последовательность

1. `forgeplan_fpf_search` с нужным вам понятием.
2. Передайте `section_id` лучшего совпадения в `forgeplan_fpf_section` для чтения тела.
3. Передать раздел в `forgeplan_reason` для рассуждений ADI с обоснованием FPF.

## Эквивалент CLI

```bash
forgeplan fpf search "trust calculus"
forgeplan fpf search "uncertainty" --semantic
```

## См. также

- [`forgeplan_fpf_section`](/ru/docs/mcp/forgeplan_fpf_section/) - получить раздел по ID.
- [`forgeplan_search`](/ru/docs/mcp/forgeplan_search/) - поиск артефактов рабочего пространства (не базы знаний FPF).
- [`forgeplan_reason`](/ru/docs/mcp/forgeplan_reason/) - рассуждения ADI с опциональным обоснованием `--fpf`.
- [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Search FPF (First Principles Framework) knowledge base. Default is keyword search. Pass `semantic: true` for vector similarity search via BGE-M3 embeddings (requires the `semantic-search` build feature). When `semantic: true` but the feature is not compiled in, the query gracefully falls back to keyword search and the response includes a `warning` field. Note: the first invocation with `semantic: true` may take several minutes if the BGE-M3 model needs to be downloaded (~2.1 GB). Params: query (required, 1..=8192 chars), limit (default 5, max 50), semantic (default false).

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `limit` | нет | integer \| null | Max results (default 5, max 50) |
| `query` | да | string | Search query — keyword or semantic depending on `semantic` flag |
| `semantic` | нет | boolean \| null | Use semantic (vector) search instead of keyword. Requires `semantic-search`<br>feature at build time. When the feature is not compiled in, the query<br>gracefully falls back to keyword search and the response includes a<br>`warning` field explaining why. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Exposed for integration test harness in tests/fpf_search_handler.rs.\nFields remain pub(crate) — only the struct itself is visible externally.\n`#[doc(hidden)]` because this is unstable test infrastructure, not a\nsupported public API (Sprint 13.7 hotfix re-audit M3).",
  "properties": {
    "limit": {
      "default": null,
      "description": "Max results (default 5, max 50)",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "query": {
      "description": "Search query — keyword or semantic depending on `semantic` flag",
      "type": "string"
    },
    "semantic": {
      "default": null,
      "description": "Use semantic (vector) search instead of keyword. Requires `semantic-search`\nfeature at build time. When the feature is not compiled in, the query\ngracefully falls back to keyword search and the response includes a\n`warning` field explaining why.",
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
  "required": [
    "query"
  ],
  "title": "FpfSearchParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
