---
title: forgeplan_discover_finding
description: "Записывает одно наблюдение как черновик артефакта в сессии исследования. Только MCP; анализ источников выполняет агент."
---

Добавляет **наблюдение** в существующую сессию исследования. Агент следует семифазному протоколу и сообщает результаты; Forgeplan создаёт черновик артефакта и записывает его ID в сессию. Завершайте исследование перед отметкой completed: текущий обработчик не обеспечивает запрет записи в завершённую сессию.

**Категория**: Обнаружение в Brownfield

> **Только MCP по замыслу.** Этот инструмент не имеет эквивалента в `forgeplan` CLI. Протокол исследования *управляется агентом* - ForgePlan не читает исходные файлы самостоятельно. Оператор-человек, используя CLI, запускает сессию, а затем запускает агента; обнаруженные факты возвращаются только через MCP.

## Когда агент вызывает этот инструмент

- После прочтения исходного файла и выявления шаблона, зависимости, границы или странности, которые стоит зафиксировать.
- После анализа `git log` и обнаружения отката, коммита с решением или заброшенной ветки.
- После запуска тестов и обнаружения поведенческого контракта, который не был явно выражен в самом коде.
- После сверки `README.md` с кодом и обнаружения расхождений (обнаруженный факт становится артефактом `ProblemCard`).

## Уровни источников (напоминание)

У каждого наблюдения есть уровень источника. Выбранная реализация принимает только `1`, `2`, `3`:

| Уровень | Источники | Типичный вид |
|---------|-----------|--------------|
| 1 | Код, история Git, манифесты пакетов | `note`, `rfc`, `problem` |
| 2 | Тесты, JSDoc-комментарии, CI-конфигурация | `evidence` |
| 3 | Документация, README, wiki | `note`, `problem` |

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `session_id` | `string` | yes | Дескриптор сессии, возвращённый `forgeplan_discover_start`. |
| `phase` | `string` | yes | Одна из фаз: `detect`, `structure`, `code`, `git`, `tests`, `docs`, `synthesize`. |
| `tier` | `integer` | да | Уровень `1`, `2`, `3`; значение `4` отклоняется. |
| `kind` | `string` | да | Тип: `note` / `prd` / `rfc` / `problem` / `evidence`; передавайте программные значения, не переведённые названия. |
| `title` | `string` | yes | Краткий, конкретный заголовок для обнаруженного факта (используется как заголовок артефакта). |
| `body` | `string` | yes | Тело в формате Markdown. Включите: что вы наблюдали, где (файл:строка) и почему это важно. |
| `source_files` | `string[]` | no (по умолчанию: `[]`) | Пути, которые послужили основой для обнаруженного факта. Записываются, чтобы рецензенты могли проследить. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::DiscoverFindingParams`_

## Возвращает

Возвращает `artifact_id`, `session_id`, `phase`, `tier`, `total_findings`, `session_status`, `artifact_status`, предупреждения, подсказку и метаданные согласования индекса.

Иллюстративный фрагмент:

```json
{
  "artifact_id": "NOTE-087",
  "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z",
  "phase": "code",
  "tier": 1,
  "total_findings": 1,
  "session_status": "active",
  "artifact_status": "draft"
}
```

Теги: `source=tier<N>`, `phase=<phase>`, `discover-session=<session_id>`; для уровня 3 добавляется `source=legacy-doc`. Это метаданные, а не типизированные связи между артефактами. Устаревающее поле `status` относится к сессии.

## Пример вызова

Иллюстративный вход: эти пути не являются зафиксированным наблюдением в данном репозитории:

```json
{
  "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z",
  "phase": "code",
  "tier": 1,
  "kind": "note",
  "title": "Billing engine uses two overlapping retry layers",
  "body": "Учебное наблюдение: `src/billing/retry.rs` и `src/http/client.rs` настраивают повторы. Перед изменением любого слоя проверьте вложенность и измерьте фактическое число попыток и время.",
  "source_files": ["src/billing/retry.rs", "src/http/client.rs"]
}
```

## Типичная последовательность

```
discover_start → (many) discover_finding → discover_complete
```

Фиксируйте одно наблюдение на один конкретный факт, не объединяя несвязанные проблемы. Число зависит от исследования; измеренный «типичный диапазон» здесь не заявляется.

## Эквивалент CLI

Нет - намеренно. Обнаруженные факты исследования передаются только через путь MCP.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_discover_start`](/ru/docs/mcp/forgeplan_discover_start/) - протокол, который является источником этого инструмента
- [`forgeplan_discover_complete`](/ru/docs/mcp/forgeplan_discover_complete/) — завершить сессию; предложения следующих работ агент синтезирует отдельно
- [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) - альтернатива для создания артефактов вне сессии

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Report a discovery finding. The agent calls this after analyzing a file/module/git-log during a phase. ForgePlan creates an artifact (note/prd/rfc/problem/evidence) with the finding content, tags it with the source tier, and links it to the discovery session.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `body` | да | string | Artifact body (markdown) |
| `kind` | да | string | Artifact kind to create (note / prd / rfc / problem / evidence) |
| `phase` | да | string | Phase (detect / structure / code / git / tests / docs / synthesize) |
| `session_id` | да | string | Session ID returned by discover_start |
| `source_files` | нет | array | Source file paths that informed this finding |
| `tier` | да | integer | Source tier (1, 2, or 3) |
| `title` | да | string | Artifact title |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "body": {
      "description": "Artifact body (markdown)",
      "type": "string"
    },
    "kind": {
      "description": "Artifact kind to create (note / prd / rfc / problem / evidence)",
      "type": "string"
    },
    "phase": {
      "description": "Phase (detect / structure / code / git / tests / docs / synthesize)",
      "type": "string"
    },
    "session_id": {
      "description": "Session ID returned by discover_start",
      "type": "string"
    },
    "source_files": {
      "default": [],
      "description": "Source file paths that informed this finding",
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "tier": {
      "description": "Source tier (1, 2, or 3)",
      "format": "uint8",
      "maximum": 255,
      "minimum": 0,
      "type": "integer"
    },
    "title": {
      "description": "Artifact title",
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
    "session_id",
    "phase",
    "tier",
    "kind",
    "title",
    "body"
  ],
  "title": "DiscoverFindingParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
