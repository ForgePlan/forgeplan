---
title: forgeplan_health
description: "Показать дашборд состояния проекта - пробелы, риски, слепые пятна, сироты (артефакты без связей), просроченные доказательства и рекомендуемые дальнейшие действия. LLM не требуется."
---

Показать дашборд состояния проекта - пробелы, риски, слепые пятна, сироты (артефакты без связей), просроченные доказательства и рекомендуемые дальнейшие действия. Это самый важный инструмент MCP для ИИ-агента: вызывайте его в начале сессии, чтобы понять состояние проекта перед началом любой работы.

**Категория**: Дашборды и Графы

## Когда агент вызывает инструмент

- **Начало сессии** - первый вызов после `memory_recall` для понимания текущего состояния проекта.
- **Перед началом новой работы** - проверьте, нет ли долгов (слепых пятен, сирот, просроченных артефактов), которые следует исправить в первую очередь.
- **После спринта** - убедитесь, что ничего не ухудшилось (новые слепые пятна, осиротевшие доказательства).
- **Перед релизом** - убедитесь, что состояние чистое.

Инструмент быстрый, детерминированный и не требует LLM. Всегда предпочитайте его `list` + ручной проверке.

## Входные параметры

_Входные параметры отсутствуют. Вызовите этот инструмент с пустым объектом `{}`._

## Возвращает

JSON-текст payload включает:

- `total`, `by_kind`, `by_status`, `by_derived_status`: количество и разбивки.
- `blind_spots`, `at_risk`, `orphans`, `stale_count`: находки по доверию, связям и срокам; `orphans` — массив ID.
- `possible_duplicates`, `active_stubs`, `stale_drafts`, `gitignore_drift`: дополнительные находки.
- `phase_mismatches`, `advisory_phase_mismatches`, `phase_read_errors`: диагностику рекомендательных фаз.
- `active_claims`, `active_claim_count`, `skipped_claim_files`: сведения MCP о захватах.
- `next_actions`, `_next_action`, `verdict_summary`: рекомендации и пояснение.
- `verdict`: только `"healthy"`, `"needs_attention"` или `"unhealthy"`; **не** `"debt"`.

Не используйте прежние примерные поля `total_artifacts`, `risks`, `stale`
для ветвления. Дашборд объединяет реализованные проверки, а не доказывает
обзор всех требований проекта, обработчиков или утверждений документации.

## Пример вызова

```json
{}
```

## Типичная последовательность

1. `memory_recall("Forgeplan")` - восстановить контекст из предыдущих сессий.
2. `forgeplan_health` - получить текущее состояние.
3. Если `verdict != "healthy"`, изучите находки и определите приоритет с владельцем задачи; агрегированный вердикт не разрешает несвязанные мутации.
4. `forgeplan_route` - выполнить роутинг следующей задачи, как только состояние станет чистым.

## Эквивалент CLI

```bash
forgeplan health
```

## См. также

- [`forgeplan_status`](/ru/docs/mcp/forgeplan_status/) - более простая разбивка по типу × статусу.
- [`forgeplan_blindspots`](/ru/docs/mcp/forgeplan_blindspots/) - сфокусированный список слепых пятен.
- [`forgeplan_stale`](/ru/docs/mcp/forgeplan_stale/) - список с истёкшими `valid_until`.
- [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Show project health dashboard — gaps, risks, blind spots, orphans, stale evidence, and recommended next actions. Returns aggregate `verdict` field ("healthy" | "needs_attention" | "unhealthy") + `verdict_summary` for one-line human rendering. Branch on `verdict` for CI gates and agent decision logic. No LLM needed.

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
