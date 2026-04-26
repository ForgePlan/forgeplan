---
title: forgeplan_claims
description: "List live claims in the workspace — who is working on what right now."
---

Возвращает каждый не истёкший клейм в `.forgeplan/claims/`, отсортированный по
возрастанию времени истечения (самые срочные — первыми). Пропускает клеймы за
пределами TTL — они считаются практически освобождёнными. По дизайну read-only и
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
      "expires_at": "2026-04-26T11:05:00Z",
      "note": null
    }
  ],
  "_next_action": "2 active claims. Use `forgeplan_dispatch --agents N` to plan ..."
}
```

`skipped > 0` означает, что хотя бы один файл клейма не удалось распарсить или он
превысил лимит размера — баг тихого молчаливого пропуска, отмеченный аудитом, теперь
явно всплывает. Запустите `forgeplan health`, чтобы найти нарушителя.

## Пример вызова

```json
{}
```

(`active` по умолчанию `false` — поле зарезервировано; передавать его не нужно.)

## Типичная последовательность

1. `forgeplan_claims` — посмотреть, кто занят.
2. [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) — спланировать с учётом живых клеймов.
3. Передать каждый бакет суб-агенту, который вызывает [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/).

## CLI эквивалент

[`forgeplan claims`](/ru/docs/cli/) — те же данные; оркестраторы, гоняющие
работников через shell, опрашивают эту команду.

## См. также

- [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) — получить клейм
- [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) — снять клейм
- [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) — multi-agent план работы (полный протокол PRD-057)
