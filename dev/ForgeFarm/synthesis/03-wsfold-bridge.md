# Мост из WSFold/constellation-анализа: что течёт в ForgeFarm

> Сессия 2026-06-29 — 2026-07-02 исследовала WSFold (atilarum/wsfold,
> «package manager для git-репозиториев») и через 3 адверсариально
> верифицированных workflow вывела **границу абсорбции для fpl core**:
> fpl владеет слоем ГРАФА артефактов, не слоем файловой системы/процессов.
> Этот документ фиксирует, как те выводы стыкуются с корпусом ForgeFarm —
> где подтверждаются, где расширяются, и какие конкретные дизайн-входы дают.
> Полная память сессии: `~/.claude/projects/-Users-explosovebit-Work-ForgePlan/memory/project_wsfold_evaluation.md`.

---

## 1. Граница абсорбции — держится и усилена корпусом

Все три больших отчёта (R1, R2, R3) независимо сошлись ровно на той границе, которую чат вывел адверсариально: ForgePlan = artifact kernel, никогда не оркестратор («wrap, don't replace»); ForgeFarm = отдельный execution/control plane. Корпус дополнительно жёстко запрещает ForgeFarm писать артефакты иначе как через fpl CLI/MCP (Artifact Gateway, никаких записей в LanceDB) — зеркальное отражение чатового «no cross-store writes, каждый стор мутируется только своим MCP/CLI».

**Дизайн-следствие:** решение об абсорбции WSFold→fpl ревизии не требует. fpl core получает только constellation (read-only artifact-graph span) + dispatch hints; ForgeFarm подтверждён как правильный второй продукт. RED-LINE #11 распространяется через границу продукта: Artifact Gateway ForgeFarm контрактно ограничен вызовами `forgeplan` CLI/MCP — записать это как **ForgeFarm ADR-001 (source-of-truth model)**.

## 1b. ForgeFarm — законный дом всего отвергнутого из fpl, но в ДВУХ разных plane'ах

Каждый пункт, отвергнутый из fpl, имеет именованный компонент корпуса:

| Отвергнуто из fpl core | Дом в ForgeFarm | Plane |
|---|---|---|
| worktree provisioning | Worktree & Merge Governor (R3) / worktree manager (R1 Phase 3) | control plane (Rust, детерминированный) |
| sandbox wiring, writable-roots | runtime-plane sandbox worktrees + runner-as-RCE-boundary security model | runtime plane |
| agent auto-summon | Runtime Broker + ExecutorDriver adapters | runtime plane |
| trust classes | таксономия read / safe-write / privileged-write + risk-policy.yaml | control plane |
| dispatch-который-спавнит | Scheduler + Lease Manager | control plane |

**Дизайн-следствие:** при проектировании ForgeFarm не сливать обратно то, что чат разделил: provisioning-механика и trust/policy enforcement → детерминированный Rust control plane; agent execution и sandbox wiring → за типизированный ExecutorDriver seam, чтобы фреймворки (LangGraph/Deep Agents/OpenCode) оставались сменными. «Система управляет фреймворками, а не наоборот».

## 2. constellation.yaml = cross-repo половина ingestion-контракта ForgeFarm

Projection DB корпуса (R2/R3) определена как rebuildable, never-authoritative материализация над Git + tracker + ForgePlan — но все отчёты предполагают ОДИН `.forgeplan/` на репо. Чатовая constellation (committed store handles + store-qualified slug IDs + `fpl graph|health|order --span` read-only union + ghost nodes) — ровно недостающая multi-repo read-поверхность.

**Дизайн-следствия:**
- Projection Builder ForgeFarm ингестит через `--span` команды fpl, а НЕ ползает по N репозиториям сам;
- ключи артефактов в таблицах ForgeFarm — **store-qualified slugs** (`web:prd-auth-system`): display numbers присваиваются CI per-store и коллидируют;
- ghost/missing stores всплывают в проекции с явным resolution state, не как ошибки;
- **обратное жёсткое требование к fpl: стабильный машиночитаемый `--json` на graph/order/health/dispatch** — у fpl появляется программный потребитель, не только агенты, читающие хинты.

## 2b. Cross-store R_eff федерация остаётся deferred в fpl — навсегда

Чат отложил федерацию за ADR (min() weakest-link молча зануляется на любом foreign draft). Gate-движки корпуса (artifact-readiness, evidence_present, activation_gate) хотят скоры, но федерацию тоже не специфицируют. Решение: **ForgeFarm читает per-store R_eff и комбинирует на уровне Policy/Gate Engine** — например «гейтить по скорам артефактов, которые Run читает; игнорировать несвязанные foreign drafts». fpl никогда не шипит federated min(): вопрос комбинирования уезжает из fpl core в policy engine ForgeFarm, и это правильный владелец — комбинация это risk-policy решение, не scoring identity.

## 3. Dispatch: слои, не замена

Lease-машинерия корпуса (task lease TTL 10–30 мин + heartbeat, scope leases по path-globs, 7-полевые lease-записи, Claim Manager, speculative branch lanes) очевидно превосходит TTL-локи fpl dispatch на farm-масштабе. Но собственно мозг dispatch — conflict-free artifact buckets + serial queue из typed link graph — ровно тот вход, который нужен Claim Manager и DAG builder корпуса, и который они не специфицируют. R4 говорит явно: топологическая сортировка fpl «уже концептуально существует внутри ForgePlan и не должна переизобретаться в БД Plansform».

**Дизайн-следствие — разделение труда:**
- **worktree hints в fpl шипнуть всё равно** — standalone-путь (без ForgeFarm) остаётся first-class;
- `fpl dispatch --json` / `fpl order --json` = planning primitive, который потребляет Scheduler ForgeFarm;
- fpl считает conflict-free partitioning (single-store, advisory); ForgeFarm владеет leases, enforcement и cross-repo concurrency.
- Формула: hints = human/agent UX; JSON = machine contract; leases = ForgeFarm.

## 4. WSFold-дисциплина манифеста → Worktree Governor: три правила

Committed-intent vs machine-local-resolution split WSFold — то, что корпус практикует неявно (committed playbooks/policies/risk-policy.yaml vs runtime Projection DB), но не называет принципом. Трёхсостоянийная модель WSFold (attached/unmounted/invalid) ложится на worktree governance: healthy / absent-recoverable / diverged-dirty. Fail-loop-схемы корпуса (quarantine_reason, reentry_condition, human_required) — естественная посадочная площадка правила «invalid НИКОГДА не авточинится».

**Три правила Worktree Governor:**
1. **Идемпотентный provisioning:** healthy = no-op; missing = пересоздать детерминированно.
2. **Invalid (dirty/diverged worktree, например после lease expiry посреди рана) НИКОГДА не авточинится** — никакого `git reset --hard` по policy; только quarantine + Human Attention Queue с quarantine_reason.
3. **Каждая конфиг-поверхность ForgeFarm классифицируется:** committed intent (playbooks, policies, compositions, constellation.yaml) vs local resolution (Projection DB, leases, worktree paths, constellation.local.yaml). Смешение двух классов — design smell, ловится на review.

## 4b. Трёхсостоянийная reconciliation обобщается на все drift-контуры

5 drift-циркуитов R3 и обязательный 6-source reconcile R5 (issue.state, labels, PR state, commit evidence, ForgePlan artifact state, runtime state) — независимое переоткрытие корпусом reconciliation-дисциплины WSFold: сверять заявленный intent с наблюдаемой реальностью, классифицировать в healthy/recoverable/invalid, авточинить только recoverable-класс.

**Дизайн-следствие:** Drift Detector и Reconcile-слой ForgeFarm строятся вокруг ОДНОГО общего verdict enum — **resolved / missing-recoverable / mismatched-refuse** — переиспользуемого во всех контурах (worktrees, projections, labels, map.json, constellation stores). Auto-repair policy привязывается к классу вердикта, а не к каждому контуру ad hoc — тогда «never auto-repair invalid» остаётся одним enforce'ируемым инвариантом, а не пятью копиями.

## 5. gastown / swarm-forge: prior art, которого корпус не знает

Ни один из пяти отчётов не упоминает gastown или swarm-forge (оба лежат в `dev/`); все предлагают строить worktree manager и agent-session механику с нуля. gastown покрывает per-agent git-worktree-backed storage + mailboxes на масштабе 20–30 агентов; swarm-forge — tmux + per-role worktrees + inbox/outbox handoffs — то есть ровно runtime-plane и provisioning-механику. Чего у обоих НЕТ — всего, что корпус ставит в центр: leases, projection DB, policy gates, evidence-first close, audit chain.

**Дизайн-следствие:** закрыть висящий build-vs-buy как **«steal mechanics, reject as kernel»**. До постройки Worktree Governor и headless runtime lane — structured extraction pass: у gastown взять worktree lifecycle, per-agent storage, mailbox protocol (в корпусе есть RunEvents, но нет inter-agent handoff паттерна); у swarm-forge — session supervision, inbox/outbox handoffs (совпадают с формой agent-session.sh из R5). Ни один не становится control plane. Это де-рискует дважды задокументированную worktree-боль (PROB-060 shared-HEAD corruption, 20–72GB disk-fill) проверенным prior art вместо третьего переоткрытия.

## 6. Eval-кортеж — единственный настоящий пробел корпуса; вытянуть вперёд

Ядро vision пользователя — скоринг кортежа **(model + harness + task type + cost + quality + human interventions)** и подача его в routing как evidence — не разработано НИ В ОДНОМ из пяти отчётов. При этом каждый отчёт УЖЕ шипит сокеты, в которые eval вставляется:

| Сигнал eval-кортежа | Готовый сокет в дизайне корпуса |
|---|---|
| какая модель на каком tier | capability_class / Runtime Broker |
| какой tier на какой риск | risk-policy.yaml allowed_tiers |
| стоимость | budget envelopes, cost-by-tier/model/provider метрики |
| сырые сигналы | OTel GenAI-метрики, run events |
| interventions | HAQ entries, human_required counts |
| качество | verifier verdicts, failure_class rates, gate-pass rates |

**Дизайн-следствие:** писать ForgeFarm **ADR «eval-as-evidence routing loop» в Phase 0–1, не Phase 5** — откладывание воспроизводит слепое пятно корпуса и противоречит raison d'être продукта. Конкретная петля: run-store накапливает per-run кортежи из сигналов, которые ForgeFarm уже собирает → периодический distillation job агрегирует per (model, harness, task-class) → model-routing.yaml Runtime Broker'а обновляется только с цитатой на дистиллированное evidence → изменения routing становятся аудируемыми policy-диффами, а не вкусовщиной.

## 6b. EvidencePack/R_eff — decision-слой eval'а, не run store

Модель EvidencePack ForgePlan ложится на eval-семантику на уровне claim'ов неожиданно хорошо:

| EvidencePack поле | Eval-семантика |
|---|---|
| verdict (supports/weakens/refutes) | «модель X адекватна task-class Y» |
| congruence_level (CL3…CL0) | совпадение harness/task-type (CL3 = тот же harness + тот же класс задач; CL1–0 = перенесённый контекст) |
| valid_until / decay | устаревание eval'ов при выходе новой версии модели — ровно та TTL-семантика, которую R_eff уже enforce'ит |

Два несовпадения: (a) R_eff = min() weakest-link неправилен для агрегации ПОПУЛЯЦИИ eval-ранов (один плохой ран занулил бы модель — агрегация нуждается в распределительной статистике ДО авторинга claim'а); (b) git-tracked markdown не может впитывать high-volume per-run записи, не нарушая дух ADR-003.

**Дизайн-следствие — двухслойное eval-хранилище, совпадающее с 4-truths split корпуса:** сырые eval-раны = строки run-store ForgeFarm (Postgres: execution_runs, gate_decisions, cost/token metrics — операционная истина, большой объём); дистиллированные routing-claims = ForgePlan EvidencePacks (один EVID на claim model×task-class, со Structured Fields verdict/congruence_level/evidence_type=benchmark и valid_until, linked `informs` к routing-policy артефакту — decision-истина, малый объём, git-reviewed). Runtime Broker гейтит изменения routing-таблицы на активные, непросроченные EVID'ы. Тогда R_eff делает ПРАВИЛЬНУЮ weakest-link работу: routing-решение, опирающееся на просроченное/опровергнутое eval-evidence, видимо падает до 0.1 и всплывает в blindspots/health.

## 7. Сквозное: T0–T3 и stdio-супервизор

Два постановления корпуса, прямо корректирующие рамку чата: (1) R3 документирует семантическую коллизию L0–L3 (SDD-слайды: L0=исполнители; узус пользователя: L0=сильнейшие модели) и мандатит внутренние **T0–T3** enum'ы (L0–L3 допустимы только как UI-лейблы); (2) R4 выводит из stdio-only природы `forgeplan serve`, что ForgeFarm — process-СУПЕРВИЗОР, спавнящий per-worker serve-процессы (lifecycle, workspace mounts, изоляция), а не HTTP-клиент fpl.

**Дизайн-следствие:** T0–T3 во всём коде/API/схемах ForgeFarm с первого дня (rename после schema freeze дорог). fpl остаётся stdio-only — никакого запроса HTTP-транспорта назад в fpl core; worker lifecycle ForgeFarm спавнит `forgeplan serve` per worktree, и PRD-078 workspace param (уже шипнут) — ровно тот routing guard, который нужен supervised workers. Второе подтверждение, что чатовый rejection-list был прав: transport/process plumbing принадлежит ForgeFarm.

## 8. Итог: что чат даёт корпусу, что корпус даёт чату

**Чат вносит пять вещей, которых у корпуса нет:** constellation как multi-store ingestion; store-qualified slug identity; именованная WSFold-дисциплина three-state + never-auto-repair; prior art gastown/swarm-forge; контракт dispatch-JSON как planning primitive. **Корпус вносит весь скелет control plane, который чат сознательно отказался класть в fpl:** leases, state machines, gates, fail-loop, projection DB, security model.

**Для стартового artifact pack ForgeFarm** (R3 называет: EPIC-001, RFC runtime-and-lease-model, RFC task-state-machine, ADR source-of-truth) добавить три артефакта, рождённых чатом:
- ADR «constellation ingestion + store-qualified identity»;
- ADR «worktree three-state governance (never auto-repair invalid)»;
- prior-art EVID/Note по gastown + swarm-forge, закрывающий build-vs-buy.

И вытянуть eval-loop ADR вперёд из Phase 5 в Phase 0–1. Всё остальное из rejection-list чата подтверждено: отвергнуто из fpl, принято в ForgeFarm.
