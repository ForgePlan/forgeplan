[English](MULTI-AGENT.md) · [Русский](MULTI-AGENT.ru.md)

# Multi-Agent Workflow (v0.24.0+)

С версии **v0.24.0** (PRD-057) Forgeplan умеет **диспетчеризовать работу** между 2–5 суб-агентами в одном workspace. Один MCP-вызов возвращает готовый план: кто над чем работает, что идёт параллельно, что ждёт в serial queue, и почему.

## Зачем это нужно

До v0.24.0 оркестратор (вы или AI-агент) должен был **вручную** держать в голове:
- кто из N агентов чем занят
- какие PRD/RFC трогают общие файлы (иначе merge-hell)
- кто кого блокирует по зависимостям
- кому какие скиллы подходят (backend/frontend/api/…)

2–3 агента ещё управляемо, 5 — уже нет. Типичные провалы:
- **Двойная работа** — два агента взяли один PRD
- **File conflict** — два агента редактируют один crate → race / merge-hell
- **Serial wasted** — A ждёт B, хотя могла бы идти параллельно с C
- **Забытый блокер** — активировали PRD, deps ещё в draft

v0.24.0 закрывает это через четыре MCP-инструмента.

## Четыре MCP-инструмента

### `forgeplan_dispatch` — главный

**Назначение:** превратить список артефактов + live claim-set + граф зависимостей в готовый план на N агентов.

**Контракт:**
```jsonc
{
  "name": "forgeplan_dispatch",
  "arguments": {
    "agents": 3,                                    // 1..=64
    "kind": "prd",                                  // optional: фильтр по kind
    "epic": "EPIC-005",                             // optional: фильтр по parent_epic
    "status": "draft",                              // default "draft", "any" = все
    "agent_skills": [["backend"], ["frontend"], []],// optional: per-agent skills
    "overlap_threshold": 0.3                        // default 0.3 Jaccard
  }
}
```

**Возвращает:**
```jsonc
{
  "buckets": [["PRD-901"], ["PRD-902"], ["PRD-903"]],  // один bucket на агента
  "serial_queue": ["PRD-905"],                           // ждут своей очереди
  "reasoning": [                                          // NFR-005 — почему каждое решение
    "PRD-901: assigned to agent 0 (no file conflict, skill match)",
    "PRD-905: serialized (conflicts with every bucket or no matching skill)"
  ],
  "candidate_count": 4,
  "claimed_count": 0,
  "blocked_count": 0,
  "skipped_parse_errors": 0,
  "agent_count": 3,
  "overlap_threshold": 0.3,
  "generated_at": "2026-04-19T15:44:56.219+00:00"
}
```

**Что алгоритм учитывает** (в порядке применения):

1. **Claims** (`forgeplan_claim`) — уже занятые артефакты исключаются.
2. **Структурные зависимости** — блокированные артефакты (через `graph::topological::kahn_sort`, тот же Kahn sort, что использует `forgeplan_blocked`) исключаются с explanation.
3. **File overlap** — Jaccard similarity по `affected_files`. Пары с overlap ≥ threshold (default 0.3) считаются конфликтующими. Источник `affected_files`: frontmatter-ключ, если есть; иначе секция `## Affected Files` в теле (fallback для legacy-артефактов).
4. **Domain/skill match** — если передан `agent_skills`, артефакт идёт только к агенту с совпадающим skill. Skill mismatch → serial queue.
5. **Least-loaded-first greedy** — распределяет работу равномерно; не валит всё на agent 0.

**Read-only:** не мутирует workspace, не берёт `workspace_lock`. Безопасно поллить с частотой 1 Hz — не блокирует writers.

### `forgeplan_claim` — «я беру этот артефакт»

```jsonc
{
  "name": "forgeplan_claim",
  "arguments": {
    "id": "PRD-901",
    "agent": "worker-1",          // optional: default = MCP clientInfo name/version
    "ttl_minutes": 30,            // default 30, min 1, max 1440 (24h)
    "note": "implementing FR-003" // optional
  }
}
```

Пишет `.forgeplan/claims/PRD-901.yaml` (gitignored). Если уже держит другой агент — **отказ** с `agent_id` и `expires_at` держащего. Same-agent calls **продлевают** TTL. Истёкшие claims **transparent overwrite** (AC-3).

**Атомарная запись** через tempfile + rename — SIGKILL mid-write не оставляет битый YAML.

### `forgeplan_release` — «закончил»

```jsonc
{
  "name": "forgeplan_release",
  "arguments": {
    "id": "PRD-901",
    "agent": "worker-1",  // обязателен, если force=false
    "force": false         // true — orchestrator override для упавшего агента
  }
}
```

Без `force`: только владелец может release'нуть (защита от случайного clobber'а). Missing claim → no-op (идемпотент).

### `forgeplan_claims` — «кто сейчас что делает»

```jsonc
{
  "name": "forgeplan_claims",
  "arguments": { "active": true }
}
```

Возвращает:
```jsonc
{
  "count": 2,
  "skipped": 0,  // malformed YAML файлы — см. server logs
  "claims": [
    { "id": "PRD-901", "agent_id": "worker-1", "expires_at": "...", "note": "..." },
    { "id": "PRD-902", "agent_id": "worker-2", "expires_at": "...", "note": null }
  ]
}
```

Sorted by `expires_at` ASC (earliest-expiring first). **Read-only** — не берёт lock.

## Типичный flow

### Оркестратор распределяет работу между 3 агентами

```
1. orchestrator → forgeplan_dispatch --agents 3 --epic EPIC-005
   ← { buckets: [[PRD-A], [PRD-B], [PRD-C]], serial: [PRD-D], ... }

2. orchestrator → worker-1 "работай над PRD-A"
   orchestrator → worker-2 "работай над PRD-B"
   orchestrator → worker-3 "работай над PRD-C"

3. worker-1 → forgeplan_claim PRD-A --ttl 30
   worker-2 → forgeplan_claim PRD-B --ttl 30
   worker-3 → forgeplan_claim PRD-C --ttl 30

4. Агенты работают параллельно. Каждые ~N минут:
   worker-1 → forgeplan_claim PRD-A (same agent, продлевает TTL)

5. worker-1 done → forgeplan_release PRD-A
   orchestrator → forgeplan_dispatch --agents 3 (re-plan — PRD-D может стать parallelizable)
```

### Агент упал — orchestrator reap'ит claim

```
worker-2 crashed; claim на PRD-B ещё валиден 20 минут.
orchestrator → forgeplan_release PRD-B --force
orchestrator → forgeplan_dispatch --agents 3 (PRD-B снова доступен)
```

Альтернатива: просто подождать TTL expiry — claim сам истечёт через 30 минут.

## Что v0.24.0 НЕ включает

(Deferred to v0.25+ per PRD-057 Growth Vision)

- **CLI parity** — нет CLI-команд `forgeplan dispatch/claim/release/claims`; только MCP. Для использования из CLI — через `forgeplan serve` stdio MCP.
- **Профили `agents/<id>.yaml`** — skills передаются per-call, не персистятся. Roadmapped на v0.27.
- **HTTP/SSE transport** — identity capture работает только на stdio (один клиент на connection). Multi-connection HTTP требует per-request identity extraction.
- **Inter-bucket overlap check** — dispatcher проверяет file-conflicts внутри bucket, но не между агентами. Два overlapping PRD могут попасть в разные buckets (они будут конфликтовать при merge на main — user-level mitigation: не давать их одновременно). Треккается как v0.25 follow-up.
- **Richer claim context** — Claim хранит только `id + agent + ttl + note`. Структурированные поля («какие FR делаю сейчас») — v0.25.

## Поля frontmatter для лучшего dispatch'а

Два поля, которые dispatcher читает (оба **optional**, но сильно улучшают распределение):

```yaml
---
id: PRD-042
title: Auth rewrite
kind: prd
status: draft
depth: standard
# ← новое для dispatch:
affected_files:
  - crates/auth/src/**
  - crates/api/src/auth/
domain: backend   # frontend | backend | api | infra | docs | testing | general
---
```

- **Без `affected_files`:** dispatcher применяет R-2 safety bias и отправляет в serial queue (consider shared-ground).
- **С `affected_files`:** dispatcher считает Jaccard overlap с другими кандидатами и решает, совместимы ли они в параллель.
- **С `domain`:** если orchestrator передал `agent_skills`, артефакт идёт к совпадающему агенту. `domain` валидируется на ASCII `[a-z0-9_-]` — non-ASCII (Cyrillic, RTL, ZWJ) **отклоняется** (security CWE-176).

**Fallback для legacy-артефактов** (без FM-ключа): dispatcher читает секцию `## Affected Files` в теле — обратная совместимость.

## Identity stamping (Inc 2)

**Автоматически** на каждый MCP write-tool, если клиент передал `clientInfo` во время `initialize`:

```yaml
# В .forgeplan/prds/PRD-042-title.md после forgeplan_update:
---
...
last_modified_by: claude-code/1.0
last_modified_at: 2026-04-19T15:44:56+00:00
---
```

Используется для:
- Retro-audit («кто последний трогал это?»)
- Activity log (`.forgeplan/logs/tools-YYYY-MM-DD.jsonl` — каждая MCP-инвокация несёт `client_info`)
- `forgeplan_get` `_next_action` hint (если claim + identity известны, подсказка «held by ...» появляется автоматически)

**Unicode-защита:** control characters, bidi override, ZWJ, path separators **отклоняются** в `AgentIdentity::new` — невидимые символы не могут попасть в markdown через `clientInfo`.

## Ограничения на input (security)

| Param | Cap | Причина |
|---|---|---|
| `agents` | 1..=64 | PRD target 2–5; unbounded → OOM (CWE-770) |
| `agent_skills[i].length` | ≤ 32 | O(N²) Jaccard hot path |
| `affected_files.length` | ≤ 512 | same |
| `affected_files[i].length` | ≤ 512 bytes | same |
| Claim file size | ≤ 64 KB | billion-laughs, R1 parity |
| Claim TTL | 1..=1440 min (24h) | короче — churn, длиннее — stuck-agent risk |

Превышение → error at MCP boundary с понятным сообщением.

## Читать дальше

- **PRD-057** (`.forgeplan/prds/PRD-057-*.md`) — полные FR/AC/NFR + Growth Vision roadmap
- **EVID-077** (`.forgeplan/evidence/EVID-077-*.md`) — что и как было протестировано (R_eff=1.00, CL3)
- **CHANGELOG.md** `[0.24.0]` — полные release notes
- [AGENT-HOOKS.ru.md](AGENT-HOOKS.ru.md) — как Forgeplan hooks (forge-safety, pre-commit-fmt) работают в multi-agent setup
- [UNIFIED-WORKFLOW.ru.md](../methodology/UNIFIED-WORKFLOW.ru.md) — как Forgeplan × Orchestra × Hindsight живут вместе
