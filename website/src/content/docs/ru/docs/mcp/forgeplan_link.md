---
title: forgeplan_link
description: "Связывает два артефакта типизированным отношением. Допустимые типы: informs, based_on, supersedes, contradicts, refines."
---

Создаёт типизированное отношение между двумя артефактами. Связи — это то, как Forgeplan строит свой граф зависимостей; они используются для отчётов о состоянии, расчёта R_eff, топологической сортировки и визуальных графов. Агент вызывает эту функцию каждый раз, когда он создаёт подтверждающее доказательство, замещающий RFC или дочерний PRD, который наследует от Epic.

**Категория**: Редактирование артефактов

## Когда агент вызывает эту функцию

- После создания EvidencePack: связывает `EVID-XXX` → `PRD-YYY` отношением `informs`, чтобы можно было рассчитать R_eff.
- При декомпозиции Epic: связывает каждый новый PRD обратно с Epic отношением `based_on`.
- При замене дизайна: связывает новый RFC со старым отношением `supersedes` (дополняет `forgeplan_supersede`).

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `source` | `string` | yes | ID исходного артефакта. |
| `target` | `string` | yes | ID целевого артефакта. |
| `relation` | `string` | no (по умолчанию: `"informs"`) | Тип отношения: `informs`, `based_on`, `supersedes`, `contradicts`, `refines`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::LinkParams`_

## Возвращает

Подтверждение с сохранённым ребром. Граф обновляется немедленно и отобразится при следующем вызове `forgeplan_graph` / `forgeplan_health` / `forgeplan_score`.

Пример структуры ответа:

```json
{
  "ok": true,
  "source": "EVID-057",
  "target": "PRD-042",
  "relation": "informs"
}
```

## Пример вызова

```json
{ "source": "EVID-001", "target": "PRD-001", "relation": "informs" }
```

С типичным контекстом агента:

> Агент завершил реализацию, создал EVID-057 с результатами бенчмарков и теперь связывает его с PRD, чтобы оценка стала зелёной.

```json
{ "source": "EVID-057", "target": "PRD-042", "relation": "informs" }
```

## Типичная последовательность

`forgeplan_new` (доказательство) → `forgeplan_update` (структурированные поля) → `forgeplan_link` → `forgeplan_score` (теперь > 0) → `forgeplan_activate`. Для декомпозиции Epic: `forgeplan_new` (epic) → `forgeplan_new` (prd) → `forgeplan_link relation=based_on` → повторить.

## Эквивалент в CLI

- [`forgeplan link`](/ru/docs/cli/link/) — та же операция, позиционные аргументы

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_score`](/ru/docs/mcp/forgeplan_score/) — использует граф связей для R_eff
- [`forgeplan_graph`](/ru/docs/mcp/forgeplan_graph/) — визуализирует граф связей
- [`forgeplan_supersede`](/ru/docs/mcp/forgeplan_supersede/) — версия связи `supersedes` с сохранением состояния
