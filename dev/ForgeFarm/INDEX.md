# INDEX — файловый RAG-роутер базы знаний ForgeFarm

> Назначение: по вопросу/задаче найти МИНИМАЛЬНЫЙ набор файлов для чтения.
> Не читать всё подряд — база спроектирована под точечную загрузку контекста.

## Правила чтения для агента

1. **Маршрут:** этот INDEX → точечный файл из таблиц ниже. Не грузить весь каталог.
2. **Иерархия авторитетности:** `decisions/` (зафиксировано) > `synthesis/01-consensus.md` (решено корпусом) > `synthesis/02-open-decisions.md` (рекомендовано, ждёт фиксации) > `architecture/`, `evals/` (нормализованные справочники) > `research/` (сырьё).
3. **`research/R*` — read-only первоисточники.** Ходить туда только для сверки формулировки/контекста или когда дистиллят ссылается на деталь (например «17-табличная схема R3», «5 пробелов map-pack R1»). Игнорировать `citeturn…`-маркеры.
4. **Инварианты из `01-consensus.md` не оспаривать** без нового сильного evidence; открытые развилки не перерешивать в чате — решение фиксируется файлом в `decisions/` (правила — в `decisions/README.md`).
5. **Термины:** уровни агентов в коде/схемах — только T0–T3 (L0–L3 = UI-лейбл, см. architecture/t0-t3-roles.md). «Board/labels» — всегда проекция, слово «носитель состояния» к ним не применять.
6. При работе над самим ForgePlan (не ForgeFarm) — этот каталог вторичен; главный контекст там: `~/Work/ForgePlan/CLAUDE.md`.

## Маршрутизация по вопросу

### Продукт и стратегия

| Вопрос | Файл(ы) | Что там |
|---|---|---|
| Что такое ForgeFarm? С чего начинать? | `synthesis/00-master-synthesis.md` | определение, решённое, развилки, фазированный план, анти-паттерны — самодостаточен |
| Что уже решено и не обсуждается? | `synthesis/01-consensus.md` | 31 инвариант (архитектура/ForgePlan/фордж/процесс) + 15 анти-паттернов с attribution |
| Какие решения ещё открыты? Какую сторону брать? | `synthesis/02-open-decisions.md` | 9 развилок: позиции → trade-off → рекомендация → flip-сигнал |
| Что зафиксировано окончательно? | `decisions/` | pre-ADR записи (пока пусто; правила в README) |
| Vision владельца: как формулировка маппится на KB? Автономный режим? Self-development? Методологии по типу задач? Комплектация скиллов/агентов? | `synthesis/04-vision-alignment.md` | маппинг-таблица + 4 требования: autonomy profile, self-hosting milestone, Methodology Router (routing-map → машинные playbooks), Agent Bundle Composer + дельта к Phase 0 |
| Идеи из Herdr (agent-aware мультиплексер)? Терминальный cockpit? Детекция зависших агентов? wait/status примитивы для агентов? | `synthesis/05-herdr-patterns.md` | 5 идей (эвристический fallback-канал состояния, HAQ-visibility, `ff top`, `ff wait/status` через control plane, persistent sessions) + что отвергнуто (peer-to-peer output-координация) + Herdr в prior-art |
| vibe-kanban: что брать (код/дизайн/уроки)? Почему он умер? Вердикт по executors-крейту? ACP-эмпирика? Merge-механика? Review→prompt? | `synthesis/06-vibe-kanban-patterns.md` | карта 14 подсистем → наши компоненты; брать-код (git/worktree, ACP-клиент, stdout_dup, лог-кластеризация — Apache-2.0); 14 дизайн-механик VK-D-1..14; 11 продуктовых уроков (умер без монетизации при 30k MAU — «координация без хранимого суждения не аккумулирует ценность»); 12 пробелов KB G-1..G-12 с адресами дописок |
| Какие Rust-либы используем? Чем отличаемся от стека VK? | `architecture/rust-stack.md` | tokio/axum/serde/tracing/git2 + command_group/os_pipe/json_patch/jsonc-parser; Postgres вместо SQLite; не наследовать форк ts-rs; не тащить relay-* |
| Решения владельца (Rust+Tauri, ACP-first)? | `decisions/D-001`, `decisions/D-002` | зафиксированные решения с VK-эмпирикой (bespoke:ACP ≈ 20:1) и flip-условиями |
| Каков план работ по фазам? Что делать первым? | `synthesis/00-master-synthesis.md` §5 | Phase 0–5 с DoD каждой фазы |
| Чего НЕ делать? | `synthesis/00-master-synthesis.md` §6, полнее `synthesis/01-consensus.md` §E | анти-паттерны |
| Какие артефакты ForgePlan авторить первыми? | `synthesis/00-master-synthesis.md` §5 Phase 0 | EPIC + PRD + ADR-001…005 + 2 RFC + prior-art EVID |

### Архитектура

| Вопрос | Файл(ы) | Что там |
|---|---|---|
| Какие плоскости/слои? Кто что владеет и что кому запрещено? | `architecture/planes.md` | 6 плоскостей, компоненты control plane, ExecutorDriver, маппинг имён слоёв на отчёты |
| Уровни агентов: кто что может, какие модели, какой поток? | `architecture/t0-t3-roles.md` | контракт T0–T3, risk-policy таблица, цикл T2↔T3, роль человека |
| Источники истины? Статусы задач? Leases? Reconcile? Fail-loop? | `architecture/state-and-truth.md` | 4 истины, ~10-статусная машина + 6 инвариантов, 2 контура lease, 6-source reconcile, verdict enum, чеклист закрытия, маппинг доски |
| Security: секреты, раннеры, права, privileged writes? | `architecture/security-trust.md` | pull_request_target, RCE boundary, write-таксономия, bot permissions, branch protection |
| Можно ли DeepSeek/Cerebras/OpenRouter в CC/Codex/OpenCode? Есть ли смысл? Model Gateway? | `architecture/model-routing.md` | факты по всем трём harness'ам, allowlist пар (support-first/gate-behind-eval/reject) × tiers, LiteLLM-gateway scope, конфиг-поверхность |
| Как запускаются сессии CC/Codex/OpenCode? Как оркестратор выбирает где кодить? | `architecture/executor-sessions.md` | per-process инъекция (env/флаги/inline config), resume, headless JSON-стримы, изоляция конфигов, version guards, новый harness = новый адаптер |
| Как система находит и пишет скиллы/суб-агентов? Безопасность чужих скиллов? | `architecture/skill-forge.md` | allowlisted-источники, trust state machine + G1–G4, authoring 8 стадий, хранение/dedup/decay, что не строить |
| UI: доски? движение задач live? метрики (задач/час)? layers-вид? worktrees? | `architecture/ui-observability.md` | Board-проекция, Control Room метрики, T0–T3 lanes, artifact-graph, worktree panel, `ff top`, фазовая карта поверхностей |
| Безотказность? Data-стек (статы/история/память)? K8s и оператор? | `architecture/reliability-and-k8s.md` | слоёный стек, идемпотентность/recovery/backup, K8s-ready дизайн-гарантии, Jobs per run, CRD-эскиз |
| Как ForgeFarm говорит с ForgePlan (CLI vs MCP)? | `synthesis/02-open-decisions.md` §5 | split: gates через CLI `--json`, workers через spawned `forgeplan serve` |
| Почему Rust, а не LangGraph/Temporal? | `synthesis/02-open-decisions.md` §1 | позиции, trade-off, flip-сигнал |
| Какая инфраструктура на MVP? | `synthesis/02-open-decisions.md` §2 | local-first compose; что взять из prodstack дёшево |
| Память: где что хранится, какой vector store? | `synthesis/02-open-decisions.md` §4 + `synthesis/01-consensus.md` A9 | ForgePlan-native + Hindsight; приоритет artifacts > policy > retrieval > hindsight |
| GitHub или Forgejo? Что зеркалится в issues? | `synthesis/02-open-decisions.md` §7 + `architecture/state-and-truth.md` §1 | GitHub-first за адаптером; mirror-семантика |

### Eval и routing (ядро vision)

| Вопрос | Файл(ы) | Что там |
|---|---|---|
| Как устроен eval-контур? Где живут кортежи? | `evals/eval-harness.md` | двухслойная схема: run rows (DB) → EvidencePacks (git) → routing; петля целиком |
| Как модель назначается на задачу? | `architecture/t0-t3-roles.md` (risk-policy) + `evals/eval-harness.md` (v1: calibrate→tier; далее eval-driven) | |
| Почему не LangSmith / отдельная eval-платформа? | `evals/eval-harness.md` «Отвергнуто» | vendor lock vs model-agnostic vision |
| Как EvidencePack/R_eff ложится на eval? | `evals/eval-harness.md` + `synthesis/03-wsfold-bridge.md` §6b | verdict/CL/valid_until маппинг; почему min() не для агрегации ранов |

### Связь с ForgePlan и WSFold-анализом

| Вопрос | Файл(ы) | Что там |
|---|---|---|
| Что менять в ForgePlan core ради ForgeFarm? | `synthesis/01-consensus.md` B4 (ноль) + `synthesis/03-wsfold-bridge.md` §2 (`--json` требование), §3 (dispatch hints) | |
| Что такое constellation и как ForgeFarm её потребляет? | `synthesis/03-wsfold-bridge.md` §2–2b | ingestion через `fpl --span`, store-qualified slugs, ghost stores, R_eff per-store |
| Worktree governance: почему never-auto-repair? | `synthesis/03-wsfold-bridge.md` §4–4b + `architecture/state-and-truth.md` §4 | три правила Governor, единый verdict enum |
| gastown/swarm-forge — брать или строить своё? | `synthesis/03-wsfold-bridge.md` §5 | «steal mechanics, reject as kernel»; что именно извлекать |
| Граница fpl vs ForgeFarm (что где живёт)? | `synthesis/03-wsfold-bridge.md` §1–1b | таблица «отвергнуто из fpl → дом в ForgeFarm → plane» |
| Что «дом» получили отвергнутые из fpl фичи? | `synthesis/03-wsfold-bridge.md` §1b | worktree provisioning→Governor, sandbox→runtime plane, trust→write-таксономия и т.д. |

### Первоисточники (только для сверки)

| Нужна деталь | Файл | Что искать |
|---|---|---|
| 6 недодуманных зон исходной идеи; 5 пробелов map-pack; 11 статусов; typed cross-system links | `research/R1-architecture-audit.md` | критика map-pack — лучшая в корпусе |
| ER-модель control plane (11 сущностей); write-таксономия; speculative branch lane; готовые playbook/hook/schema примеры; dual-CI трюк; 7 UI surfaces | `research/R2-production-stack.md` | NB: его стек (Temporal/K8s) отвергнут, но данные-модели и артефакты — ценные |
| 16 статусов + 12 событий; двухконтурные leases с числами; ExecutorDriver interface; 17-табличная схема; risk-policy.yaml; monorepo scaffold; стартовый artifact pack; risk register | `research/R3-rust-first-control-plane.md` | самый близкий к принятому дизайну; T0–T3 rename |
| forgeplan serve stdio ⇒ супервизор; Forgejo API cautions (Projects board, pull_request_target, dispatch endpoint); маппинг доски на session phases; literal-body/@file урок; version-skew правило | `research/R4-forgejo-plansform-integration.md` | самые конкретные ForgePlan-reuse мандаты |
| 4-truth доктрина; 7-полевая lease-запись; 6-полевая fail-запись; 6-source reconcile; чеклист evidence-close; нормализованная Mermaid-схема | `research/R5-sdd-scheme-normalization.md` | схемы записей field-level; NB: диаграммы-ассеты отсутствуют |
| Провенанс, дубликаты, citeturn-маркеры | `research/README.md` | |

## Ключевые слова → файл (для быстрого grep-роутинга)

`control plane, projection DB, scheduler, lease, claim, ExecutorDriver` → architecture/planes.md · state-and-truth.md
`T0 T1 T2 T3, tier, capability_class, risk-policy, human-on-exception, HAQ` → architecture/t0-t3-roles.md
`state machine, статусы, reconcile, drift, fail-loop, quarantine, evidence-first close, доска, kanban, labels` → architecture/state-and-truth.md
`pull_request_target, runner, RCE, secrets, branch protection, privileged write, single writer` → architecture/security-trust.md
`eval, кортеж, routing, model-routing.yaml, EvidencePack, verdict, congruence, valid_until, LangSmith` → evals/eval-harness.md
`constellation, store-qualified, slug, --span, --json, dispatch hints, ghost store` → synthesis/03-wsfold-bridge.md
`gastown, swarm-forge, prior art, build-vs-buy` → synthesis/03-wsfold-bridge.md §5
`map-pack, map.json, guardian, G1 G2 G3 G4, determinism, tombstones, scratch` → synthesis/02-open-decisions.md §8 + research/R1 (пробелы) + research/R3 (lane spec)
`Temporal, LangGraph, BullMQ, Deep Agents, Mastra, VoltAgent` → synthesis/02-open-decisions.md §1 + synthesis/01-consensus.md A12/E10
`Postgres, pgvector, Qdrant, Mem0, LightRAG, память, Hindsight` → synthesis/02-open-decisions.md §4
`submodule, co-located, export/import, monorepo` → synthesis/01-consensus.md A8/E4
`Forgejo, GitHub, webhook, issues, Projects board, workflow_dispatch` → synthesis/02-open-decisions.md §7 + architecture/security-trust.md
`Phase 0, Phase 1, Alpha, spine, roadmap, план, DoD` → synthesis/00-master-synthesis.md §5 (+ дельта в synthesis/04)
`autonomy, автономный режим, self-hosting, dogfooding, BMAD SDD RIPER routing-map, playbook, single-row rule, bundle, скиллы агенты подгрузка, CC Codex OpenCode` → synthesis/04-vision-alignment.md
`Herdr, мультиплексер, tmux, ff top, terminal cockpit, wait примитивы, зависший агент, blocked on approve, persistent session, PTY` → synthesis/05-herdr-patterns.md
`DeepSeek, Cerebras, OpenRouter, GLM, Kimi, MiniMax, Qwen, LiteLLM, gateway, ANTHROPIC_BASE_URL, wire_api, модель, пара harness model` → architecture/model-routing.md
`сессия, spawn, resume, headless, stream-json, codex exec, opencode run, CODEX_HOME, OPENCODE_CONFIG_CONTENT, permission-mode` → architecture/executor-sessions.md
`скилл, subagent, SKILL.md, marketplace, discovery, authoring, trust, quarantine, ToxicSkills, skill-creator, dedup` → architecture/skill-forge.md
`доска, board, kanban live, метрики, throughput, задач в час, lead time, Control Room, layers view, DAG-view, eval dashboard, audit explorer` → architecture/ui-observability.md
`безотказность, reliability, backup, recovery, идемпотентность, K8s, kubernetes, operator, CRD, Jobs, leader election` → architecture/reliability-and-k8s.md
`vibe-kanban, VK, BloopAI, executors crate, NormalizedEntry, MsgStore, squash merge, review comments, follow-up queue, sunsetting, ACP 20:1, can_use_tool, Stop-hook` → synthesis/06-vibe-kanban-patterns.md
`rust стек, крейты, tokio, axum, sqlx, ts-rs, petgraph, tauri, git2, command_group` → architecture/rust-stack.md + decisions/D-001
`Tauri, desktop, ACP, agent-client-protocol` → decisions/D-001, D-002

## Что где НЕ искать

- **Кода и API продукта здесь нет** — это pre-code база знаний.
- **Актуальный статус ForgePlan** (версии, PRD, PROB) — не здесь; см. `~/Work/ForgePlan/CLAUDE.md` и `.forgeplan/`.
- **Диаграммы из R5** — ассеты не выгружены (см. research/README.md); текстовая Mermaid-версия есть в самом R5.
- **Обоснование WSFold-решений для fpl core** (constellation дизайн, store-qualified IDs, отвергнутые маунты) — первоисточник в памяти сессии: `~/.claude/projects/-Users-explosovebit-Work-ForgePlan/memory/project_wsfold_evaluation.md`; здесь только следствия для ForgeFarm (`synthesis/03`).
