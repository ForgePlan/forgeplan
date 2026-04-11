---
title: forgeplan_blindspots
description: "Показать слепые пятна — решения (PRD/RFC/ADR/Epic) без связанных доказательств, а также сироты (артефакты без связей)."
---

Показать слепые пятна в рабочем пространстве — активные решения (PRD, RFC, ADR, Epic), которые не имеют связанных доказательств, а также сироты (артефакты без связей) с нулевым количеством входящих или исходящих связей. Слепые пятна — это главный сигнал технического долга в Forgeplan: `active` PRD без доказательств — это ложное обещание.

**Категория**: Панели мониторинга и Граф

## Когда агент вызывает это

- **После начала сессии** — если `forgeplan_health` сообщает `verdict: debt`, вызовите это для получения подробного списка.
- **Перед слиянием PR** — убедитесь, что новая работа не оставляет новых слепых пятен.
- **Во время рефакторинга** — найдите сироты (артефакты без связей) Note / Problem, которые следует отменить или перепривязать.
- **Проверки качества** — составьте список исправлений, сортируемый по владельцу / типу.

## Входные параметры

_Входные параметры отсутствуют. Вызовите этот инструмент с пустым объектом `{}`._

## Возвращает

```json
{
  "blind_spots": [
    {
      "id": "PRD-042",
      "kind": "prd",
      "status": "active",
      "reason": "no linked evidence",
      "r_eff": 0.0
    },
    {
      "id": "RFC-006",
      "kind": "rfc",
      "status": "active",
      "reason": "no linked evidence"
    }
  ],
  "orphans": [
    {
      "id": "NOTE-017",
      "kind": "note",
      "reason": "no incoming or outgoing links"
    }
  ],
  "summary": {
    "blind_spots_count": 2,
    "orphans_count": 1
  }
}
```

## Пример вызова

```json
{}
```

## Типичная последовательность

1. `forgeplan_blindspots` — перечисляет нарушителей.
2. Для каждого слепого пятна: `forgeplan_new` доказательство → `forgeplan_link` → `forgeplan_score`.
3. Для каждой сироты (артефакта без связей): либо `forgeplan_link` к родителю, либо `forgeplan_deprecate`.
4. Повторно запустите `forgeplan_health` → ожидайте `verdict: healthy`.

## Эквивалент CLI

```bash
forgeplan blindspots
```

## См. также

- [`forgeplan_health`](/docs/mcp/forgeplan_health/) — полный отчёт о состоянии.
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — создать EvidencePack.
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) — прикрепить доказательство к решению.
- [Руководство по методологии](/docs/methodology/overview/)