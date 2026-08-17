# Безотказность и путь в K8s

> Ответ на два вопроса владельца: (1) «статы — forgeplan — история — mem0 —
> hindsight — langgraph — rs graph: что ещё должно быть, чтобы всё работало
> безотказно»; (2) «как это будет работать в K8s на будущее, с оператором».
> Принцип: **K8s-ready by design, K8s-deployed по flip-сигналу** — сейчас
> local-first compose (развилка №2), но каждое дизайн-решение Phase 1
> выбирается так, чтобы миграция была деплой-операцией, а не рефакторингом.

## 1. Слоёный data-стек: что где живёт (по пунктам владельца)

| Слой | Решение | Статус |
|---|---|---|
| **Статы** | два уровня: (а) runtime-метрики — считаются из projection DB + `audit_events` (timestamps есть у каждого перехода; throughput/lead-time выводятся запросом); (б) `forgeplan activity-stats` — телеметрия артефактных операций (call counts, error counts, p50/p95) — готовый вход, не дублировать | решено |
| **ForgePlan** | artifact kernel, единственная истина по артефактам; мутации только CLI/MCP; LanceDB — derived, восстанавливается `scan-import` | решено (ADR-001) |
| **История** | три несмешиваемых вида: (а) git history — история кода и артефактов; (б) **hash-chained `audit_events`** (`hash_prev`/`hash_self`, append-only) — история переходов и privileged-действий, тампер-видимая; (в) `run_events` — потоковая история ранов (RunEvents). «Историю» никогда не выводить из labels/доски | решено |
| **Mem0** | **reject на MVP** (развилка №4) — добавляет сервис и вторую семантическую истину; flip-сигнал: память как сетевой сервис для не-ForgeFarm клиентов (маловероятно) | решено |
| **Hindsight** | episodic-слой (ретроспективы ранов, уроки) — уже существует у пользователя per-project banks; retrospective writeback при закрытии ранов (Phase 4). Приоритет памяти: **artifacts > policy > retrieval > hindsight** — hindsight никогда не перекрывает артефакты | решено |
| **LangGraph** | **не хранилище и не ядро** — опциональный T0/T1 runtime за ExecutorDriver; его checkpoints — внутреннее дело адаптера, канонический статус рана всё равно в projection DB | решено (развилка №1) |
| **Rust graph** | два графа, оба в control plane: (а) **артефактный DAG** — из `fpl graph/order --json` (typed links, topological sort — НЕ переизобретать, R4); (б) **runtime DAG задач** (blockedBy, lease-конфликты) — таблицы Postgres + in-memory граф в `ff-scheduler` (petgraph-класс структура), пересобираемый из DB на старте. In-memory граф — кэш, не истина | решено |

**Чего не хватало в списке владельца (добавляю):**

| Недостающий слой | Зачем |
|---|---|
| **Projection DB (Postgres)** | операционная истина: tasks, leases, runs, gate_decisions, eval-строки. Единственный stateful-сервис MVP |
| **Object store (логи/большие evidence)** | сырые логи ранов, стенограммы — не в Postgres и не в git; MVP: каталог на диске / MinIO опционально; K8s: S3-совместимый |
| **Retrieval index** | LanceDB/fastembed per store (уже в ForgePlan); федерация — deferred |
| **Секреты** | MVP: env/файл вне git; K8s: Secrets/External Secrets. API-ключи провайдеров живут ТОЛЬКО в Model Gateway/Broker-слое, агентам не передаются |
| **Конфиг-поверхности** | все — с классификацией committed intent vs local resolution (state-and-truth §8) |

## 2. Безотказность: инженерный контракт

Свойство, из которого выводится всё остальное (R3): **projection DB —
rebuildable**: `git (код+артефакты) + tracker + audit-журнал` достаточны для
пересборки операционного состояния. Поэтому катастрофа деградирует до
«пересобрать проекцию», а не «потеряли фабрику».

1. **Идемпотентные переходы:** каждый переход state machine — CAS-запись
   (`WHERE status = expected`) + уникальные ключи на lease; повторная
   доставка webhook/события не создаёт дублей (webhook delivery-id
   дедупликация).
2. **Crash-recovery по ролям:** `ff-api`/`ff-scheduler` — stateless,
   рестарт = чтение Postgres; упавший агентский ран — lease истекает по TTL
   → expiry policy (`requeue | fail_human | kill_runtime`); упавший
   верификатор — ран возвращается в `awaiting_verifier`. Никакое состояние
   не живёт только в памяти процесса.
3. **Reconcile как иммунная система:** 6-source reconcile (issue, labels,
   PR, commits, artifact state, runtime) + 5 drift-контуров на едином
   verdict enum; `mismatched-refuse` никогда не авточинится (quarantine+HAQ).
   Polling-fallback переживает потерю webhook'ов.
4. **Атомарность записей:** tmp-rename для emitted-артефактов; транзакции
   Postgres для переходов+audit (один commit); literal-body writes в MCP.
5. **Backup-модель:** git — push (артефакты+код уже реплицированы);
   Postgres — ежедневный dump + WAL-архив опционально; object store —
   rsync/versioning. Restore-тест — часть DoD Phase 1 («первый инцидент
   невосстановимости» — flip-сигнал №2, лучше не дожидаться).
6. **Тампер-видимость:** hash-chain audit_events проверяется фоновым
   джобом; разрыв цепи = инцидент.
7. **Деградация без падения:** фордж недоступен → ingest копится
   (polling-очередь), фабрика продолжает текущие раны; ForgePlan binary
   сломан → gate-проверки фейлятся fail-closed (не silent-pass); Model
   Gateway недоступен → раны паркуются в `retry_scheduled`, не теряются.
8. **Версионная дисциплина:** pinned `forgeplan` binary + health smoke на
   каждом runner; централизованные миграции (export → upgrade → migrate →
   health) — правило R4.

## 3. K8s на будущее: ready by design

**Дизайн-гарантии, закладываемые в Phase 1 (бесплатные сейчас):**

1. **Stateless бинари:** `ff-api`, `ff-scheduler` не держат состояния вне
   Postgres → в K8s это обычные Deployments; scheduler — replicas=1 c
   leader-election через Postgres advisory lock (закладывается сразу —
   это одна строка, а не рефакторинг).
2. **12-factor конфиг:** env + файлы, никаких «локальных путей в коде»;
   секреты через env → K8s Secrets без изменений.
3. **Health/readiness endpoints** в `ff-api` с первого дня → K8s probes.
4. **OTel-совместимые spans** (`tracing` crate, sink = файл) → в K8s замена
   sink на collector, инструментация не трогается.
5. **Single-writer через leases в DB, не через локальные mutex** → работает
   при N репликах и N нодах без переделки.
6. **Runner'ы как отдельная плоскость:** агентские worktrees и executors
   уже изолированы от control plane процессов → в K8s это Jobs/Pods.

**Целевая K8s-форма (Phase 5+, по flip-сигналам: второй хост / второй
оператор / multi-tenant):**

- `ff-api` Deployment (HPA), `ff-scheduler` Deployment (1 replica +
  leader-election), Postgres — managed/CNPG, MinIO/S3, ingest webhook —
  через Ingress.
- **Агентские раны = Kubernetes Jobs:** один ран → один Pod (executor CLI +
  spawned `forgeplan serve` + worktree в ephemeral volume / PVC c git clone
  --depth); лимиты CPU/mem/disk на Pod = budget envelope в железе; network
  policy per tier (T2/T3 без выхода в интернет кроме Model Gateway — RCE
  boundary из security-trust.md).
- **Оператор (CRD) — только когда фабрика станет платформой** для
  нескольких команд/проектов. Эскиз CRD: `AgentPool` (executor, model
  allowlist, capacity, tier), `FarmWorkload` (репо/store, playbook, autonomy
  profile). Operator reconcile = наш же verdict enum (resolved /
  missing-recoverable / mismatched-refuse) — K8s-оператор философски
  совпадает с нашим Drift Detector, поэтому ляжет естественно.
- **Чего не делать раньше времени:** не строить CRD/operator на MVP;
  не тащить Temporal/NATS «потому что K8s»; не делать web-UI зависимым от
  кластера. Анти-паттерн №16 (enterprise-стек авансом) остаётся в силе.

## 4. Сессии executors (CC/Codex/OpenCode) — указатель

Управление сессиями (spawn/resume/headless, session-id, передача модели и
worktree per run) — отдельный документ `executor-sessions.md`, пишется по
результатам фактического research (headless-режимы и per-run параметризация
CC/Codex/OpenCode). Контрактная рамка уже зафиксирована: ExecutorDriver
(createRun/streamEvents/cancelRun/collectOutcome) + spawned `forgeplan serve`
per worktree + двухканальное состояние (контрактный + PTY-эвристика, Herdr
H-1) + persistent sessions под супервизором (Herdr H-5).
