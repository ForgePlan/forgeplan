---
depth: tactical
id: PROB-061
kind: problem
status: draft
title: change_log table reset on reindex destroys mutation timeline — git holds truth, log reads stale cache
---

---
id: PROB-061
title: "change_log table reset on reindex destroys mutation timeline — git holds truth, log reads stale cache"
status: Draft
created: 2026-05-06
depth: tactical / standard / deep
context: "{grouping tag}"
parent_epic: EPIC-061
---

# PROB-061: change_log table reset on reindex destroys mutation timeline — git holds truth, log reads stale cache

## Signal

`forgeplan log --json` возвращает синтетические данные вместо реальной истории мутаций. Reproduction:

```
$ forgeplan log --json | jq '.entries | length, [.[].source] | unique, [.[].action] | unique'
39
["reindex"]
["create"]

$ forgeplan log --json | jq '[.entries[].timestamp] | min, max'
"2026-05-06T12:21:48"
"2026-05-06T12:21:49"
```

Все 39 записей — это «create at 12:21:48-49 today, source=reindex». Реальные timestamps создания/активации/линкования артефактов утрачены — `change_log` table в LanceDB **обнуляется при `forgeplan reindex` / `scan-import`** и переписывается синтетическими «create» записями для каждого markdown-файла.

**Аналогия**: `git log` после `git gc` показывает один коммит «Initial commit, today» вместо реальной истории. Файлы целы, timeline уничтожен.

**Где история жива**: `git log .forgeplan/` — каждый создание/активация/линк коммитился отдельным `docs(forgeplan): ...` коммитом. Real timestamps, authors, diffs — всё intact в git, потому что git unaware of LanceDB.

## Constraints

Hard constraints:

- **ADR-003** (Markdown source of truth, LanceDB derived) — change_log как **state** correct rebuilds; как **temporal record** — НЕ rebuildable из markdown alone. Это identity collision двух разных типов данных в одной таблице.
- **Git как append-only event log** — markdown в git **уже** содержит полный mutation history. Не нужно дублировать.
- **Local-first** — решение должно работать offline (но если git local — это всё ещё local-first, в отличие от network-based).
- **Backward compat** — существующие callers `forgeplan_log` / `forgeplan_journal` / `forgeplan_activity` получают тот же JSON shape, только теперь с правильными timestamps.

## Optimization Targets (1-3 макс)

1. **Persistent timeline** — mutation history survives `reindex`, `scan-import`, fresh clone, любые tooling-rebuilds
2. **Single source of truth** — git становится canonical event log; LanceDB — optionally cache, не authority
3. **Запросы по реальным timestamps** работают для всех существующих use cases (`forgeplan log`, `journal`, `activity_stats`, ForgePlanWeb timeline F18)

## Observation Indicators (Anti-Goodhart)

Мониторим, не оптимизируем:

- **Latency `forgeplan log --json`** — соблазн оптимизировать через aggressive cache до момента когда invalidation станет невидимо broken (этот же баг другой формы). Не трогать пока actual user complaints не появятся.
- **Размер git log output** — соблазн trim'ать historical data. Git size — это feature, не bug. Не оптимизировать.

## Acceptance Criteria

1. `forgeplan reindex` запущенный 10 раз подряд **не меняет** timestamps в `forgeplan log --json` (для существующих артефактов)
2. Свежий clone репозитория + `forgeplan init` → `forgeplan log` показывает **те же** timestamps что в parent workspace (потому что git carries state)
3. Создание артефакта через `forgeplan new prd` followed by `forgeplan link/activate/score` → каждая мутация появляется в log с **реальным** timestamp коммита (не reindex-time)
4. ForgePlanWeb timeline endpoint `/api/timeline` работает корректно — slider показывает реальную хронологию артефактов от первого commit до сегодня
5. Performance: `forgeplan log --limit 100` p95 latency ≤ 500ms на workspace с 500 артефактами и 5000 коммитов в `git log .forgeplan/`

## Blast Radius

Затронутые системы:

- **CLI**: `forgeplan log`, `forgeplan journal`, `forgeplan activity`, `forgeplan activity_stats` — все читают сейчас из LanceDB `change_log`
- **MCP**: соответствующие tools (`forgeplan_log`, `forgeplan_activity`, `forgeplan_activity_stats`, `forgeplan_journal`)
- **Storage**: `crates/forgeplan-core/src/db/store.rs` — функции вокруг `change_log` table; reindex logic в `crates/forgeplan-core/src/projection/`
- **ForgePlanWeb**: F18 timeline slider — currently blocked этим багом
- **Health checks**: возможно используют change_log для «recent activity» — нужно audit
- **External**: пользователи которые сделали `forgeplan reindex` уже потеряли свою historical data — нужен migration plan для recovery из git

## Reversibility

**High** — изменение чисто additive:

- Новый код читает из git (новый source)
- Старый LanceDB `change_log` остаётся как fallback для git-less workspaces
- Если новая реализация имеет проблемы — feature flag `log.source: lancedb | git | hybrid` позволит быстрый rollback
- 0 breaking changes в JSON output (контракт сохраняется)

---

## Considered Alternatives (preview, full analysis в RFC)

| Подход | Описание | R_eff |
|---|---|---|
| **A. Защитить change_log от reindex** | Reindex preserves existing rows | low — две источника истины, любой новый rebuild может опять обнулить |
| **B. Derive from git** (recommended) | `forgeplan log` парсит `git log .forgeplan/` напрямую | high — single source of truth, never lost, aligns с ADR-003 |
| **C. Hybrid — git → cache в LanceDB** | LanceDB log = cache of git events. Reindex rebuilds from git instead of nuking | medium — fast queries но complex invalidation |

Полный F-G-R анализ — в будущем RFC после `forgeplan route`.

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| | based_on / informs |

