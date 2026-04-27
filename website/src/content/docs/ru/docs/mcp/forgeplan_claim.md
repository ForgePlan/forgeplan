---
title: forgeplan_claim
description: "Claim an artifact for exclusive work — TTL-based advisory lock for multi-agent dispatch."
---

Записывает `.forgeplan/claims/<id>.yaml`, объявляя, что конкретный агент работает над
артефактом. Удерживает workspace-lock на время записи, чтобы два суб-агента не могли
гоняться за одним и тем же клеймом. Падает с понятной ошибкой, когда другой агент уже
держит живой клейм; вызовы тем же агентом продлевают TTL (идемпотентно для держателя).
По дизайну рекомендательный — никакой другой инструмент не блокируется на клеймах, но
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
| `agent` | `string` | no | Идентичность агента (`"name/version"` или произвольная строка). По умолчанию — `clientInfo` MCP-вызывающего. |
| `ttl_minutes` | `number` | no (default 30, max 1440) | TTL в минутах. Жёсткий потолок 24 ч предотвращает зомби-клеймы. |
| `note` | `string` | no | Произвольная заметка, которую отображает `forgeplan_claims`. |

_Источник схемы: `crates/forgeplan-mcp/src/types.rs::ClaimParams`_

## Возвращает

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

При коллизии ответом будет ошибка с `_next_action`, советующим «либо работать над
другим артефактом, ждать истечения TTL, либо попросить оркестратора форс-релизнуть».

## Пример вызова

Работник клеймит с дефолтным TTL:

```json
{ "id": "PRD-057", "note": "implementing FR-003" }
```

Оркестратор клеймит от имени работника:

```json
{ "id": "RFC-012", "agent": "worker-2", "ttl_minutes": 60 }
```

Продление существующего клейма (тот же агент, любой TTL):

```json
{ "id": "PRD-057", "ttl_minutes": 30 }
```

## Типичная последовательность

1. [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) — оркестратор строит бакеты.
2. Работник вызывает `forgeplan_claim` на назначенном артефакте.
3. Работник делает реальные правки кода / артефакта.
4. [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) — снимает клейм по завершении.

## CLI эквивалент

[`forgeplan claim`](/ru/docs/cli/) — те же семантики, используется оркестраторами,
которые управляют суб-агентами через shell, а не через MCP.

## См. также

- [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) — снять активный клейм
- [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/) — список активных клеймов
- [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) — производит план работы, который защищают клеймы (полный протокол PRD-057)
