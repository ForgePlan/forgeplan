---
title: forgeplan_blindspots
description: "Показать слепые пятна - решения (PRD/RFC/ADR/Epic) без связанных доказательств, а также сироты (артефакты без связей)."
---

Показать слепые пятна в рабочем пространстве - активные решения (PRD, RFC, ADR, Epic), которые не имеют связанных доказательств, а также сироты (артефакты без связей) с нулевым количеством входящих или исходящих связей. Слепые пятна - это главный сигнал технического долга в Forgeplan: `active` PRD без доказательств - это ложное обещание.

**Категория**: Панели мониторинга и Граф

## Когда агент вызывает это

- **После начала сессии** - если `forgeplan_health` сообщает о слепых пятнах или сиротах, запросите подробный список. Вердикта `debt` у отчёта здоровья нет.
- **Перед слиянием PR** - убедитесь, что новая работа не оставляет новых слепых пятен.
- **Во время рефакторинга** - найдите сироты (артефакты без связей) Note / Problem, которые следует отменить или перепривязать.
- **Проверки качества** - составьте список исправлений, сортируемый по владельцу / типу.

## Входные параметры

_Входные параметры отсутствуют. Вызовите этот инструмент с пустым объектом `{}`._

## Возвращает

Учебный фрагмент ответа, не измерение рабочего пространства. Слепые пятна представлены объектами с `id`, `title`, `issue`; `orphans` содержит строки ID.

```json
{
  "blind_spots": [
    {
      "id": "PRD-042",
      "title": "Система аутентификации",
      "issue": "No linked evidence — decision without proof"
    }
  ],
  "orphans": ["NOTE-017"],
  "total_blind_spots": 1,
  "total_orphans": 1
}
```

## Пример вызова

```json
{}
```

## Типичная последовательность

1. `forgeplan_blindspots` - перечисляет нарушителей.
2. Для каждого слепого пятна: `forgeplan_new` доказательство → `forgeplan_link` → `forgeplan_score`.
3. Для каждой сироты (артефакта без связей): либо `forgeplan_link` к родителю, либо `forgeplan_deprecate`.
4. Повторно запросите `forgeplan_health` и проверьте остальные предупреждения. Устранение этих двух списков само по себе не гарантирует `verdict: healthy`.

## Эквивалент CLI

```bash
forgeplan blindspots
```

## См. также

- [`forgeplan_health`](/ru/docs/mcp/forgeplan_health/) - полный отчёт о состоянии.
- [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) - создать EvidencePack.
- [`forgeplan_link`](/ru/docs/mcp/forgeplan_link/) - прикрепить доказательство к решению.
- [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Show blind spots — decisions (PRD/RFC/ADR/Epic) without linked evidence, and orphan artifacts with no connections.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| — | — | — | Параметров нет. |

<details>
<summary>JSON Schema</summary>

```json
{
  "properties": {},
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
