---
title: forgeplan_dispatch
description: "Рекомендательные очереди агентов и последовательная очередь; оркестратор проверяет файловые конфликты между очередями."
---

Рассчитывает по одной очереди на агента и последовательную очередь по
объявленным `affected_files`, действующим захватам, структурным зависимостям
и необязательным навыкам домена. Текущий алгоритм сравнивает пересечение
только с артефактами уже выбранной очереди, а не с очередями других агентов.
Поэтому даже одинаковые объявленные наборы файлов могут попасть разным
одновременно работающим агентам. Внутри рассматриваемой очереди пересечение
Жаккара **на пороге или выше** (по умолчанию 0.3) запрещает такое размещение,
но не гарантирует безопасность работы между очередями.

Этот план без записи состояния носит рекомендательный характер. До передачи
работы оркестратор обязан сравнить **все одновременно назначаемые наборы файлов**
и выполнить пересекающуюся работу последовательно либо переназначить её,
включая явно объявленные пересечения. Захваты артефактов — рекомендательные
маркеры, не файловые блокировки: разные захваты не делают наборы файлов
непересекающимися. Захваченное описание интерфейса ниже называет план
«parallel-safe», но текущая реализация такой гарантии не даёт.

**Категория**: Multi-agent

## Когда агент вызывает

- Старт multi-agent спринта: оркестратор хочет посадить 2–5 суб-агентов на
  непересекающуюся работу.
- После [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) - пересчитать план,
  потому что набор кандидатов изменился.
- После того как [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) создал свежие
  черновики - учесть их.
- Истечение TTL на застрявшем клейме - перепланировать, чтобы заполнить
  освободившийся слот.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `agents` | `integer` | yes | Число субагентов, 1..=64. MCP handler отклоняет значения вне диапазона. |
| `kind` | `string` | no | Фильтр по одному типу артефакта (`prd`, `rfc`, `spec` и т. д.). По умолчанию: все типы. |
| `epic` | `string` | no | Только кандидаты, у которых `parent_epic` во frontmatter совпадает с этим Epic ID. |
| `status` | `string` | no (default `"draft"`) | Фильтр по статусу. `"any"` - включить все состояния жизненного цикла. |
| `agent_skills` | `string[][]` | no | Списки навыков в порядке агентов; не больше одного списка на запрошенного агента и 32 навыков в списке. Пропуск даёт `[]`, не null. |
| `overlap_threshold` | `number \| null` | no (default 0.3) | Включительный порог Жаккара. MCP handler отклоняет значения вне `[0.0, 1.0]`, несмотря на формулировку clamp в захваченной схеме. |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/types.rs::DispatchParams`_

## Возвращает
Ниже иллюстративный payload, не результат зафиксированного запуска.


```json
{
  "buckets": [
    ["PRD-057"],
    ["RFC-012"]
  ],
  "serial_queue": [{"id": "SPEC-018", "reason": "file overlap >=0.30 with bucket 0"}],
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

Передавайте `buckets[i]` субагенту `i` только после сравнения всех одновременно
назначаемых наборов файлов между очередями и последовательного выполнения либо
переназначения пересекающейся работы. После изменения набора захватов или
кандидатов перепланируйте работу и повторите сравнение. `skipped_parse_errors > 0`
означает, что у хотя бы одного кандидата не удалось прочитать frontmatter —
проверьте логи сервера.

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

1. `forgeplan_dispatch agents=N` — оркестратор получает рекомендательный план.
2. Оркестратор сравнивает все одновременно назначаемые наборы файлов между очередями, включая объявленные пересечения, и до передачи работы назначает последовательное выполнение либо переназначает конфликтующую работу.
3. Для каждого получившегося непустого назначения субагент `i` вызывает [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) для следующего артефакта; пустые очереди пропускаются. Эти маркеры артефактов не блокируют файлы.
4. Субагенты работают только в пределах проверенных назначений; оркестратор следит за [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/).
5. [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) по завершении → перепланирование и повторное сравнение наборов между очередями.

## CLI эквивалент

[`forgeplan dispatch`](/ru/docs/cli/) - тот же движок. Оркестраторы на shell используют
CLI; LLM-driven оркестраторы - MCP-инструмент.

## См. также

- [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) - суб-агент берёт элемент из бакета
- [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) - вернуть слот в пул
- [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/) - мониторинг работы в полёте

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Compute a parallel-safe work plan for N sub-agents. Returns buckets (one per agent), a serial queue for leftover work, and human-readable reasoning for every placement decision. Skips artifacts already claimed, defers artifacts with file-overlap >= threshold (default Jaccard 0.3), and when agent_skills is provided routes by domain match. Read-only — does not mutate workspace state.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `agent_skills` | нет | array | Per-agent skill lists in index order. Empty entries default to<br>"any skill"; omit to disable skill matching entirely. |
| `agents` | да | integer | Number of sub-agents the orchestrator can hand work to. Required;<br>clamped to `&gt;= 1` downstream. |
| `epic` | нет | string \| null | Optional filter: only artifacts with this parent Epic ID. Matches<br>the `parent_epic` frontmatter field. |
| `kind` | нет | string \| null | Optional filter: only consider artifacts of this kind<br>(`prd`/`rfc`/`spec`/etc.). When omitted, all kinds are considered. |
| `overlap_threshold` | нет | number \| null | Jaccard threshold above which two artifacts are considered<br>file-conflicting. Default 0.3. Clamp 0.0..=1.0. |
| `status` | нет | string \| null | Optional filter: consider only artifacts in this status (default<br>`draft`). Set to `"any"` to include every lifecycle state. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "agent_skills": {
      "default": [],
      "description": "Per-agent skill lists in index order. Empty entries default to\n\"any skill\"; omit to disable skill matching entirely.",
      "items": {
        "items": {
          "type": "string"
        },
        "type": "array"
      },
      "type": "array"
    },
    "agents": {
      "description": "Number of sub-agents the orchestrator can hand work to. Required;\nclamped to `>= 1` downstream.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "epic": {
      "description": "Optional filter: only artifacts with this parent Epic ID. Matches\nthe `parent_epic` frontmatter field.",
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "description": "Optional filter: only consider artifacts of this kind\n(`prd`/`rfc`/`spec`/etc.). When omitted, all kinds are considered.",
      "type": [
        "string",
        "null"
      ]
    },
    "overlap_threshold": {
      "description": "Jaccard threshold above which two artifacts are considered\nfile-conflicting. Default 0.3. Clamp 0.0..=1.0.",
      "format": "double",
      "type": [
        "number",
        "null"
      ]
    },
    "status": {
      "description": "Optional filter: consider only artifacts in this status (default\n`draft`). Set to `\"any\"` to include every lifecycle state.",
      "type": [
        "string",
        "null"
      ]
    },
    "workspace": {
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "agents"
  ],
  "title": "DispatchParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
