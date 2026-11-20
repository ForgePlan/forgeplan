---
title: forgeplan_search
description: "Умный поиск по артефактам: поиск по ключевым словам BM25 + опциональный семантический поиск + расширение графа. Поддерживает фильтры по типу/статусу/глубине/доказательствам/дате создания и переключение расширения графа."
---

Находит артефакты workspace без чтения каждого полного тела. Описание BM25/морфологии v0.18.0 — исторический контекст, не утверждение об одном конвейере всех текущих режимов. В выбранном локальном MCP handler keyword использует поиск подстроки без учёта регистра; остальные строки режима идут по общему smart-search пути.

**Категория**: Чтение артефактов

## Когда агент вызывает эту функцию

- Пользователь спрашивает: «У нас уже есть решение о повторных попытках?» - агент выполняет поиск, прежде чем создать новый ADR.
- Проверка на дубликаты перед `forgeplan_new`: убедиться, что ни один существующий PRD уже не охватывает эту тему.
- Сбор контекста для `forgeplan_reason`: извлечь 3-5 связанных артефактов для инициализации запроса ADI.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `query` | `string` | yes | Поисковый запрос (поиск по ключевым словам BM25 + опциональный семантический поиск, без учета регистра). |
| `kind` | `string` | no | Фильтр по типу артефакта (например, `prd`, `rfc`). |
| `status` | `string` | no | Фильтр по статусу (например, `active`, `draft`). |
| `depth` | `string` | no | Фильтр по глубине (`tactical`, `standard`, `deep`, `critical`). |
| `with_evidence` | `bool` | no (по умолчанию: `false`) | Включать только артефакты со связанными доказательствами (R_eff > 0). |
| `no_evidence` | `bool` | no (по умолчанию: `false`) | Включать только артефакты без доказательств (R_eff == 0). |
| `since` | `string` | no | Фильтр по дате `created_at` (ГГГГ-ММ-ДД). |
| `no_expand` | `bool` | no (по умолчанию: `false`) | Отключить расширение графа на 1 переход для лучших результатов. |
| `limit` | `integer` | no (по умолчанию: `20`) | Максимальное количество возвращаемых результатов. |
| `mode` | `string` | no | Режим поиска: `keyword`, `semantic` или `smart` (по умолчанию). |
| `workspace` | `string?` | no | Абсолютный путь или `~/`. Приоритетнее `FORGEPLAN_WORKSPACE`, затем default сервера, затем поиска от CWD по родителям. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::SearchParams`_

## Возвращает

MCP `CallToolResult` содержит JSON-текст в `content[]`. Разобранный payload -
объект `SearchResponse`, не массив верхнего уровня:

- `results`: ранжированные записи с `id`, `kind`, `title`, `status`,
  `matched_lines`, `score`, `bm25_score`, `semantic_score`, `r_eff`,
  `id_canonical`, `id_display`; необязательные `slug`, `predicted_number`,
  `assigned_number`, `expanded_from`.
- `total`: число результатов.
- `coverage`: booleans `semantic`, `lexical`, `graph`, отражающие состояние каналов. В smart-пути `graph` задаётся как `!no_expand` даже при ошибке получения графа; это не доказательство вклада соседей в данный ответ.

Для отключения expansion передайте `no_expand: true`; входа `expand_graph`
нет. Соседи указывают родителя через `expanded_from`, не `via`.
Совпавший текст находится в `matched_lines`, не `snippet`.

В локальной `0.39.0+constellation.s1.1` запросы не скачивают недостающие
веса. В отличие от forced-semantic контракта CLI, этот MCP handler
не имеет отдельной forced-semantic ветки: `semantic` и даже неизвестные
строки режима попадают в общий smart-путь. Изучайте возвращённый coverage.

Keyword учитывает `kind` и лимит результатов, но не применяет построенные
фильтры `status`, `depth`, evidence, `since`; расширения графа тоже нет.
Если эти фильтры важны, используйте smart-путь и проверяйте результат,
не считая схему доказательством исполнения. У smart-результатов сейчас
пустые `matched_lines`; читайте тела через `forgeplan_get`.

Лимит по умолчанию 20 (`0` тоже преобразуется в 20); отдельный pinned
Constellation search по умолчанию даёт 10. См. [настройку Constellation](/ru/docs/guides/constellation/)
и [контракты](/ru/docs/reference/constellation/).

## Пример вызова

```json
{ "query": "authentication flow", "limit": 5, "workspace": "/path/to/project" }
```

В типичном контексте агента:

> Прежде чем предложить новую стратегию повторных попыток, агент ищет любое существующее решение по этой теме.

```json
{ "query": "retry backoff strategy", "kind": "adr", "limit": 10 }
```

## Типичная последовательность

`forgeplan_search` → агент выбирает лучший результат → `forgeplan_get` для чтения полного содержимого → принимает решение об изменении, замещении или создании нового. В протоколе сессии Forgeplan `search` является обязательной предварительной проверкой перед любым `new` для предотвращения дублирования.

## Эквивалент в CLI

- [`forgeplan search`](/ru/docs/cli/search/) - тот же движок, вывод в терминал

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [Руководство по поиску BM25 в v0.18.0](/ru/docs/guides/search-v2/)
- [`forgeplan_fpf_search`](/ru/docs/mcp/forgeplan_fpf_search/) - семантический поиск FPF KB
- [`forgeplan_discover_start`](/ru/docs/mcp/forgeplan_discover_start/) - исследовательский поиск

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Smart search across artifacts: BM25 keyword + optional semantic + graph expansion. Supports filters by kind/status/depth/evidence/since and graph expansion toggle.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `depth` | нет | string \| null | Filter by depth (tactical, standard, deep, critical). |
| `kind` | нет | string \| null | Filter by artifact kind (e.g. "prd", "rfc"). |
| `limit` | нет | integer | Max results (default 20). |
| `mode` | нет | string \| null | Search mode: "keyword", "semantic", or "smart" (default). |
| `no_evidence` | нет | boolean | Only include artifacts without evidence (R_eff == 0). |
| `no_expand` | нет | boolean | Disable 1-hop graph expansion of top results. |
| `query` | да | string | Search query (BM25 keyword + optional semantic, case-insensitive). |
| `since` | нет | string \| null | Filter by created_at date (YYYY-MM-DD). |
| `status` | нет | string \| null | Filter by status (e.g. "active", "draft"). |
| `with_evidence` | нет | boolean | Only include artifacts with linked evidence (R_eff &gt; 0). |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "depth": {
      "default": null,
      "description": "Filter by depth (tactical, standard, deep, critical).",
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "default": null,
      "description": "Filter by artifact kind (e.g. \"prd\", \"rfc\").",
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "default": 20,
      "description": "Max results (default 20).",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "mode": {
      "default": null,
      "description": "Search mode: \"keyword\", \"semantic\", or \"smart\" (default).",
      "type": [
        "string",
        "null"
      ]
    },
    "no_evidence": {
      "default": false,
      "description": "Only include artifacts without evidence (R_eff == 0).",
      "type": "boolean"
    },
    "no_expand": {
      "default": false,
      "description": "Disable 1-hop graph expansion of top results.",
      "type": "boolean"
    },
    "query": {
      "description": "Search query (BM25 keyword + optional semantic, case-insensitive).",
      "type": "string"
    },
    "since": {
      "default": null,
      "description": "Filter by created_at date (YYYY-MM-DD).",
      "type": [
        "string",
        "null"
      ]
    },
    "status": {
      "default": null,
      "description": "Filter by status (e.g. \"active\", \"draft\").",
      "type": [
        "string",
        "null"
      ]
    },
    "with_evidence": {
      "default": false,
      "description": "Only include artifacts with linked evidence (R_eff > 0).",
      "type": "boolean"
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
  "title": "SearchParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
