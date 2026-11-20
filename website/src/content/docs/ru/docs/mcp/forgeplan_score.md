---
title: forgeplan_score
description: "Вычисляет показатель качества R_eff для артефакта на основе связанных доказательств. R_eff использует принцип слабого звена: показатель = min(показатели_доказательств). Пакеты с терминальным статусом (superseded/deprecated) остаются в ответе с флагом excluded: true и своей сырой оценкой, но в min не входят (ADR-020)."
---

Вычисляет показатель R_eff (эффективной надёжности) для артефакта, полученный из связанных с ним EvidencePack. R_eff следует принципу слабого звена - это минимум из показателей доказательств, а не среднее значение - поэтому один EvidencePack с CL0 может снизить общий показатель до 0.1. Агенты используют это, чтобы ответить на вопрос: "достаточно ли надёжно это решение для выпуска?" и выявить PRD, которым требуется больше доказательств.

**Категория**: Качество и валидация

## Когда агент вызывает это

- После создания и связывания EvidencePack - "действительно ли R_eff поднялся выше нуля?"
- В рамках глубокого анализа `forgeplan_health`: какие активные артефакты всё ещё имеют R_eff = 0?
- Перед предложением `forgeplan_activate` для глубокого/критического артефакта, где доказательства обязательны.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для оценки. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ScoreParams`_

## Возвращает

Ответ содержит `r_eff`, `self_score`, поля F-G-R, `weakest_link`, `factors`, `decay_penalty` и массив `evidence`. У каждого доказательства есть `verdict`, `congruence_level`, `score`, `expired`, `status`, `excluded`. Решение без текущих подтверждающих доказательств получает ноль; EvidencePack без зависимостей может иметь собственную оценку. Рекурсивный расчёт может ограничиваться и связанным артефактом, не только прямо прикреплённым EvidencePack.

`evidence[].score` использует плоскую собственную оценку; рекурсивный `r_eff` дополнительно учитывает тип доказательства и штрафы зависимостей, поэтому значения могут различаться. Вердикты в этом ответе — `Supports`/`Weakens`/`Refutes`; во входных полях доказательств значения вердикта пишутся строчными буквами. Просроченное допустимое доказательство получает 0.1 до вычисления вердикта, в том числе просроченный refutes-пакет. Безусловное утверждение о refutes в описании каталога ниже не отражает это исключение.

Учебный фрагмент ответа для двух непросроченных подтверждающих measurement-пакетов, не измеренная оценка:

```json
{
  "id": "PRD-042",
  "r_eff": 0.9,
  "weakest_link": "EVID-056",
  "evidence": [
    { "id": "EVID-057", "verdict": "Supports", "congruence_level": 3, "score": 1.0, "expired": false, "status": "active", "excluded": false },
    { "id": "EVID-056", "verdict": "Supports", "congruence_level": 2, "score": 0.9, "expired": false, "status": "active", "excluded": false }
  ]
}
```

## Пример вызова

```json
{ "id": "PRD-001" }
```

В типичном контексте агента:

> Агент только что связал EVID-057 с PRD-042 и хочет убедиться, что R_eff изменился с 0.00 на реальное число.

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

`forgeplan_new evidence` → `forgeplan_update` с буквальными ключами `verdict`, `congruence_level`, `evidence_type` → `forgeplan_link` → `forgeplan_score` → проверка всех условий активации перед `forgeplan_activate`. Положительная оценка сама по себе не означает приёмку активации. На legacy layout вызов сохраняет производную кешированную оценку; на migrated layout вычисляет её по захваченному каноническому представлению. Регистрация инструмента не доказывает проверку каждой раскладки или входа.

## Эквивалент CLI

- [`forgeplan score`](/ru/docs/cli/score/) - то же вычисление

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_link`](/ru/docs/mcp/forgeplan_link/) - как прикрепляются доказательства
- [`forgeplan_decay`](/ru/docs/mcp/forgeplan_decay/) - управление TTL доказательств
- [`forgeplan_health`](/ru/docs/mcp/forgeplan_health/) - агрегированная панель R_eff

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Compute R_eff quality score for an artifact based on linked evidence. R_eff uses the weakest-link principle: score = min(evidence_scores) over the artifact's CURRENT evidence — packs with a terminal status (superseded/deprecated) are excluded from the min (ADR-020) but stay listed with `excluded: true`. An active refutes pack still zeroes the score.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Artifact ID to score |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to score",
      "type": "string"
    },
    "workspace": {
      "default": null,
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
  "title": "ScoreParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
