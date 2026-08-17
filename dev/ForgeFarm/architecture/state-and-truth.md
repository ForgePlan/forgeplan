# Состояние и истина: 4 плана, state machine, leases, reconcile, fail-loop

> Операционная модель ForgeFarm: где живёт какая истина, как формализованы
> состояния, кто и как их меняет. Источники: R1 (словарь статусов),
> R3 (state machine + leases + audit), R4 (маппинг доски + fail-причины),
> R5 (6-source reconcile + схемы записей), WSFold-мост (three-state discipline).

## 1. Четыре плана истины (никогда не смешивать)

| Истина | Носитель | Владелец | Rebuildable? |
|---|---|---|---|
| **Артефактная** (решения, требования, evidence) | `.forgeplan/` markdown в git | ForgePlan | нет — это и есть truth |
| **Плановая** (work intents) | issues форджа → с момента ingestion: projection DB | фордж (ingress) → ForgeFarm | — |
| **Исполнительная** (runs, leases, retries, verdicts, approvals) | Projection DB ForgeFarm | ForgeFarm control plane | да, из git + tracker + ForgePlan |
| **Evidence/наблюдаемость** | commits/PRs + event/trace store + audit log | git + ForgeFarm audit | append-only |

Правило зеркалирования: в трекер возвращаются только человекочитаемый summary-статус, ссылки на PR и evidence. Labels вида `state:*`, `risk:*`, `lease:held` — исходящие проекции; переход НИКОГДА не инициируется из трекера.

## 2. Task/Run state machine

Принято (развилка №6): собственная машина в projection DB, стартовать с **~10 статусов**, расширять когда появятся code paths. Полный референс — 16 статусов + 12 событий R3; минимальный словарь R1:

```
ready → claimed → planned → executing → awaiting_verifier → done
                     ↓            ↓             ↓
        blocked_by_dependency  merge_conflict  awaiting_human
                                  ↓
                          policy_failed → retry_scheduled → (re-enter scheduler)
```

**Шесть жёстких инвариантов (R3):**
1. Переходы делает ТОЛЬКО control plane (ни агент, ни трекер, ни UI напрямую).
2. Каждый переход пишет `audit_event` (append-only, hash-chained: `hash_prev`/`hash_self`).
3. У задачи максимум один активный task lease.
4. Записи в репо — только через approved run с активным scope lease.
5. High-risk переходы идут через policy path (gate + при необходимости human).
6. Проекции (доска, labels) вычисляются из машины, не наоборот.

`forgeplan session` (idle → routing → shaping → coding → evidence → pr) — **один из входов reconcile**, phase oracle для методологии; не носитель run-состояния.

## 3. Lease-модель (два контура)

| Контур | Гранулярность | Параметры | Назначение |
|---|---|---|---|
| **Task lease** | задача/ран | TTL 10–30 мин; heartbeat 30–60 с; expiry policy: `requeue \| fail_human \| kill_runtime` | «кто сейчас владеет задачей»; защита от зомби-агентов |
| **Scope lease** | path-globs / bounded context / artifact subtree / map zone (ключ `repo_id + scope_hash`) | exclusive/shared | «какие файлы/зоны можно писать»; сериализация конфликтов |

Референс lease-записи (R5, 7 полей): `task_run_id, agent_level(tier), lease_owner, expires_at, workflow_label, claimed_paths, worktree_path`. Конфликтующие claims: сериализовать или увести в speculative branch lane (R2) с обязательным поздним rebase+verify. Особый случай: exclusive writer lease на `map.json` — только у map-emitter.

**Помнить:** git worktree сам по себе НЕ решает конфликтные записи (урок PROB-060) — изоляция workspace и claim-резервация это два разных механизма, нужны оба.

## 4. Reconcile (обязательный слой)

Шесть источников сверки (R5): `issue.state` · `issue.labels` · linked PR state · commit evidence · ForgePlan artifact state · ForgeFarm runtime state. Labels никогда не доверять. Webhook-first ingest (с подписью), polling — только reconciliation fallback.

**Единый verdict enum для всех drift/reconcile-контуров** (WSFold-мост): 

| Вердикт | Значение | Политика |
|---|---|---|
| `resolved` | intent и реальность совпадают | no-op |
| `missing-recoverable` | реализация отсутствует, но детерминированно восстановима | auto-repair разрешён |
| `mismatched-refuse` | реальность противоречит intent (dirty worktree, чужие файлы, битые метаданные) | **НИКОГДА не авточинить** → quarantine + HAQ с `quarantine_reason` |

Пять drift-контуров (R3), все на одном enum: code↔artifact · artifact completeness · issue↔projection · execution · map.json. Auto-repair policy привязана к классу вердикта, не к контуру.

## 5. Fail-loop (state machine первого класса)

Запись фейла (R2/R5): `failure_class, retry_count, retry_budget, repair_strategy, owner_level(tier), human_required, quarantine_reason, reentry_condition`.

Формальные причины входа (R4): CI failed · gate refused transition · validate/review blocking issues · evidence decay lowered trust · task lease expired · blocked by graph dependency.

Маршрутизация: T3 FAIL → классификация → repair ticket в T1 (архитектурная причина) или локальный фикс в T2 (цикл T2↔T3); исчерпан retry_budget → `failed_human` → Human Attention Queue. Fail-loop — не колонка доски: колонка «Fail» лишь проецирует формальную причину.

## 6. Evidence-first close (чеклист закрытия)

Задача закрыта только при наличии ВСЕХ шести (R5): commit hash · PR link · test results · linked ForgePlan artifact · машиночитаемый verdict верификатора · (для активации) evidence score > 0. Слова агента «готово» не являются событием закрытия.

## 7. Маппинг kanban-доски (проекционная функция, из R4)

| Колонка | Вычисляется из |
|---|---|
| Backlog | задача без lease/claim |
| Ready | открытый issue, не заблокированный графом зависимостей |
| Shaping / Coding / Evidence / Review-PR | `session.phase` соответствующего workspace |
| Fail | формальная причина из fail-loop (не «красный стикер») |
| Human | запись в HAQ |

## 8. Committed intent vs local resolution (WSFold-дисциплина)

Каждая конфиг-поверхность ForgeFarm обязана быть отнесена к одному из двух классов; смешение — design smell:

| Committed intent (в git) | Local resolution (машинно-локально) |
|---|---|
| playbooks, policies (risk-policy.yaml, write-allowlist) | Projection DB |
| compositions, constellation.yaml | constellation.local.yaml |
| схемы (map.schema.json), prompts per tier | leases, worktree paths, heartbeats |
| `.forgeplan/` артефакты | LanceDB index, session.yaml |
