---
title: forgeplan_fpf_check
description: "Оценивает набор правил FPF для конкретного артефакта. Возвращает каждое совпавшее правило, выигрышное правило (первое в порядке приоритета — как и во время выполнения), правила, которые не совпали, и рекомендуемую категорию действий. Используйте это, чтобы ответить на вопрос 'что мне делать дальше с этим артефактом и почему?'"
---

Запускает активные правила FPF для одного артефакта и возвращает всё, что обнаружил движок: какие правила совпали, какие нет, какое правило выиграло в порядке приоритета, и рекомендуемую категорию действий (`EXPLORE` / `INVESTIGATE` / `EXPLOIT`). Это инструмент агента «что мне делать дальше?» — он превращает абстрактный набор правил из `forgeplan_fpf_rules` в конкретную рекомендацию, привязанную к реальному артефакту.

**Категория**: База знаний FPF

## Когда агент вызывает это

- После того как пользователь спрашивает «какое моё следующее действие по PRD-042?» — `message` выигрышного правила является ответом.
- Во время цикла ревью — для проверки каждого активного артефакта, чтобы выявить те, что застряли ниже порога EXPLORE.
- Для отладки неожиданного вывода `forgeplan_reason` или `forgeplan_health` — `fpf_check` показывает точное правило, вызвавшее рекомендацию.
- Перед `forgeplan_activate` — для подтверждения, что артефакт находится в категории EXPLOIT, а не EXPLORE.

## Как это работает

1. Загружает действующий набор правил (переопределения конфигурации > значения по умолчанию), тот же путь, что и у `forgeplan_fpf_rules`.
2. Извлекает артефакт по ID (frontmatter + R_eff + ссылки на доказательства).
3. Оценивает дерево условий каждого правила относительно состояния артефакта.
4. Собирает совпадения и сортирует по `priority` — **первое совпадение выигрывает**, точно так же, как это делает движок во время выполнения.
5. Сообщает `action` выигрышного правила как рекомендуемую категорию вместе с его `message`.

Пороги, определяющие категории, берутся из `fpf.thresholds` в конфигурации (`explore_reff`, `investigate_reff`, `exploit_reff`). Глубина влияет на пороги: критическому артефакту требуется более сильное доказательство для достижения EXPLOIT, чем тактическому, поэтому один и тот же R_eff может попасть в разные категории в зависимости от глубины.

## Входные параметры

| Name | Type | Required | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта (без учёта регистра), например, `PRD-042`, `RFC-007`, `ADR-003`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::FpfCheckParams`_

## Возвращает

```json
{
  "artifact": {
    "id": "PRD-042",
    "kind": "prd",
    "status": "draft",
    "depth": "deep",
    "r_eff": 0.28
  },
  "winning_rule": {
    "name": "low_trust_explore",
    "priority": 10,
    "action": "EXPLORE",
    "message": "R_eff 0.28 < 0.33 explore threshold — add evidence or narrow scope before activation."
  },
  "matched": [
    { "name": "low_trust_explore", "priority": 10, "action": "EXPLORE" },
    { "name": "draft_needs_adi",   "priority": 50, "action": "INVESTIGATE" }
  ],
  "unmatched": [
    { "name": "high_trust_exploit", "priority": 30, "action": "EXPLOIT",
      "reason": "r_eff (0.28) < exploit_reff (0.66)" }
  ],
  "thresholds": {
    "explore_reff":     0.33,
    "investigate_reff": 0.66,
    "exploit_reff":     0.66,
    "depth_adjustment": "+0.10 for deep"
  }
}
```

Если ID не существует, инструмент возвращает ошибку, чтобы агент мог вернуться к `forgeplan_search`.

## Пример вызова

```json
{ "id": "PRD-042" }
```

## Типовая последовательность

```
forgeplan_list (status=draft)       ← найти кандидатов
forgeplan_fpf_check { id: "PRD-X" } ← какая рекомендуемая категория?
  → EXPLORE:     forgeplan_new evidence + forgeplan_link
  → INVESTIGATE: forgeplan_reason + добавить измерения
  → EXPLOIT:     forgeplan_review → forgeplan_activate
```

## Эквивалент CLI

- [`forgeplan fpf check <ID>`](/docs/cli/fpf-check/) — идентичный вывод, отображаемый в терминале.

## См. также

- [Обзор MCP](/docs/mcp/)
- [`forgeplan_fpf_rules`](/docs/mcp/forgeplan_fpf_rules/) — перечень правил, оцениваемых здесь
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) — вычисляет R_eff, с которым сопоставляются правила
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) — рассуждения ADI, дополняющие проверки на основе правил
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) — завершающее действие, как только артефакт достигает EXPLOIT
