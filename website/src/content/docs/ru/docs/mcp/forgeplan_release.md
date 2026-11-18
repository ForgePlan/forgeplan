---
title: forgeplan_release
description: "Снять рекомендательный захват артефакта, чтобы другой агент мог его получить."
---

Удаляет файл клейма по адресу `.forgeplan/claims/<id>.yaml`. По умолчанию вызов
отказывается работать, если клейм держит другой агент - передайте `force: true`
(escape hatch оркестратора), чтобы переопределить после краша суб-агента. Отсутствие
клейма - no-op (идемпотентно). Удерживает workspace-lock на время записи, чтобы
конкурентные claim/release-вызовы не могли чередоваться.

**Категория**: Multi-agent

## Когда агент вызывает

- Работник заканчивает артефакт и освобождает слот для следующего раунда диспатча.
- Работник упал / превысил TTL - оркестратор force-релизит с `agent: null, force: true`.
- Ошибочный клейм: агент схватил не тот ID, немедленно релизит для повтора.
- Уборка в конце сессии: пройтись по активным клеймам и снять каждый перед выходом.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта, чей клейм снять. |
| `agent` | `string \| null` | no | Явная identity держателя; иначе используется кэшированная identity MCP handshake. Без обеих identity нефорсированный вызов отклоняется. “Required unless force” в каталоге — условие handler, не обязательный JSON-ключ. |
| `force` | `bool` | no (default `false`) | Force-release независимо от держателя - override оркестратора для упавших суб-агентов. |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/types.rs::ReleaseParams`_

## Возвращает

Пример успеха ниже иллюстративный и опускает дополнительные поля workspace
trace. Захват — рекомендательный маркер, не блокировка правок исходников.

```json
{
  "id": "PRD-057",
  "released": true,
  "force": false,
  "_next_action": "Released claim on `PRD-057`."
}
```

Несовпадение держателя без `force` возвращает MCP tool error с текстом,
например `claim held by worker-2, not you`, и текстовой подсказкой
`_next_action`, не JSON-объект `{"ok":false,"error":...}`. Перед выбором
примера принудительного снятия подтвердите, что держатель действительно
прекратил работу.

## Пример вызова

Работник релизит после работы:

```json
{ "id": "PRD-057" }
```

Оркестратор подбирает упавшего суб-агента:

```json
{ "id": "RFC-012", "force": true }
```

Явная идентичность для shell-driven оркестраторов:

```json
{ "id": "SPEC-018", "agent": "worker-2" }
```

## Типичная последовательность

1. [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) → бакеты на агента.
2. [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) → работник записывает рекомендательное намерение работать над следующим артефактом.
3. Работник делает работу с артефактом / кодом.
4. `forgeplan_release` → освободить слот.
5. Оркестратор пере-диспатчит.

## CLI эквивалент

[`forgeplan release <id>`](/ru/docs/cli/) - те же семантики.

## См. также

- [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) - взять клейм
- [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/) - посмотреть, кто что держит
- [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) - пере-диспатч после релиза

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Release an active claim. Refuses if the claim is held by a different agent (use `force: true` to override — the orchestrator's escape hatch for a crashed sub-agent). Missing claim is a no-op (idempotent).

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `agent` | нет | string \| null | Required unless `force=true`. |
| `force` | нет | boolean | Force-release regardless of holder (orchestrator escape hatch). |
| `id` | да | string |  |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "agent": {
      "description": "Required unless `force=true`.",
      "type": [
        "string",
        "null"
      ]
    },
    "force": {
      "default": false,
      "description": "Force-release regardless of holder (orchestrator escape hatch).",
      "type": "boolean"
    },
    "id": {
      "type": "string"
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
  "title": "ReleaseParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
