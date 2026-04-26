---
title: forgeplan_release
description: "Release an active claim — drop the lock so other sub-agents can pick up the artifact."
---

Удаляет файл клейма по адресу `.forgeplan/claims/<id>.yaml`. По умолчанию вызов
отказывается работать, если клейм держит другой агент — передайте `force: true`
(escape hatch оркестратора), чтобы переопределить после краша суб-агента. Отсутствие
клейма — no-op (идемпотентно). Удерживает workspace-lock на время записи, чтобы
конкурентные claim/release-вызовы не могли чередоваться.

**Категория**: Multi-agent

## Когда агент вызывает

- Работник заканчивает артефакт и освобождает слот для следующего раунда диспатча.
- Работник упал / превысил TTL — оркестратор force-релизит с `agent: null, force: true`.
- Ошибочный клейм: агент схватил не тот ID, немедленно релизит для повтора.
- Уборка в конце сессии: пройтись по активным клеймам и снять каждый перед выходом.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта, чей клейм снять. |
| `agent` | `string` | no | Идентичность агента (должна совпадать с держателем, если не задано `force: true`). По умолчанию — `clientInfo` MCP-вызывающего. |
| `force` | `bool` | no (default `false`) | Force-release независимо от держателя — override оркестратора для упавших суб-агентов. |

_Источник схемы: `crates/forgeplan-mcp/src/types.rs::ReleaseParams`_

## Возвращает

```json
{
  "id": "PRD-057",
  "released": true,
  "force": false,
  "_next_action": "Released claim on `PRD-057`."
}
```

Сбой, когда не держатель и без `force`:

```json
{
  "ok": false,
  "error": "claim held by worker-2, not you",
  "_next_action": "Use `force: true` (orchestrator override) if the holder has crashed."
}
```

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
2. [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) → работник лочит голову своего бакета.
3. Работник делает работу с артефактом / кодом.
4. `forgeplan_release` → освободить слот.
5. Оркестратор пере-диспатчит.

## CLI эквивалент

[`forgeplan release <id>`](/ru/docs/cli/) — те же семантики.

## См. также

- [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) — взять клейм
- [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/) — посмотреть, кто что держит
- [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) — пере-диспатч после релиза
