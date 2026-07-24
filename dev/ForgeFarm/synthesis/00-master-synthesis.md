# ForgeFarm — мастер-синтез: что делать

> Синтез пяти deep-research отчётов (`research/R1…R5`) + WSFold/constellation-анализа
> (сессия 2026-06-29 — 2026-07-02). Документ самодостаточен: можно действовать,
> не открывая исходные отчёты. Детализация по разделам: консенсус →
> [01-consensus.md](01-consensus.md), развилки → [02-open-decisions.md](02-open-decisions.md),
> WSFold-входы → [03-wsfold-bridge.md](03-wsfold-bridge.md).

---

## 1. Что такое ForgeFarm

**ForgeFarm — это отдельный control plane для агентной фабрики разработки, надстроенный НАД ForgePlan и форджем (GitHub/Forgejo), а не вместо них.** Он не хранит истину об артефактах (это ForgePlan, ADR-003: markdown = truth) и не является трекером задач (это фордж). Его собственность — исполнительная истина: projection DB, task state machine, leases и file claims, policy/gate engine, fail-loop, worktree governance, audit log и eval-контур. Формула всех пяти отчётов сходится: побеждает не «система агентов», а **«система контрактов вокруг агентов»** — явные источники истины, fail-closed gates между планированием и кодом, generator≠verifier, evidence-first закрытие задач и наблюдаемость. Ядро — детерминированный Rust control plane, который управляет подключаемыми агентскими runtime'ами за типизированным адаптером: **система управляет фреймворками, а не фреймворки системой**. Человек участвует по policy (review/approve high-risk), а не ведёт пайплайн руками.

---

## 2. Решённые вопросы (единогласный консенсус, ревизии не требуют)

1. **Четыре плана истины, никогда не смешиваются:** артефакты = `.forgeplan/` в git; планирование = issues форджа; исполнение = projection DB ForgeFarm; evidence/наблюдаемость = commits/PRs + event/trace store.
2. **Kanban и labels — всегда ПРОЕКЦИЯ состояния**, никогда не носитель. В трекер зеркалится только человекочитаемый summary + ссылки на PR/evidence.
3. **ForgePlan — artifact kernel, «wrap, don't replace».** Никаких изменений в fpl core не требуется — все пять отчётов потребляют существующие surfaces как есть. Все мутации артефактов — только через CLI/MCP (RED-LINE #11 распространяется через границу продукта: это ADR-001 ForgeFarm).
4. **Готовые gates из ForgePlan переиспользуются:** `validate`/`health`/`drift`/`score`/`activate` — это и есть quality gates оркестратора (readiness до кода, evidence после). Топологическая сортировка typed links = dependency ordering планировщика, `calibrate` depth = вход routing policy, `session` = phase oracle, `export/import` = детерминированный CI seeding.
5. **Фордж = ingress/egress only:** webhook-first с валидацией подписи, scoped tokens; polling — только как reconciliation fallback. Обязательный **reconcile-слой из 6 источников** (issue.state, labels, PR state, commit evidence, ForgePlan artifact state, runtime state) — labels никогда не доверять.
6. **Формальная state machine ранов** (богаче чем backlog/doing/review/fail; ориентир — 16 статусов + 12 событий R3), переходы делает только control plane, каждый переход пишет audit_event.
7. **Lease/claim до начала работы:** task lease (TTL 10–30 мин, heartbeat 30–60 с, expiry policy) + scope lease (path-globs / artifact subtree / map zone). Параллельные агенты — только в изолированных worktrees.
8. **Generator ≠ verifier, evidence-first close:** задача закрывается только с commit hash + PR + tests + linked ForgePlan artifact + машиночитаемым вердиктом верификатора — никогда со слов агента «готово».
9. **Fail-closed gate между планированием (T0/T1) и кодом (T2/T3):** сначала spec + readiness, потом код, потом независимая проверка.
10. **Fail-loop — state machine первого класса** (failure_class, retry_budget, repair_strategy, human_required, quarantine_reason, reentry_condition), с формальным списком причин: CI failed, gate refused, validation blocked, evidence decayed, lease expired, dependency blocked.
11. **Human-on-exception по policy:** категории Auto / Human required / Human optional / «Human запрещено микроменеджить»; ограниченная Human Attention Queue.
12. **`.forgeplan/` co-located с кодом.** Submodule для project-instance артефактов — отвергнут (ломает атомарный PR «code + artifact + evidence»); submodule легитимен только для shared harness/packs.
13. **Память слоистая:** artifact (ForgePlan, авторитетна) > policy > retrieval > hindsight. Одна retrieval-помойка запрещена.
14. **Критичные emitted-артефакты (map.json)** — single writer, atomic tmp-rename, детерминированный guardian, advisory LLM только после deterministic pass, per-run scratch namespaces, gates G1–G4, enforcement на трёх уровнях (hook + runtime policy + CI).
15. **`forgeplan serve` stdio-only ⇒ ForgeFarm — супервизор процессов** (lifecycle, workspace mounts, изоляция), а не HTTP-клиент. PRD-078 workspace param — ровно тот guard, что нужен supervised workers.
16. **Rollout фазированный, contracts-before-intelligence:** сначала фундамент/контракты, потом один orchestration loop, потом swarm. Observability/evals — продуктовая фаза, не «доделаем потом».

---

## 3. Открытые развилки (решение + рекомендованная сторона + сигнал переворота)

Полные позиции сторон и trade-off'ы — в [02-open-decisions.md](02-open-decisions.md).

| # | Развилка | Рекомендация | Что перевернёт решение |
|---|---|---|---|
| 1 | **Kernel: LangGraph.js / Temporal / custom Rust** | **Custom Rust control plane** (позиция R3): leases, claims, gates, audit — то, чего фреймворки не дают; совпадает со стеком fpl и local-first этосом. LangGraph/Deep Agents — только как T0/T1 runtime за ExecutorDriver. Temporal+K8s (R2) — отвергнуть сейчас: team-scale стек для solo builder'а | >30–40% времени control plane уходит в durable-execution plumbing (crash-resume многочасовых ранов, «парковка» approvals на дни, replay) — тогда Temporal как внешняя оболочка |
| 2 | **Инфраструктура MVP** | **Local-first:** docker compose + Postgres + пара Rust-бинарей (api+scheduler), Postgres SKIP LOCKED как очередь. Без Redis/K8s/ArgoCD/NATS. Из R2 взять два дешёвых пункта: hash-chained `audit_events` и `tracing` crate с OTel-совместимыми span'ами (retrofit = замена sink'а) | Второй хост с раннерами, второй регулярный оператор, или первый инцидент «ран не восстановился из Postgres+git» |
| 3 | **L0–L3: семантика и нейминг** | **Переименовать в T0–T3 немедленно** (R3: коллизия с SDD-слайдами) + семантика role contract (R2): tier = разрешённые действия + требуемые evidence; модель = параметр `capability_class`. T1 = spec/design/verification reasoning, T3 = deterministic-tool validate/repair. Разрешить цикл T2↔T3 (не строгая цепочка R5) | Если eval покажет, что дешёвые модели+инструменты системно пропускают verification failures (>10–15% вердиктов T3 опрокидывается на human review) — verification мигрирует в T1 на сильные модели; контракт остаётся, меняется только capability_class binding |
| 4 | **Memory substrate** | **ForgePlan-native:** LanceDB/fastembed для artifact retrieval (уже есть), Hindsight для episodic, обычные Postgres-таблицы (без pgvector) для run-ретроспектив. Mem0/LightRAG/Qdrant/GraphRAG — безусловный reject на MVP | pgvector — когда конкретная recall-задача («найди раны с этим failure_class на похожем коде») структурно не решается structured-запросами. Qdrant — только multi-repo scale с измеренными проблемами |
| 5 | **Транспорт ForgeFarm↔ForgePlan** | **Split по плану:** gates/reads control plane'а — CLI subprocess с `--json` (`health --ci`, `validate --ci`, `drift --ci`, `order`, `score`); агентские workers — spawned stdio `forgeplan serve` в их worktree, владеет тот же runtime adapter, что владеет worktree. Никакого standalone MCP worker pool. Дисциплина R4 с первого дня: literal-body writes (не `@file`) + pinned binary + health smoke per runner | Замерить cold-call CLI (start + LanceDB open): если gate-проверки >1–2 c на переход при swarm-частоте — control plane тоже переходит на resident serve-процесс per workspace |
| 6 | **Run-state vocabulary vs проекция session phases** | **Своя state machine в projection DB** (R3, обрезанная до ~10 статусов на старт); `forgeplan session` — один из ВХОДОВ reconcile, не носитель. Маппинг колонок доски — из R4 (Backlog=нет lease; Ready=не заблокирован графом; Shaping/Coding/Evidence/PR=session.phase; Fail=формальная причина). UI на старте — ровно 2 surface: Board + Run Inspector/HAQ | Если reconcile покажет, что run machine и session phases почти изоморфны на практике — схлопнуть к тонкой проекции. HAQ стабильно >5–10 items — выделять Fail Lab |
| 7 | **Фордж: GitHub vs self-hosted Forgejo** | **Tracker-agnostic за тонким адаптером, MVP на GitHub** (там живёт пользователь). Issues = intent ingress + status mirror с обязательным reconcile; фразу «system of record for tasks» изъять из дизайн-языка. Dual-CI трюк (`.forgejo/workflows` fallback → `.github/workflows`) делает миграцию почти бесплатной | Стоимость GitHub Actions minutes для swarm-CI станет ощутимой, понадобятся network-isolated runners, или приватность task stream |
| 8 | **Очерёдность map-pack** | **Сначала минимальный spine (Alpha), потом map-pack как ПЕРВЫЙ workload на нём — до любых кодящих агентов.** Не flagship-first (R1: примитивы придётся строить ad hoc внутри пайплайна), не Phase 5 (R2: кодящие агенты до проверенных gates инвертируют лестницу рисков). Пять пробелов map-pack чинить при постройке: stable IDs/lineage, serialization determinism spec, машинные G1–G4, per-run scratch, tombstones | Если standalone map generator (dev/forge-understand) понадобится как продукт раньше — шипнуть его как plain CLI pipeline вне ForgeFarm, re-ingest как composition позже |
| 9 | **Eval tuple: где живёт** | **Двухслойно, решается СЕЙЧАС (схема Phase 1):** (1) каждый TaskRun пишет структурированную outcome-строку (model, harness, task_type, cost/tokens, gate results, retries, human interventions, verifier verdict) в projection DB; (2) периодическая дистилляция → ForgePlan EvidencePacks (один EVID на claim «model×task-class», с verdict/CL/valid_until), linked `informs` к routing-policy ADR. Routing ИСПОЛНЯЕТСЯ по быстрым агрегатам DB; routing МЕНЯЕТСЯ только с цитатой на активный EVID. LangSmith как eval-spine — reject (vendor lock против model-agnostic vision). v1 policy = calibrate depth→tier (R4) | Если после ~100–200 ранов дистилляция в EVID — церемония, которую никто не читает, — оставить eval чисто в DB. Если routing-решения начнут оспариваться («почему X на T1?») — это валидация evidence-слоя |

---

## 4. Что взять из WSFold/constellation-анализа (входы в дизайн ForgeFarm)

Развёрнуто — в [03-wsfold-bridge.md](03-wsfold-bridge.md).

1. **Граница абсорбции подтверждена корпусом и не требует ревизии:** fpl владеет только constellation (read-only artifact-graph span) + dispatch hints; всё отвергнутое из fpl (worktree provisioning, sandbox wiring, trust classes, auto-summon, spawning dispatch) имеет именованный дом в ForgeFarm — но **в двух разных plane'ах**: provisioning/policy → детерминированный Rust control plane; agent execution/sandbox → runtime plane за ExecutorDriver. Не сливать обратно.
2. **constellation.yaml = cross-repo половина ingestion-контракта ForgeFarm:** Projection Builder читает `fpl graph|health|order --span`, а не ползает по N репозиториям сам. Ключи артефактов в DB — store-qualified slugs (`web:prd-auth-system`; display numbers коллидируют). Ghost/missing stores — явный resolution state, не ошибка. **Обратное требование к fpl: стабильный `--json` на graph/order/health/dispatch** — у fpl появляется программный потребитель.
3. **Cross-store R_eff федерация остаётся deferred в fpl навсегда:** ForgeFarm читает per-store R_eff и комбинирует на уровне Policy/Gate Engine (это risk-policy решение, не scoring identity). Silent-zero ловушка min() на foreign draft — обойдена по построению.
4. **Dispatch: слои, не замена.** Worktree hints в fpl шипнуть всё равно (standalone-путь без ForgeFarm — first-class). `fpl dispatch --json` / `order --json` = planning primitive, который потребляет Scheduler. Разделение: fpl считает conflict-free partitioning (single-store, advisory); ForgeFarm владеет leases, enforcement, cross-repo concurrency.
5. **WSFold-дисциплина → Worktree Governor, три правила:** (a) идемпотентный provisioning (healthy=no-op, missing=пересоздать детерминированно); (b) **invalid (dirty/diverged worktree) НИКОГДА не авточинится** — никакого `git reset --hard` по policy, только quarantine + HAQ с quarantine_reason; (c) каждая конфиг-поверхность классифицируется: committed intent (playbooks, policies, constellation.yaml) vs local resolution (projection DB, leases, worktree paths) — смешение ловится на review.
6. **Единый verdict enum для всех drift/reconcile контуров:** resolved / missing-recoverable / mismatched-refuse — переиспользуется во всех 5 drift-circuits (worktrees, projections, labels, map.json, constellation stores); auto-repair policy привязана к классу вердикта, а не к каждому контуру отдельно.
7. **gastown/swarm-forge — prior art, которого корпус не знает:** перед постройкой Worktree Governor и headless runtime — structured extraction pass: у gastown взять worktree lifecycle + per-agent storage + mailbox protocol (в корпусе есть RunEvents, но нет inter-agent handoff), у swarm-forge — session supervision + inbox/outbox. Ни один не становится kernel'ом. Это де-рискует дважды задокументированную worktree-боль (PROB-060 shared-HEAD, disk-fill).
8. **Eval-loop ADR — вперёд, в Phase 0–1, не Phase 5:** откладывание воспроизводит слепое пятно корпуса и противоречит raison d'être продукта. Все сокеты уже есть в дизайне: capability_class, risk-policy allowed_tiers, budget envelopes, verifier verdicts, HAQ counts.
9. **EvidencePack/R_eff — decision-слой eval'а, не run store:** verdict↔«model X адекватна task-class Y», congruence_level↔совпадение harness/task-type, decay/valid_until↔устаревание eval'ов при выходе новой версии модели. R_eff тогда делает ПРАВИЛЬНУЮ weakest-link работу: routing-решение на просроченном/опровергнутом eval-evidence видимо падает до 0.1 в blindspots/health.
10. **T0–T3 в коде/API/схемах с первого дня** (rename после schema freeze дорог); fpl остаётся stdio-only — никакого HTTP transport запроса назад в fpl core.

---

## 5. Что делать: фазированный план (solo builder + агенты)

Сверка четырёх MVP-планов корпуса в один. Размеры — относительные (M/L), последовательность — жёсткая: контракты → детерминированный workload → агенты → eval-routing → hardening по сигналам.

### Phase 0 — Артефакты и решения (M, чистый ForgePlan-workflow, кода нет)

Авторинг стартового artifact pack в ForgePlan (в новом репо ForgeFarm, `.forgeplan/` co-located):

- **EPIC** `epic-forgefarm-platform`
- **PRD** `prd-forgefarm-control-plane-mvp` (scope Phase 1–3)
- **ADR-001 source-of-truth model** — четыре плана истины + Artifact Gateway (только fpl CLI/MCP)
- **ADR-002 T0–T3 role contract** — tiers как разрешённые действия + evidence, model = capability_class; risk-policy skeleton
- **ADR-003 eval-as-evidence routing loop** — двухслойная схема (run rows → EVID дистилляция), *вытянут вперёд из Phase 5 сознательно*
- **ADR-004 constellation ingestion + store-qualified identity** (из WSFold-анализа)
- **ADR-005 worktree three-state governance** — never-auto-repair-invalid (из WSFold)
- **RFC** `rfc-runtime-and-lease-model` (task+scope leases, ExecutorDriver seam), **RFC** `rfc-task-state-machine` (~10 статусов + события + инварианты)
- **EVID/Note** prior-art extraction по gastown + swarm-forge → закрывает build-vs-buy как «steal mechanics, reject as kernel»
- Решения фиксируются: Rust kernel, local-first compose, GitHub-first за адаптером, map-pack-after-spine

Параллельно на стороне fpl (отдельный трек, уже одобрен): dispatch worktree hints + disk preflight; аудит `--json` полноты на graph/order/health/dispatch.

### Phase 1 — Spine / Alpha (L): «no magic, all contracts»

- docker compose: Postgres (+ MinIO опционально). Два Rust-бинаря: `ff-api`, `ff-scheduler`.
- Projection DB: урезанная схема R3 — tasks, task_instances, leases, execution_runs, **run outcome rows (eval-кортеж с первого дня!)**, gate_decisions, hash-chained audit_events.
- Task state machine (~10 статусов), переходы только в control plane, каждый — audit_event.
- Lease Manager: task lease (TTL/heartbeat/expiry policy) + scope lease.
- Webhook ingest GitHub (signed) + issue normalizer + **reconcile-слой (6 источников)**.
- Gate engine stub: вызовы `forgeplan health --ci / validate --ci / drift --ci` как CLI subprocess с JSON.
- `tracing` со структурированными span'ами (OTel-совместимо, sink — локальный файл).
- **DoD (Alpha):** issue→task проекция; lease acquire/release; статус зеркалится в GitHub; audit events читаемы; ни одной строки кода не написано агентом.

### Phase 2 — Map-pack как первый workload (M/L)

- Детерминированный пайплайн на spine: scanners (per-run scratch `.work/runs/<runId>/scan/*.json`) → zone-extractor → edge-verifier → **map-emitter (единственный writer, exclusive scope lease на map.json)** → deterministic guardian → advisory LLM.
- Чинятся 5 пробелов R1: stable_node_key + lineage/supersedes; serialization determinism spec (порядок ключей/массивов, UTF-8 LF, path normalization); машинные G1–G4; per-run namespaces; tombstones (`status: active|superseded|deleted`).
- Enforcement fail-closed на трёх уровнях: hook + runtime policy + CI.
- **DoD (Gamma-критерий R3):** rerun после +1 node даёт byte-identical позиции нетронутых нод; append refresh работает по zone scope. Spine доказан на детерминированной нагрузке.
- Бонус: продуктово полезный map.json стыкуется с уже идущим project-map generator (dev/forge-understand).

### Phase 3 — Один orchestration loop с агентами (L)

- Worktree Governor (механики из gastown/swarm-forge extraction; three-state + never-auto-repair).
- ExecutorDriver seam + первый adapter (Claude Code / OpenCode headless); spawned `forgeplan serve` per worktree (PRD-078 workspace param).
- **Только T0 (planning) + T2 (implementation)** — совет R4; T3-верификатор как отдельный ран (generator≠verifier), консервативные gates, максимум human checkpoints.
- ForgePlan lifecycle hooks вокруг T2: readiness до кода; evidence + validate/drift после; evidence-first close.
- Fail-loop с формальными причинами + Human Attention Queue (пока просто таблица+CLI/минимальный view).
- Eval-строки пишутся на каждый ран (схема уже есть из Phase 1).
- Write-дисциплина R4: literal-body в MCP, pinned binary + health smoke.
- **DoD:** low-risk issue проходит happy path end-to-end (T2 → worktree → code+tests → drift clean → EVID linked → PR → board); policy escalation на high-risk останавливается в HAQ; fail loop делает N retry → failed_human.

### Phase 4 — Eval distillation + routing + минимальный UI (M/L)

- Дистилляция run rows → ForgePlan EvidencePacks per (model × task-class): gate-pass rate, cost/task, interventions/task, n. Structured Fields обязательны (verdict/CL/evidence_type/valid_until).
- Runtime Broker: model-routing.yaml меняется только со ссылкой на активный EVID; v1 policy = calibrate depth→tier.
- Добавление T1/T3 lanes полноценно (spec/verification + validate/repair, цикл T2↔T3).
- UI: 2 surfaces — Board (проекция) + Run Inspector/HAQ. Не больше.
- Retrospective writeback в episodic memory (Hindsight) по закрытию ранов.

### Phase 5 — Hardening и масштаб (по сигналам, не по расписанию)

- Security: branch protection suite, runner isolation (RCE boundary), scoped bot tokens; sandbox для untrusted кода.
- Observability upgrade: OTel sink → реальный collector/дашборды — когда появится второй хост/оператор.
- NATS / Temporal / Qdrant / Forgejo self-hosted / Fail Lab / Governance Console — **только при срабатывании flip-сигналов из §3**.
- Composition family (spec-build, review-pack, evidence-pack, release-pack) — по шаблону map-pack.

---

## 6. Чего НЕ делать (анти-паттерны, единогласно или большинством)

1. **Labels/issues/board как runtime database** — самое повторяемое предупреждение корпуса: гарантированные race conditions и drift четырёх «истин».
2. **Превращать ForgePlan в оркестратор** или строить второй artifact store внутри ForgeFarm.
3. **Писать напрямую в LanceDB или raw `.forgeplan/` markdown** мимо CLI/MCP («ForgeFarm must not get clever»).
4. **`.forgeplan/` как submodule** для ежедневного flow (ломает атомарный PR; export/import уже закрывает CI-воспроизводимость).
5. **Параллельные агенты без leases/claims в общем workspace** — git worktree сам по себе не решает конфликтные записи (урок PROB-060).
6. **Закрывать задачи со слов агента; мержить без машиночитаемого вердикта верификатора.**
7. **Генерация кода до стабилизации артефактов** (нет fail-closed gate планирование→код).
8. **Одна retrieval-помойка вместо слоистой памяти** — hindsight начинает перезаписывать официальные артефакты.
9. **Несколько writers у критичных emitted-артефактов** — включая «оркестратор тоже чуть подправит map.json»; single-writer правило старше всех остальных write-прав.
10. **Ставка core на один «магический» фреймворк** (LangChain-only — explicit «do not choose»; VoltAgent — preview-grade workflows; Deep Agents — «nested primitive, not the brain»).
11. **Untrusted код с write-токенами/секретами** — misuse `pull_request_target`, host-mode/privileged runners.
12. **Tiers = мощность модели** и сохранение двусмысленного L0–L3 нейминга во внутренних enum'ах.
13. **Автоматизация на Forgejo Projects board API как primary state** — under-documented, version-tied; доска вычисляется, а не опрашивается.
14. **Запуск полного swarm сразу; evals/observability «потом»** — получится «красивая, но ненадёжная оболочка».
15. **Hand-rolling durable-workflow забот на голых очередях/cron'ах** — retry/pause/approval это norm case; им место в формальной state machine (или Temporal при flip-сигнале), а не «нигде».
16. **Enterprise-стек авансом** (K8s, ArgoCD, Temporal, Redis, Qdrant, Mem0, LightRAG, GraphRAG, 7 UI-surfaces, multi-tenant auth) — каждый элемент помечен в корпусе как conditional, и ни одно условие сейчас не выполнено.

---

**Следующий физический шаг:** Phase 0 — создать репо ForgeFarm (`~/Work/ForgePlanFarm` по PascalCase-конвенции), `forgeplan init -y`, и авторить artifact pack начиная с `EPIC-forgefarm-platform` и `ADR-001 source-of-truth`. Параллельный fpl-трек (dispatch hints + `--json` аудит) можно вести в основном репо ForgePlan независимо.
