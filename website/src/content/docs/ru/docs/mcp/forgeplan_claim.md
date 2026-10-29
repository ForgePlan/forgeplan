---
title: forgeplan_claim
description: "Claim an artifact for exclusive work - TTL-based advisory lock for multi-agent dispatch."
---

Записывает `.forgeplan/claims/<id>.yaml`, объявляя, что конкретный агент работает над
артефактом. Удерживает workspace-lock на время записи, чтобы два суб-агента не могли
гоняться за одним и тем же клеймом. Падает с понятной ошибкой, когда другой агент уже
держит живой клейм; вызовы тем же агентом продлевают TTL и обновляют временные метки.
По дизайну рекомендательный - никакой другой инструмент не блокируется на клеймах, но
оркестраторы должны проверять [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/)
перед диспатчем параллельной работы.

**Категория**: Multi-agent

## Когда агент вызывает

- Суб-агент берёт артефакт из бакета `forgeplan_dispatch` и клеймит его перед
  тем, как трогать файлы.
- Долгая работа (R3-grade рефакторинг, multi-PR feature): продлевает клейм тем же
  вызовом до истечения TTL.
- Оркестратор клеймит от имени работника, явно передавая `agent: "worker-1"`.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для клейма (например, `PRD-057`). На диске нормализуется к верхнему регистру. |
| `agent` | `string` | нет | Идентификатор (`"name/version"` или строка) — не более 64 UTF-8 байт после удаления крайних пробелов. По умолчанию берётся `clientInfo`; без обоих источников идентичности вызов завершается ошибкой. Управляющие и форматирующие символы запрещены. |
| `ttl_minutes` | `integer` | нет (по умолчанию 30) | От `1` до `1440` минут; значения вне диапазона отклоняются. Для долгой работы продлевайте захват. |
| `note` | `string` | no | Произвольная заметка, которую отображает `forgeplan_claims`. |

_Источник схемы: `crates/forgeplan-mcp/src/types.rs::ClaimParams`_

## Возвращает
Иллюстративный ответ:


```json
{
  "id": "PRD-057",
  "agent_id": "worker-1",
  "claimed_at": "2026-04-26T10:00:00Z",
  "expires_at": "2026-04-26T10:30:00Z",
  "note": "implementing FR-003",
  "_next_action": "Claimed `PRD-057` for `worker-1`. Release with `forgeplan_release PRD-057` ..."
}
```

При коллизии обработчик возвращает ошибку и рекомендует выбрать другой артефакт
через `forgeplan_dispatch`. Не отбирайте чужой захват принудительно ради обхода отказа.

## Пример вызова

Работник клеймит с дефолтным TTL:

```json
{ "id": "PRD-057", "note": "implementing FR-003" }
```

Оркестратор клеймит от имени работника:

```json
{ "id": "RFC-012", "agent": "worker-2", "ttl_minutes": 60 }
```

Продление существующего клейма (тот же агент, TTL в допустимом диапазоне):

```json
{ "id": "PRD-057", "ttl_minutes": 30 }
```

## Типичная последовательность

1. [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) - оркестратор строит бакеты.
2. Работник вызывает `forgeplan_claim` на назначенном артефакте.
3. Работник делает реальные правки кода / артефакта.
4. [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) - снимает клейм по завершении.

## CLI эквивалент

[`forgeplan claim`](/ru/docs/cli/) использует то же хранилище рекомендательных захватов,
но по умолчанию берёт идентичность CLI, не MCP `clientInfo`. Явный CLI `--agent`
не допускает `/`: используйте, например, `worker-1`, и того же держателя при release.

## См. также

- [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) - снять активный клейм
- [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/) - список активных клеймов
- [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) - производит план работы, который защищают клеймы (полный протокол PRD-057)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Claim an artifact for exclusive work. Writes .forgeplan/claims/<id>.yaml with the caller's agent identity, timestamp, and TTL. Fails if a live claim by a different agent exists — same-agent calls renew the TTL. Advisory: other tools do not block on claims, but orchestrators should use `forgeplan_claims --active` to avoid double-assigning work.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `agent` | нет | string \| null | Agent identity ("name/version" or free-form). Optional — defaults to<br>the MCP caller's clientInfo when omitted. Providing this explicitly<br>lets an orchestrator claim on behalf of a sub-agent. |
| `id` | да | string | Artifact ID to claim (e.g. `PRD-057`). Normalized to uppercase on disk. |
| `note` | нет | string \| null | Optional free-form note surfaced by `forgeplan_claims --active`. |
| `ttl_minutes` | нет | integer \| null | Time-to-live in minutes. Default 30, max 1440 (24h), min 1. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "agent": {
      "description": "Agent identity (\"name/version\" or free-form). Optional — defaults to\nthe MCP caller's clientInfo when omitted. Providing this explicitly\nlets an orchestrator claim on behalf of a sub-agent.",
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "description": "Artifact ID to claim (e.g. `PRD-057`). Normalized to uppercase on disk.",
      "type": "string"
    },
    "note": {
      "description": "Optional free-form note surfaced by `forgeplan_claims --active`.",
      "type": [
        "string",
        "null"
      ]
    },
    "ttl_minutes": {
      "description": "Time-to-live in minutes. Default 30, max 1440 (24h), min 1.",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "workspace": {
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
  "title": "ClaimParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
