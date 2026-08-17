# Открытые развилки: позиции, trade-off'ы, рекомендации, flip-сигналы

> Всё, в чём отчёты `research/R1…R5` РАСХОДЯТСЯ или оставляют выбор открытым.
> По каждой развилке: позиции сторон → реальный trade-off → рекомендация для
> данного контекста (solo builder, local-first этос, Rust-скиллы, ForgePlan уже
> существует, model-agnostic eval vision, человек только на review/approve) →
> **flip-сигнал** — конкретное evidence, которое перевернёт рекомендацию.
> Когда развилка решена — фиксировать в `../decisions/D-NNN-*.md`.

---

## 1. Оркестрационное ядро: framework (LangGraph.js / Temporal) vs custom Rust

**Позиции.** R1 делает LangGraph.js workflow-ядром (StateGraph, checkpoints, interrupts, time-travel) + BullMQ для capacity, Temporal явно отложен. R2 ставит Temporal в центр как durable workflow engine, LangGraph понижен до T0/T1 reasoning-графов, Rust — только низкоуровневые git/scanner/guardian сервисы. R3 отвергает любой framework как ядро: custom Rust control plane (scheduler, lease manager, policy engine, audit log) управляет подключаемыми headless runtime'ами (OpenCode/Codex/LangGraph/Deep Agents) за типизированным ExecutorDriver. R4/R5 ядро не выбирают, но их форма (control plane + projection DB) ближе к R3.

**Trade-off.** Framework даёт durable-execution семантику бесплатно (retries, многодневные паузы, human approvals, replay — вся ценность Temporal; checkpoints/interrupts LangGraph), но ценой TS/JS-мозга над Rust artifact kernel, lock-in и ops-footprint (Temporal server, Redis для BullMQ). Custom Rust даёт детерминизм, один язык с ForgePlan, крошечный ops footprint и чистый seam для pluggable execution plane — но lease TTLs, heartbeats, retry budgets, resumable state строятся руками, с риском тихо переизобрести худший Temporal.

**Рекомендация: Rust-first (позиция R3).** Центр тяжести этой системы — lease-семантика, claim manager, policy gates, audit chain, ForgePlan-interop — ровно то, чего ни один framework не даёт. Доменную state machine (статусы, leases, gates) придётся владеть кодом при любом движке; Postgres + SKIP LOCKED закрывает очередь на PoC. LangGraph/Deep Agents — строго как T0/T1 runtime за ExecutorDriver, никогда как мозг. Temporal+K8s (R2) отвергнуть сейчас: team-scale стек, прописанный solo builder'у.

**Flip-сигнал.** Если PoC покажет, что >30–40% времени разработки control plane уходит в durable-execution plumbing (crash-resume многочасовых ранов, «парковка» approvals на дни, replay/debug зависших workflow), а не в доменную логику — это сигнал, что Temporal решал реальную проблему; взять его как внешнюю оболочку (критерий отсрочки самого R1: появление multi-day saga/compensation семантики).

---

## 2. Инфраструктура MVP: production-стек авансом vs local-first compose

**Позиции.** R2: Kubernetes + Argo CD GitOps + OTel/Tempo/Prometheus + SLSA/Cosign supply chain на 7 фаз. R1 посередине: Postgres + Redis/BullMQ + LangGraph, observability как first-class phase-5. R3 и R4 local-first: docker compose (postgres, minio, forgejo), `cargo run` для api/scheduler, Postgres-as-queue, NATS «потом», `activity-stats` ForgePlan как достаточная стартовая телеметрия.

**Trade-off.** Production-стек авансом даёт operability/security/team-handoff, но каждый компонент — ежедневный ops-налог solo builder'а при нулевой orchestration-функциональности. Local-first шипит контракты (leases, gates, projections) быстрее всего, но откладывает hardening — а retrofit observability исторически болезненный.

**Рекомендация: local-first (сторона R3+R4).** docker compose + Postgres + пара Rust-бинарей (`ff-api`, `ff-scheduler`), без Redis, без K8s, без ArgoCD, без NATS. Из R2 забрать два дешёвых пункта уже сейчас: (1) hash-chained таблица `audit_events`; (2) `tracing` crate со структурированными OTel-совместимыми span'ами в локальный файл — тогда observability-retrofit сводится к замене sink'а, а не переинструментированию. Остальные ~90% R2 — enterprise-косплей для этого контекста.

**Flip-сигнал.** Первый multi-machine runner pool (агенты на >1 хосте), второй регулярный оператор-человек, или первый инцидент «потерянный ран не восстановился из Postgres + git» — любой из трёх оправдывает NATS/managed-Postgres/настоящие дашборды. K8s — только для multi-tenant/customer-facing деплоя; для личного использования, вероятно, никогда.

---

## 3. L0–L3: нейминг, семантика уровней, tiers-as-power vs tiers-as-contract

**Позиции.** Четыре несовместимых прочтения. R1: уровни = фазы пайплайна (L0 scope/decision, L1 planning/DAG/capacity, L2 implementation, L3 proof/merge/closure). R2: role contract (разрешённые действия, SLA, цена ошибки, evidence), модель = `capability_class`; **L1 владеет spec+verification**, L3 — дешёвый validate/repair детерминированными инструментами. R4: model-power-flavored (L0 = самые дорогие reasoning-модели) с маппингом на `calibrate` depth. R5: строго последовательная цепочка L0→L1→L2→L3, только L3 входит в Policy/Gate Engine. R3: объявляет весь нейминг семантической коллизией (SDD-слайды: L0=исполнители; у пользователя: L0=сильнейшие модели) и мандатит внутренний rename в **T0–T3** + risk-policy.yaml.

**Trade-off.** Power-tiers интуитивны и ложатся на cost-интуицию (дорогой reasoning сверху, дешёвые кодеры снизу), но «какая модель» меняется ежемесячно, а «какие действия разрешены и какие evidence обязательны» — стабильно. Прочтение фазы-vs-роли меняет владельца верификации: T1 (R2) или T3 (остальные). Последовательная цепочка (R5) проста, но запрещает плотные repair-циклы T2↔T3, которые хотят все остальные.

**Рекомендация.** Принять rename R3 в **T0–T3 немедленно** (дёшево, предотвращает реальную путаницу людей и агентов) и семантику role contract R2: tier = разрешённые типы действий + требуемые evidence; модель — параметр `capability_class`, резолвится routing policy. Спор T1/T3 решить так: **T1 = spec/design/verification reasoning; T3 = deterministic-tool-heavy validate/repair/evidence-normalization**; независимость верификатора (generator≠verifier) — сквозной инвариант, не уровень. Отбросить строгую цепочку R5 в пользу policy-gated переходов с repair-циклом T2↔T3. Это единственное прочтение, совместимое с eval-vision: `capability_class` — ровно тот слот, который заполняет eval-кортеж.

**Flip-сигнал.** Если eval-данные покажут, что дешёвые модели + детерминированные инструменты системно пропускают verification failures, которые ловят сильные модели (>10–15% вердиктов T3 опрокидывается на human review) — верификация мигрирует вверх в T1 на сильные модели, ровно по split'у R2. Role contract переживает; двигается только capability_class binding.

---

## 4. Memory substrate: какой vector store и нужны ли Mem0/LightRAG/Qdrant

**Позиции.** Слоистость признают все, субстрат разный. R1: pgvector в run-store Postgres, LightRAG на v2, Mem0/Hindsight-like для cross-session. R2: pgvector базово, Qdrant только при search-heavy, LightRAG только как graph overlay, Mem0 только как standalone memory service. R3: LanceDB + fastembed-rs (Rust-native, совпадает со стеком ForgePlan) для PoC/mono-repo, Qdrant/pgvector только на multi-repo масштабе, строгий приоритет artifacts > policy > retrieval > hindsight.

**Trade-off.** pgvector держит память транзакционной с run state (joins, одна backup-story), но добавляет embedding-plumbing в control plane. LanceDB+fastembed переиспользует ровно то, что ForgePlan уже шипит (BGE-M3, embedded, ноль новых сервисов), но держит episodic вне projection DB. Mem0/LightRAG/Qdrant — каждый добавляет сервис в ops и вторую семантическую истину для reconcile.

**Рекомендация: ForgePlan-native (субстрат R3).** LanceDB/fastembed для artifact retrieval (уже построено), существующий Hindsight для episodic/conversational, обычные Postgres-таблицы (БЕЗ pgvector) для run-ретроспектив с ключами task-type/repo — eval-петле нужны structured-запросы по outcome'ам сильно раньше, чем semantic similarity. Приоритет artifacts > policy > retrieval > hindsight записать как письменный инвариант с первого дня. Mem0, LightRAG, Qdrant, GraphRAG — безусловный reject на MVP: каждый помечен в корпусе как conditional, и ни одно условие не выполнено.

**Flip-сигнал.** pgvector — когда конкретная recall-задача структурно провалится: «найди прошлые раны с этим failure_class на похожем коде» не выражается keyword/structured-запросом. Qdrant — только multi-repo масштаб с измеренными latency/recall проблемами pgvector. Mem0/LightRAG — потребовалась бы память как сетевой сервис для не-ForgeFarm клиентов; для этого пользователя — вряд ли когда-либо.

---

## 5. Транспорт ForgeFarm ↔ ForgePlan: spawned stdio MCP workers vs CLI subprocess

**Позиции.** R4 формулирует ограничение жёстче всех: `forgeplan serve` — stdio-only, значит оркестратор — супервизор worker-процессов; развилку «spawned serve workers vs CLI/MCP host abstraction» оставляет открытой. R1: «ForgePlan CLI + MCP only» без выбора. R3 показывает оба: enrichment-пайплайны зовут health/context/drift/validate (CLI-shaped), агенты получают MCP wiring (`{command: forgeplan, args: [serve]}`). R2 отмахивается «CLI/MCP interop».

**Trade-off.** Долгоживущие per-worker MCP-процессы дают низкую latency на вызов и session-непрерывность, но делают оркестратор process-супервизором (lifecycle, restarts, workspace binding, version pinning по правилу R4) и множат LanceDB-держащие процессы по worktrees. CLI subprocess-per-call — stateless, тривиально версионируется и тестируется, совпадает с тем, как ForgePlan уже потребляется в CI (`--ci` флаги), но платит process-spawn + index-load на каждый gate check и теряет session-семантику.

**Рекомендация: split по плоскости.** Gates и reads control plane'а (`health --ci`, `validate --ci`, `drift --ci`, `score`, `order`, `graph`) — CLI subprocess с JSON-выводом: batch, идемпотентно, 10-шаговый CI-пайплайн R3 уже потребляет их так. Агентские workers — spawned stdio `forgeplan serve` внутри их собственного worktree (для чего и построен PRD-078 workspace param), владеет тот же runtime adapter, что владеет worktree. Никакого standalone MCP worker pool: ForgeFarm супервайзит MCP-процессы только там, где уже существует агентская сессия. Две дисциплины R4 с первого дня: literal-body writes (никогда `@file` через MCP — урок v0.32.1) и pinned binary + health smoke per runner.

**Flip-сигнал.** Замерить cold-call CLI (старт бинаря + открытие LanceDB) на реальном workspace. Если gate-проверки >1–2 с каждая, а scheduler гоняет их per-transition на swarm-частоте — control plane тоже переходит на один резидентный serve-процесс per workspace. Обратно: если upstream когда-нибудь шипнет HTTP transport для `forgeplan serve` — вся супервизорная сложность схлопывается, решение пересмотреть.

---

## 6. Run-state словарь vs проекция ForgePlan session phases

**Позиции.** Все согласны: kanban — проекция. Но проекция ЧЕГО? R4 выводит колонки доски прямо из `forgeplan session` phases (Shaping/Coding/Evidence/Review-PR = session.phase) + lease/graph, предостерегая от параллельных статусов. R1 определяет 11-значный машинный run-status словарь. R3 — 16-статусную task state machine + 12 событий + hash-chained audit, 12-lane kanban. R2 — свои TASK/TASK_RUN + 7 UI-surfaces; R1 — 4 surfaces; R3 добавляет Human Attention Queue как отдельный surface.

**Trade-off.** Переиспользование session phases избегает параллельного словаря и держит методологию канонической в одном месте — но session phase это per-workspace методологическое состояние, структурно не выражающее leases, retry budgets, merge blockers, verifier verdicts, fail-loop причины (перечень R1 «что не выражает kanban-колонка» бьёт и по session phases). Полная 16-статусная машина честна к реальности оркестрации, но это вторая state machine, которую держит консистентной reconcile.

**Рекомендация.** Своя run state machine в projection DB (R3, обрезанная — стартовать с ~10 статусов, добавлять merge_pending/expired_lease когда появятся code paths); `forgeplan session`/phase — один из ВХОДОВ reconcile-петли (ровно как 6-source reconcile R5), не носитель. Маппинг колонок взять из R4 (его Fail = формальный список причин — лучшая fail-loop спецификация корпуса). UI на старте — ровно 2 surface: Board (проекция) + Run Inspector/HAQ вместе, потому что оператор один. Fail Lab, Governance Console, Memory Explorer — team-scale, дорастать.

**Flip-сигнал.** Если после первого реального orchestration loop reconcile покажет, что run machine и session phases почти изоморфны (переходы всегда со-происходят, run-only статусы кроме leases не заселяются) — схлопнуть к тонкой проекционной модели R4. Обратно: устойчивая глубина HAQ >5–10 items или повторяющаяся боль fail-triage — сигнал выделять Fail Lab.

---

## 7. Фордж: GitHub vs self-hosted Forgejo; issues как «system of record» vs «mirror»

**Позиции.** R1 и R4 Forgejo-специфичны и называют его «system of record for tasks» с Actions как execution substrate. R3 и R2 сознательно tracker-agnostic и понижают issues до «work intents + human-facing surface». R5 идёт дальше всех: tracker labels — только «внешнее зеркало статуса», никогда не доверять, обязательный 6-source reconcile.

**Trade-off.** Forgejo даёт self-hosted local-first alignment, org/system webhooks, свои раннеры — но это целый сервис в ops, его Projects API under-documented (предупреждение самого R4), а реальная жизнь пользователя сегодня на GitHub (репо ForgePlan, gh CLI, существующий CI). «System of record» vs «mirror» определяет, может ли ingestion доверять трекеру или обязан всегда reconcile'ить: доверие проще, reconcile корректен при параллельных агентах.

**Рекомендация.** Принять строжайшее прочтение R5 независимо от форджа: issues = intent ingress + status mirror, обязательный reconcile, не доверять; фразу «system of record for tasks» изъять из дизайн-языка (planning truth живёт в projection DB с момента ingestion). По самому форджу: tracker-agnostic за тонким адаптером (framing R3), MVP на GitHub — там живёт пользователь; self-hosted Forgejo — постоянный ops-налог, который ничего не покупает, пока agent-CI не потребует раннеров, которых GitHub дёшево не даст. Dual-CI трюк R2 (`.forgejo/workflows` fallback → `.github/workflows`) делает миграцию почти бесплатной — ровно поэтому за выбор не надо платить сейчас.

**Flip-сигнал.** Self-hosted Forgejo становится правильным, когда: GitHub Actions minutes для swarm-CI станут ощутимой статьёй; агентам понадобятся network-isolated раннеры, которых GitHub не даёт; или приватность task stream станет требованием. Любой из трёх флипает MVP-фордж; adapter boundary делает флип днями, не неделями.

---

## 8. Очерёдность map-pack: flagship-first vs mid-phase

**Позиции.** R1 делает map-pack первой флагманской композицией — «ForgeFarm в миниатюре», референс-шаблон для семейства (spec-build, review-pack, evidence-pack, release-pack) — после починки его 5 пробелов. R2 ставит его Phase 5 из 7, после execution kernel и reasoning lanes. R3 — Gamma milestone: третьим, после control-plane spine (Alpha) и ForgePlan-gate интеграции (Beta).

**Trade-off.** Flagship-first прогоняет каждый контракт (single writer, scratch isolation, generator≠verifier, детерминированные gates, fail loop) на low-risk детерминированной нагрузке до появления кодящих агентов — отличная валидация и полезный продуктовый артефакт (map, который пользователь уже хочет — см. project_map_generator). Но спецификация map-pack сама предполагает control-plane примитивы (exclusive write lease эмиттера, per-run scratch, gate engine, fail queue) — строить его первым значит строить примитивы ad hoc внутри пайплайна и потом извлекать.

**Рекомендация: разделить разницу, склоняясь к R3.** Сначала минимальный spine (projection DB, task state machine, lease manager, gate engine stub — Alpha R3), затем map-pack как ПЕРВЫЙ workload на этом spine — до любой кодящей lane. Сохраняет инсайт R1 (map-pack как проверочная композиция) без примитивов-внутри-пайплайна. Пять пробелов R1 чинить при постройке — это лучшая pre-implementation критика корпуса, все пять выдерживают проверку. НЕ следовать порядку R2 (кодящие агенты до map-pack): пускать T2 code-writers до того, как gate/lease-машинерия доказана на детерминированной нагрузке — инверсия лестницы рисков.

**Flip-сигнал.** Если standalone map generator (dev/forge-understand) созреет как продуктовый deliverable на собственном таймлайне — шипнуть его как plain deterministic CLI pipeline вне ForgeFarm, re-ingest как композицию позже: продуктовая срочность бьёт архитектурную очерёдность. Обратно: если spine займёт >4–6 недель solo — резать Alpha дальше, но не пускать map-pack вперёд очереди.

---

## 9. Eval-кортеж: где живёт (единогласный пробел корпуса с реальной развилкой субстрата)

**Позиции.** Ни один отчёт его не проектирует; каждый оставляет свой заглушечный намёк. R1: phase-5 golden tasks + judge/eval пайплайны (LangSmith-flavored). R2: «model = capability_class параметр» + пустой `POLICIES/model-routing.yaml`. R3: признаёт, что eval-tool design не существует, паркует «model routing optimization» в Scale-фазу; cost-by-tier как метрика Control Room. R4: единственный ForgePlan-native хук — `calibrate` depth как вход routing policy. Развилка: eval-результаты как ForgePlan Evidence, питающие routing (ForgePlan-native) vs выделенная eval-подсистема в control plane (framework/LangSmith или projection-DB таблицы).

**Trade-off.** Evidence-native даёт eval-вердиктам R_eff scoring, decay/TTL, typed links и human review через уже существующую машинерию — буквально заявленная vision («eval кормит оркестратор как evidence»), но EvidencePack — per-artifact прозаический формат, возможно слишком грубый для тысяч кортежей, а min()-семантика R_eff спроектирована для доверия решениям, не для model routing. Таблица в projection DB — правильная форма для aggregation/routing-запросов, но создаёт вторую evidence-систему вне trust calculus ForgePlan, нарушая доктрину единственного источника истины самого корпуса.

**Рекомендация: двухслойный split, решается СЕЙЧАС (схема Phase 1 должна захватывать raw data).** (1) Каждый TaskRun пишет структурированную outcome-строку (model, harness/runtime, task_type, tokens/cost, gate results, retry count, human interventions, verifier verdict) в projection DB с первого дня — дёшево, это сырой eval-корпус. (2) Периодическая агрегация дистиллирует строки в ForgePlan EvidencePacks («claude-x на T2 rust-impl: 78% gate-pass, $0.4/task, 0.2 interventions/task, n=40»), linked к routing-policy ADR. Routing-ИСПОЛНЕНИЕ читает быстрые агрегаты DB; routing-ИЗМЕНЕНИЯ — evidence-backed решения в ForgePlan. LangSmith как eval-spine — reject (запирает model-agnostic vision в один vendor lane). Маппинг R4 calibrate-depth→tier — корректная v1 routing policy, пока не накопились реальные кортежи.

**Flip-сигнал.** Если после ~100–200 ранов дистилляция aggregate→Evidence — церемония, которую ни одно routing-решение не читает, — отбросить слой 2, оставить eval чисто в DB. Если наоборот routing-решения начнут оспариваться («почему модель X на T1?») — это валидация evidence-слоя: ответом должен быть linked EvidencePack, а не археология конфиг-файла.
