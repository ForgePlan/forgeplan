---
title: forgeplan_claims
description: "List live claims in the workspace - who is working on what right now."
---

Возвращает каждый не истёкший клейм в `.forgeplan/claims/`, отсортированный по
возрастанию времени истечения (самые срочные - первыми). Пропускает клеймы за
пределами TTL - они считаются практически освобождёнными. По дизайну read-only и
без блокировок (audit-driven): оркестратор, опрашивающий с частотой 1 Гц, не должен
сериализовать запись суб-агентов. Битые файлы клеймов пропускаются со счётчиком,
чтобы health-проверки могли их подсветить.

**Категория**: Multi-agent

## Когда агент вызывает

- Оркестратор на каждом тике диспатча: «какая работа уже в полёте?».
- Суб-агент перед клеймом: «другой работник опередил меня по этому артефакту?».
- Health-проверки: ненулевой `skipped` сигнализирует о повреждённых файлах клеймов,
  которые стоит изучить.
- Session-start протокол после краша: листинг osиротевших клеймов, force-release
  мёртвых.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `active` | `bool` | no (default `false`) | Зарезервировано под будущие фильтры; сейчас всегда возвращает только живые клеймы. |

_Источник схемы: `crates/forgeplan-mcp/src/types.rs::ClaimsListParams`_

## Возвращает
Иллюстративный ответ; отсутствующее необязательное поле `note` не выводится:


```json
{
  "count": 2,
  "skipped": 0,
  "claims": [
    {
      "id": "PRD-057",
      "agent_id": "worker-1",
      "claimed_at": "2026-04-26T10:00:00Z",
      "expires_at": "2026-04-26T10:30:00Z",
      "note": "implementing FR-003"
    },
    {
      "id": "RFC-012",
      "agent_id": "worker-2",
      "claimed_at": "2026-04-26T10:05:00Z",
      "expires_at": "2026-04-26T11:05:00Z"
    }
  ],
  "_next_action": "2 active claim(s). Plan around them: `forgeplan_dispatch agents=3`."
}
```

`skipped > 0` означает, что хотя бы один файл не удалось разобрать или он превысил лимит размера.
Счётчик не указывает конкретный файл: найдите его в `.forgeplan/claims/`.

## Пример вызова

```json
{}
```

(`active` по умолчанию `false` - поле зарезервировано; передавать его не нужно.)

## Типичная последовательность

1. `forgeplan_claims` - посмотреть, кто занят.
2. [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) - спланировать с учётом живых клеймов.
3. Передать каждый бакет суб-агенту, который вызывает [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/).

## CLI эквивалент

[`forgeplan claims`](/ru/docs/cli/) - те же данные; оркестраторы, гоняющие
работников через shell, опрашивают эту команду.

## См. также

- [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) - получить клейм
- [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) - снять клейм
- [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) - multi-agent план работы (полный протокол PRD-057)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

List live claims in the workspace, sorted by expiry ascending. Skips expired claims (they're considered practically released). Used by orchestrators to build dispatch plans and by sub-agents to avoid double-claiming.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `active` | нет | boolean | Reserved for future filters; currently always returns only live claims. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "active": {
      "default": false,
      "description": "Reserved for future filters; currently always returns only live claims.",
      "type": "boolean"
    },
    "workspace": {
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "ClaimsListParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
