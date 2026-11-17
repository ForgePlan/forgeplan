[English](MULTI-AGENT.md) · [Русский](MULTI-AGENT.ru.md)

# Совместная работа агентов (Forgeplan 0.38.0)

Forgeplan рассчитывает план работы и хранит рекомендательные захваты артефактов (claims). Оркестратор запускает и контролирует исполнителей средствами своего агентного окружения; `dispatch` их не запускает. Первые инструменты координации появились в v0.24.0; примеры CLI ниже соответствуют релизу 0.38.0.

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

Четыре операции координации помогают не назначить одну работу дважды. Они не обеспечивают владение исходными файлами и не заменяют трекер задач.

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
  "serial_queue": [{"id": "PRD-905", "reason": "missing affected_files frontmatter"}],
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
2. **Структурные зависимости** — артефакты с неразрешёнными зависимостями не попадают в buckets и появляются в `serial_queue` с причинами блокировки.
3. **Пересечение файлов** — Jaccard similarity по объявленным строкам путей. Пары с overlap ≥ threshold (по умолчанию 0.3) нельзя поместить в один bucket. Пути и glob-шаблоны не раскрываются по файловой системе; разные строки могут обозначать пересекающиеся файлы. Источник: frontmatter `affected_files`, иначе секция `## Affected Files` в теле.
4. **Domain/skill match** — если передан `agent_skills`, артефакт идёт только к агенту с совпадающим skill. Skill mismatch → serial queue.
5. **Least-loaded-first greedy** — распределяет работу равномерно; не валит всё на agent 0.

**Только чтение:** не берёт `workspace_lock` и не резервирует назначения. Перед передачей работы перечитайте claims. **Ограничение:** пересечения проверяются внутри bucket, но не между buckets. До запуска оркестратор должен сверить реальные наборы файлов всех параллельных исполнителей.

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

**Claim — мягкий сигнал, не блокировка записи.** Другие инструменты артефактов не запрещают запись из-за чужого claim. Отдельный workspace lock сериализует участвующие в протоколе мутации CLI/MCP; ни он, ни claim не блокируют исходные файлы, git-ветки или удалённый клон. Claims локальны для выбранного `.forgeplan/` и не распространяются автоматически между worktrees.

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

Запускайте CLI в нужном workspace. Синтаксис соответствует 0.38.0; ID приведены для примера:

```bash
forgeplan dispatch --agents 3 --epic EPIC-005 --json
forgeplan claims --json
# Сверить пересечения между buckets и назначить каждому точные файлы.
forgeplan claim PRD-901 --agent worker-1 --ttl-minutes 30
# Оркестратор запускает worker-1 средствами своего окружения.
# До истечения TTL продлить той же командой с тем же именем.
forgeplan claim PRD-901 --agent worker-1 --ttl-minutes 30
# После завершения работы:
forgeplan release PRD-901 --agent worker-1
forgeplan dispatch --agents 3 --json
```

Используйте **одинаковый явный `--agent`** при захвате, продлении и освобождении. В реализации 0.38.0 значения по умолчанию различаются: `claim` использует `cli-0.38.0`, `release` — `cli/0.38.0` (help этого расхождения не показывает). Всегда передавайте простое имя вроде `worker-1`: явная CLI-валидация claim отвергает `/` и `\`, хотя MCP `clientInfo` может иметь вид `name/version`. `--agent` обозначает владельца claim, а не выбирает и не запускает модель.

### Агент упал — оркестратор снимает claim

Убедившись, что владелец больше не работает:

```bash
forgeplan release PRD-902 --force
forgeplan dispatch --agents 3 --json
```

Либо дождитесь указанного `expires_at`. Не исправляйте несовпадение собственного имени при release через `--force`.

## Границы поддержки в 0.38.0

- **CLI:** доступны `dispatch`, `claim`, `release`, `claims`. Параметр `agent_skills` остаётся только в MCP.
- **Нет постоянных профилей:** skills передаются в каждом вызове, а не читаются из `agents/<id>.yaml`.
- **Stdio MCP:** `forgeplan serve` получает `clientInfo` соединения; claims не являются аутентифицированными правами доступа.
- **Пересечения между buckets:** описанное выше ограничение сохраняется. План не доказывает отсутствие конфликтов параллельных правок.
- **Разделение ролей:** Forgeplan хранит решения, доказательства и зависимости артефактов. Настроенный трекер хранит задачи, исполнителей и сроки. Взятие задачи в трекере — не `forgeplan claim`; закрытие задачи не активирует артефакт.

MCP-вызовы координации принимают `workspace`, если worktree отличается от каталога запуска сервера. Передавайте абсолютный корень нужного worktree последовательно; смена `cwd` CLI не перенаправляет уже запущенный MCP-сервер.

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
  - crates/auth/src/session.rs
  - crates/api/src/auth.rs
domain: backend   # frontend | backend | api | infra | docs | testing | general
---
```

- **Без `affected_files`:** dispatcher применяет R-2 safety bias и отправляет в serial queue (consider shared-ground).
- **С `affected_files`:** dispatcher считает Jaccard overlap с другими кандидатами и решает, совместимы ли они в параллель.
- **С `domain`:** используются ASCII-имена доменов и сравнение skills без учёта регистра. Некорректные символы приводят к отсутствию домена, а не к отказу записи артефакта. Пустой список skills агента принимает любой домен.

**Fallback для legacy-артефактов** (без FM-ключа): dispatcher читает секцию `## Affected Files` в теле — обратная совместимость.

## Identity stamping (Inc 2)

**Best-effort** запись при MCP `new` и `update`, если сервер получил корректный `clientInfo`:

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

Поведение зависит от параметра: неверные число агентов и threshold отвергаются на границе вызова, а список файлов обрезается до 512 записей, слишком длинные пути отбрасываются. Это не проверка владения файлами.

## Читать дальше

- **PRD-057** (`.forgeplan/prds/PRD-057-*.md`) — исходные требования и Growth Vision, не подтверждение реализации всей дорожной карты
- **EVID-077** (`.forgeplan/evidence/EVID-077-*.md`) — историческая проверка v0.24.0 (R_eff=1.00, CL3), не новый результат тестирования 0.38.0
- **CHANGELOG.md** `[0.24.0]` — исторические release notes
- [AGENT-HOOKS.ru.md](AGENT-HOOKS.ru.md) — публичная регистрация хуков, рекомендательные проверки и границы хостов
- [UNIFIED-WORKFLOW.ru.md](../methodology/UNIFIED-WORKFLOW.ru.md) — как Forgeplan × Orchestra × Hindsight живут вместе

## Источники релиза

- [Claims в core 0.38.0](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/claim/mod.rs#L1-L10) — намерение работать и отдельная блокировка workspace.
- [Диспетчер и проверка пересечений](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/dispatch/mod.rs#L383-L508), [CLI](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/dispatch.rs#L114-L185).
- [Публичная роль task-runner](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-orchestra/agents/task-runner.md#L184-L218) — трекер отдельно от исходников и мутаций Forgeplan.
- [CLI claim default](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/claim.rs#L17-L28) и [release default](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/release.rs#L13-L15) — почему нужно явно передавать одно имя.
