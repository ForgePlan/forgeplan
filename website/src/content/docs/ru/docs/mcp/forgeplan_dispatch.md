---
title: forgeplan_dispatch
description: "Compute a parallel-safe work plan for N sub-agents — buckets, serial queue, reasoning."
---

Точка входа оркестратора для multi-agent работы. Возвращает по одному бакету на
агента с артефактами, которые можно вести параллельно без файловых конфликтов, плюс
серийную очередь для оставшейся работы. Пропускает уже заклеймленные артефакты
(живой клейм другим агентом), откладывает артефакты, у которых пересечение Жаккара
по `affected_files` превышает порог (по умолчанию 0.3), уважает структурный граф
зависимостей (заблокированные артефакты никогда не попадают в бакет), а при заданном
`agent_skills` — маршрутизирует по совпадению домена. Read-only — состояние
рабочего пространства не мутирует.

**Категория**: Multi-agent

## Когда агент вызывает

- Старт multi-agent спринта: оркестратор хочет посадить 2–5 суб-агентов на
  непересекающуюся работу.
- После [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) — пересчитать план,
  потому что набор кандидатов изменился.
- После того как [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) создал свежие
  черновики — учесть их.
- Истечение TTL на застрявшем клейме — перепланировать, чтобы заполнить
  освободившийся слот.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `agents` | `number` | yes | Число суб-агентов (1..=`MAX_AGENTS`). PRD-057 целится в 2–5. |
| `kind` | `string` | no | Фильтр по одному типу артефакта (`prd`, `rfc`, `spec` и т. д.). По умолчанию: все типы. |
| `epic` | `string` | no | Только кандидаты, у которых `parent_epic` во frontmatter совпадает с этим Epic ID. |
| `status` | `string` | no (default `"draft"`) | Фильтр по статусу. `"any"` — включить все состояния жизненного цикла. |
| `agent_skills` | `string[][]` | no | Списки навыков по агентам в порядке индексов (максимум `MAX_SKILLS_PER_AGENT` на агента). |
| `overlap_threshold` | `number` | no (default 0.3) | Порог Жаккара для откладывания файлового конфликта. Диапазон `[0.0, 1.0]`. |

_Источник схемы: `crates/forgeplan-mcp/src/types.rs::DispatchParams`_

## Возвращает

```json
{
  "buckets": [
    ["PRD-057"],
    ["RFC-012"]
  ],
  "serial_queue": ["SPEC-018"],
  "reasoning": [
    "PRD-057 → bucket 0 (no skill match, no claim, no overlap)",
    "RFC-012 → bucket 1 (skill match: 'rust')",
    "SPEC-018: deferred (file overlap with PRD-057 @ Jaccard 0.42)"
  ],
  "generated_at": "2026-04-26T10:00:00Z",
  "agent_count": 2,
  "overlap_threshold": 0.3,
  "candidate_count": 3,
  "claimed_count": 0,
  "skipped_parse_errors": 0,
  "blocked_count": 0,
  "_next_action": "Plan ready: 3 candidate(s), 2 parallel bucket(s), 1 serial ..."
}
```

Передайте `buckets[i]` суб-агенту `i`. Перепланируйте, когда меняется набор клеймов
или кандидатов. `skipped_parse_errors > 0` означает, что у хотя бы одного кандидата
не удалось прочитать frontmatter — проверьте логи сервера.

## Пример вызова

По умолчанию: 3 агента, только draft PRD:

```json
{ "agents": 3, "kind": "prd" }
```

Skill-aware диспатч:

```json
{
  "agents": 2,
  "agent_skills": [["rust", "mcp"], ["docs", "ru"]],
  "overlap_threshold": 0.25
}
```

Перепланирование целого Epic:

```json
{ "agents": 4, "epic": "EPIC-005", "status": "any" }
```

## Типичная последовательность

1. `forgeplan_dispatch agents=N` — оркестратор получает план.
2. Каждый суб-агент `i` вызывает [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) на `buckets[i][0]`.
3. Суб-агенты работают; оркестратор опрашивает [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/).
4. [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) по завершении → пере-диспатч.

## CLI эквивалент

[`forgeplan dispatch`](/ru/docs/cli/) — тот же движок. Оркестраторы на shell используют
CLI; LLM-driven оркестраторы — MCP-инструмент.

## См. также

- [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) — суб-агент берёт элемент из бакета
- [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) — вернуть слот в пул
- [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/) — мониторинг работы в полёте
