---
title: forgeplan_discover_complete
description: "Завершает сессию исследования существующей системы (brownfield discovery). ForgePlan группирует результаты по фазам и уровням, запускает forgeplan_health в рабочем пространстве и предлагает PROB / PRD / RFC, синтезированные из результатов. Предложенные артефакты выводятся на печать — не создаются автоматически — чтобы агент или человек могли их просмотреть перед фиксацией."
---

Закрывает активную сессию исследования, начатую с помощью `forgeplan_discover_start`. ForgePlan обрабатывает результаты, о которых сообщил агент, группирует их по фазам и уровням, выполняет проверку состояния проекта и синтезирует набор **предложенных** последующих артефактов (PROB для рисков, PRD для требований, RFC для описания реализации). Критически важно, что предложения **только выводятся на печать** — они не создаются автоматически. Агент (или человек) просматривает их и решает, какие из них перенести в рабочее пространство с помощью `forgeplan_new`.

**Категория**: Исследование существующей системы

## Когда агент вызывает эту функцию

- После прохождения всех семи фаз протокола и выдачи всех результатов.
- Когда пользователь говорит «завершить исследование» / «предложить следующие шаги» / «завершить сканирование».
- Перед началом любого цикла Shape → Validate → Code для вновь обнаруженной кодовой базы — предложения становятся основой бэклога.

## Что она делает

1. **Собирает** каждый артефакт-результат, помеченный сессией (через `discover:<session_id>`).
2. **Группирует** их по фазам и уровням, чтобы сводка отражала порядок приоритета источника.
3. **Запускает** `forgeplan health` в рабочем пространстве, чтобы выявить слепые пятна, сирот (артефакты без связей) и просроченные доказательства наряду со свежими результатами.
4. **Синтезирует** предложения, группируя связанные результаты:
   - Несколько результатов уровня 1 об одной и той же подсистеме → предложенный **RFC**.
   - Результаты, связанные с риском / нестабильностью / дрейфом → предложенный **PROB**.
   - Цели, ориентированные на пользователя, выведенные из тестов + намерений git → предложенный **PRD**.
5. **Помечает** сессию как `completed`, чтобы она была исключена из дальнейших вызовов `discover_finding`.

## Почему предложения не создаются автоматически

Автоматическое создание артефактов из результатов исследования заполнило бы рабочее пространство низкодоверительными заглушками и нарушило бы правило проекта «никогда не оставлять заглушки PRD». Вывод предложений на печать позволяет человеку (или агенту, с согласия пользователя) оставаться в курсе событий: только те элементы, которые получают одобрение, становятся реальными артефактами через `forgeplan_new`.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `session_id` | `string` | yes | Идентификатор сессии из `forgeplan_discover_start`. Должен быть активным. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::DiscoverCompleteParams`_

## Возвращает

Сводный отчёт и набор предложений:

```json
{
  "session_id": "discover-legacy-billing-service-…",
  "status": "completed",
  "findings_total": 27,
  "findings_by_phase": {
    "detect": 3, "structure": 4, "code": 9,
    "git": 4, "tests": 3, "docs": 4
  },
  "findings_by_tier": { "1": 12, "2": 5, "3": 4, "4": 6 },
  "health": { "blind_spots": 2, "orphans": 1, "stale_evidence": 0 },
  "proposed": [
    { "kind": "rfc",     "title": "Consolidate retry layers in billing engine",
      "rationale": "3 tier-1 findings describe overlapping exponential backoff." },
    { "kind": "problem", "title": "README drifted from src/auth — 4 claims unverified",
      "rationale": "Tier-4 vs tier-1 reconciliation mismatch." },
    { "kind": "prd",     "title": "Formalise idempotency guarantees on checkout",
      "rationale": "Tests imply exactly-once semantics not reflected in code or docs." }
  ],
  "next_steps": [
    "Review each proposal with the user.",
    "For each accepted proposal: forgeplan_new <kind> <title>.",
    "Start Shape → Validate cycle on the first PRD."
  ]
}
```

## Пример вызова

```json
{ "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z" }
```

## Типичная последовательность

```
discover_start → …many discover_finding calls… → discover_complete
                                                    ↓ просмотр предложений
                                                 forgeplan_new (по одному на каждое принятое предложение)
                                                    ↓
                                                 forgeplan_validate / forgeplan_reason …
```

## Эквивалент CLI

- [`forgeplan discover complete`](/ru/docs/cli/discover-complete/) — та же финализация из терминала.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_discover_start`](/ru/docs/mcp/forgeplan_discover_start/) — начать сессию
- [`forgeplan_discover_finding`](/ru/docs/mcp/forgeplan_discover_finding/) — сообщать наблюдения
- [`forgeplan_health`](/ru/docs/mcp/forgeplan_health/) — снимок состояния, включённый в сводку
- [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) — перенести предложение в реальный артефакт
