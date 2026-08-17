# Плоскости ForgeFarm: нормализованная модель

> Сведение архитектур пяти отчётов в одну модель плоскостей. Названия слоёв
> в отчётах различаются, состав — сходится. Принятая (рекомендованная) форма —
> по R3 (Rust-first), с UI-инсайтами R1/R2 и ingestion-деталями R4/R5.

## Принятая модель: 6 плоскостей

```
┌─────────────────────────────────────────────────────────────────┐
│  UI (operator console)          Board · Run Inspector · HAQ     │  ← читает только Projection DB + audit
├─────────────────────────────────────────────────────────────────┤
│  CONTROL PLANE (Rust, детерминированный)                        │
│  API Gateway · Projection Builder · Scheduler · Lease Manager   │  ← ЕДИНСТВЕННОЕ место переходов состояния;
│  Policy/Gate Engine · Runtime Broker · Worktree & Merge         │    каждый переход = audit_event
│  Governor · Drift Detector · Audit Service · Memory Orchestrator│
├─────────────────────────────────────────────────────────────────┤
│  RUNTIME PLANE (pluggable, за ExecutorDriver)                   │
│  адаптеры: Claude Code / OpenCode / Codex CLI / LangGraph /     │  ← исполняет T0–T3 в изолированных worktrees;
│  Deep Agents · sandbox worktrees · spawned forgeplan serve      │    никогда не владеет каноническим состоянием
├───────────────────────────┬─────────────────────────────────────┤
│  ARTIFACT KERNEL          │  TRACKER (ingress/egress)           │
│  ForgePlan: .forgeplan/   │  GitHub/Forgejo: issues, PRs,       │  ← две внешние истины;
│  markdown = truth,        │  labels (projection-mirror only),   │    ForgeFarm их читает/зеркалит,
│  LanceDB derived,         │  webhooks (signed), Actions/CI      │    никогда не владеет
│  CLI/MCP only             │                                     │
├───────────────────────────┴─────────────────────────────────────┤
│  OPERATIONAL STORAGE                                            │
│  Postgres Projection DB (rebuildable, never authoritative) ·    │
│  hash-chained audit_events · object store (логи/evidence) ·     │
│  retrieval index (LanceDB → pgvector/Qdrant по flip-сигналу)    │
└─────────────────────────────────────────────────────────────────┘
```

## Ответственности плоскостей (и что каждой ЗАПРЕЩЕНО)

| Плоскость | Владеет | Запрещено |
|---|---|---|
| **Tracker** | work intents (issues), PR/merge механика, permissions boundary, webhook events, CI checks | быть state engine; labels как runtime state; Projects board API как primary store |
| **Artifact kernel (ForgePlan)** | PRD/RFC/ADR/Spec/Evidence, typed links, R_eff, lifecycle gates, semantic search | быть оркестратором/трекером/dashboard-backend; принимать записи мимо CLI/MCP |
| **Control plane** | переходы состояния, очереди, leases, gates, dependencies, health, telemetry, audit | писать код/артефакты сам; продуктовые решения в Policy Engine; запускать агентов из Projection Builder |
| **Runtime plane** | исполнение T0–T3 в worktrees, agent sessions, sandbox | канонические состояния; переходы state machine; записи вне scope lease |
| **UI** | рендер проекций + audit, human approvals (HAQ) | собственное состояние; прямые мутации (всё через API control plane) |
| **Storage** | операционная материализация | быть авторитетным (всегда rebuildable из git + tracker + ForgePlan) |

## Компоненты control plane (по R3, канонический список)

| Компонент | Делает | НЕ делает |
|---|---|---|
| Projection Builder | материализует состояние из Git/tracker/ForgePlan | не запускает агентов |
| Scheduler | выбирает runnable задачи по DAG/риску/capacity | не пишет в Git |
| Lease Manager | честные task+scope локи (TTL, heartbeat) | не синкает labels |
| Policy/Gate Engine | YAML-политики + детерминированные проверки | не принимает продуктовых решений |
| Runtime Broker | выбирает executor/model/tier per task | не держит канонического состояния |
| Worktree & Merge Governor | ветки, worktrees, rebase, PR-дисциплина | не ставит приоритеты |
| Drift Detector | artifact↔code, map, workflow drift (5 контуров) | никогда не авточинит без policy |
| Audit Service | append-only журнал с hash-chain, evidence links | — |
| Memory & RAG Orchestrator | hindsight/retrieval/memory scopes | никогда не источник истины |

## ExecutorDriver — seam между control и runtime plane

Типизированный контракт (форма из R3): `createRun / streamEvents / cancelRun / collectOutcome`; типизированные RunEvents: `status_changed, tool_called, file_read, file_write_attempted, patch_generated, test_result, artifact_proposed, gate_request, memory_write, error, heartbeat`. `tier` (T0–T3) — first-class поле createRun. Через этот seam любой агентский framework (Claude Code, OpenCode, Codex CLI, LangGraph, Deep Agents) — сменный адаптер.

## Как отчёты называют те же слои (для сверки с первоисточниками)

| Принятый слой | R1 (audit) | R2 (prodstack) | R3 (rustfirst) | R4 (plansform) | R5 (sdd) |
|---|---|---|---|---|---|
| Tracker | Ingress Layer | Tracker Plane | Tracker | Task & execution plane | Tracker + evidence plane |
| Artifact kernel | Artifact Gateway | Artifact Plane | ForgePlan Artifact Core | Methodology & artifact plane | ForgePlan Artifact Plane |
| Control plane | Workflow Kernel + Queue + Run Store | Control Plane | ForgeFarm Control Plane | Orchestration & visibility plane | Control Plane + Ingestion |
| Runtime plane | (внутри Kernel: LangGraph) | Runtime Plane (Temporal+LangGraph+DeepAgents) | Headless Runtime Plane | Agent pools + execution substrate | Headless Agent Runtime + Ladder |
| UI | Product UI (4 surfaces) | UX Plane (7 surfaces) | ForgeFarm UI (Control Room + 12-lane kanban + HAQ) | Realtime UI (derived board) | UI (5 panels) |
| Storage | Run Store (Postgres+pgvector) | Projection DB | Operational Storage (17 таблиц) | Telemetry store | Projection DB |

Расхождение «кто движок control plane» (LangGraph vs Temporal vs Rust) — решено в пользу Rust: см. [../synthesis/02-open-decisions.md](../synthesis/02-open-decisions.md) §1.

## UI: старт с двух surfaces

Корпус предлагает от 4 (R1) до 7 (R2) поверхностей. Принято (решение из развилки №6): на MVP ровно **две** — **Board** (проекция state machine, маппинг колонок из R4) и **Run Inspector / Human Attention Queue** (объединённый). Fail Lab, Governance Console, Memory Explorer, Artifact Explorer — team-scale, дорастать по flip-сигналам. Главная продуктовая сущность UI — **Run**, не задача и не чат (R1: «control tower for AI-native delivery»).
