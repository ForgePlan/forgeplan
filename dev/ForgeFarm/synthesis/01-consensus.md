# Консенсус корпуса: инварианты и анти-паттерны

> В чём сходятся все (или почти все) пять отчётов `research/R1…R5`.
> Эти утверждения считаются **решёнными** — их не пересматривают без нового
> сильного evidence. Attribution: R1=audit, R2=prodstack, R3=rustfirst,
> R4=plansform, R5=sdd-scheme. Сила: 🟢 unanimous / 🟡 majority / ⚪ notable
> (одиночное, но важное и не оспоренное).

## A. Архитектурные инварианты

| # | Инвариант | Кто | Сила |
|---|---|---|---|
| A1 | ForgeFarm — отдельный orchestration/control plane НАД ForgePlan и форджем; никого не заменяет и не хранит собственной артефактной истины. Идентичность: «система контрактов вокруг агентов», не ещё один agent framework | R1–R5 | 🟢 |
| A2 | Истина разделена по плоскостям и не смешивается: артефакты = `.forgeplan/` markdown в git; планирование = issues форджа; исполнение = projection/run store ForgeFarm; evidence/наблюдаемость = commits/PRs + event/trace store. (Каждый отчёт называет это по-своему — 3-plane, 4-truths, 3-store — но контракт один) | R1–R5 | 🟢 |
| A3 | Kanban-доска и tracker-labels — всегда ПРОЕКЦИЯ runtime-состояния control plane, никогда не носитель. В трекер зеркалится только человекочитаемый summary, ссылки на evidence и PR | R1–R5 | 🟢 |
| A4 | Runtime-состояние (leases, claims, retries, budgets, checkpoints, verifier verdicts, approvals, heartbeats) требует выделенной операционной БД ForgeFarm — kanban-колонка/issue не выражает эти концепции. Базовый движок: Postgres (+pgvector) — конкретика от R1/R2/R3 | R1–R5 (Postgres: R1–R3) | 🟢 |
| A5 | Жизненный цикл task/run — формальная машиночитаемая state machine со словарём заметно богаче, чем backlog/doing/review/fail (R1: 11 статусов; R3: 16 статусов + 12 событий + инварианты). Переходы делает только control plane | R1–R5 | 🟢 |
| A6 | Параллельные кодящие агенты работают исключительно в изолированных git worktrees/sandbox — никогда в общем изменяемом workspace | R1–R5 | 🟢 |
| A7 | Lease/claim приобретаются ДО начала работы: task lease (TTL, owner, expiry policy) + scope/file claim (path-globs, ownership zones, worktree binding). Конфликтующие claims сериализуются или уводятся в отдельную лейн (R2: speculative branch lane; R3: двухконтурный lease TTL 10–30 мин / heartbeat 30–60 с; R5: 7-полевая lease-запись) | R1–R5 | 🟢 |
| A8 | `.forgeplan/` живёт co-located в репо с кодом (как задумано `forgeplan init -y`); git submodule для project-instance артефактов отвергнут — ломает атомарный бандл «code + artifact + evidence + PR». Submodule легитимен ТОЛЬКО для shared packs/harness/compliance-зеркал с независимым lifecycle; `forgeplan export/import` закрывает CI-воспроизводимость без submodule | R1–R4 (R5 молчит) | 🟢 |
| A9 | Память слоистая, никогда не один retrieval pool: operational/working (run checkpoints), episodic (ретроспективы), semantic/retrieval (индекс), artifact (ForgePlan — авторитетна). Приоритет R3: artifacts > policy > retrieval > hindsight; retrieval/hindsight никогда не перекрывают официальные артефакты | R1, R2, R3, R5 | 🟡 |
| A10 | Критичные emitted-артефакты (флагман — map.json): single-writer, atomic tmp-rename, детерминированный (не-LLM) guardian гейтит приём, advisory LLM только после deterministic pass, per-run scratch namespaces, машинные gates G1–G4. Enforcement fail-closed на трёх уровнях: hook + runtime policy + CI | R1, R2, R3 | 🟡 |
| A11 | RAG/vector-зрелость — лестница, не upfront-стройка: старт на ForgePlan local search (+pgvector при нужде); Qdrant/LightRAG — только при measured-масштабе; GraphRAG — только для org-wide вопросов. Не плодить сторы на старте | R1, R2, R3 | 🟡 |
| A12 | Ни один agent framework не является всей системой. Стек собирается по ответственности; явная внешняя state machine наверху; Deep Agents/Mastra/VoltAgent/LangChain — только nested worker/prototype/adapter lanes. «Ваша система управляет фреймворками, а не фреймворки — системой» | R1, R2, R3 | 🟡 |

## B. Роль ForgePlan

| # | Инвариант | Кто | Сила |
|---|---|---|---|
| B1 | ForgePlan остаётся artifact kernel и system of record для PRD/RFC/ADR/Spec/Evidence + typed link graph. НЕ превращать в оркестратор, трекер или dashboard backend — «wrap, don't replace». ForgeFarm строит только read models/projections поверх | R1–R5 | 🟢 |
| B2 | Всё взаимодействие с артефактами — исключительно через нативный контракт ForgePlan (CLI + MCP). Прямые записи в LanceDB или raw markdown запрещены — markdown = truth, LanceDB = derived (зеркалит ADR-003 и RED-LINE #11) | R1–R5 | 🟢 |
| B3 | Существующая quality-машинерия ForgePlan (`validate`, `health`, `drift`, `score`/R_eff, `review`, `activate`, evidence lifecycle) = gate set оркестратора: artifact readiness до кода, evidence + validate/health после, linked evidence до close/activation | R1–R5 | 🟢 |
| B4 | ForgeFarm требует по сути НУЛЬ изменений в ForgePlan — каждый отчёт потребляет существующие surfaces как есть; добавляются только конвенции/layout поверх (`.forgeplan/map/`, `projections/`). Сильный сигнал: fpl core функционально полон для нужд оркестратора | R1, R3, R4, R5 (R2: минимальный аддитивный layout) | 🟢 |
| B5 | Существующие примитивы ForgePlan переиспользуются, не переизобретаются: топологическая сортировка typed links = dependency ordering планировщика; `session`/guard phases = канонический lifecycle («phase oracle»); `calibrate` depth = вход model-routing; `activity-stats` = телеметрическая база; `export/import` = детерминированный CI seeding | R4 (сильнее всех), R1, R3 | 🟡 |
| B6 | `forgeplan serve` — stdio-only ⇒ оркестратор интегрируется как СУПЕРВИЗОР порождаемых worker-процессов (lifecycle, workspace mounts, изоляция), а не клиент сетевого HTTP-сервиса. Игнорирование гарантирует болезненный рефакторинг | R4 (центрально), R3 | ⚪ |

## C. Роль форджа (GitHub/Forgejo)

| # | Инвариант | Кто | Сила |
|---|---|---|---|
| C1 | Фордж = ingress/egress only: task ledger (issues/milestones/labels), PR/merge surface, permissions boundary, webhook/event source, CI host, evidence plane (commits/PRs/tests). Явно НЕ workflow engine, не scheduler, не транзакционный runtime store | R1–R5 | 🟢 |
| C2 | Ingest — webhook-first (обязательная валидация подписи, scoped tokens); polling — только reconciliation fallback. Обязателен reconcile-слой, сверяющий трекер с PR state, commit evidence, ForgePlan artifact state и runtime state — labels никогда не доверять (R5: 6 источников, «absolutely mandatory») | R1–R5 | 🟢 |
| C3 | Security-split вокруг untrusted кода: `pull_request_target` исполняется в write-контексте базовой ветки с доступом к секретам — опасен с fork-PR; untrusted код только в sandbox/worktrees; критичная write-автоматизация триггерится только от `issues` events или `workflow_dispatch`; merge только через verified gates оркестратора; секреты только в trusted branches/jobs; раннеры = RCE boundary (no host mode, no privileged containers by default) | R1, R3, R4 (R2: signed webhooks + scoped tokens) | 🟡 |
| C4 | Не строить автоматизацию на Forgejo Projects board API как primary state: Projects — kanban для людей, board-автоматизация under-documented и version-tied (проверять `/api/swagger` per instance); доска вычисляется в ForgeFarm как derived view | R4 (сильнее всех), R1, R2 | 🟡 |

## D. Процесс и методология

| # | Инвариант | Кто | Сила |
|---|---|---|---|
| D1 | Четырёхуровневая лестница агентов решена по ФОРМЕ (стратегия/декомпозиция → spec/verification → implementation в worktrees → validation/repair/evidence), НО формализуется как ROLE CONTRACT (разрешённые действия, артефакты, gates, SLA, цена ошибки, требуемые evidence) — никогда как «уровни мощности модели»; модель = параметр `capability_class`. R3 дополнительно: внутренний rename в T0–T3 (коллизия семантики L0–L3 с SDD-слайдами) | R1–R5 | 🟢 |
| D2 | Generator ≠ verifier обязателен: каждый кодящий ран получает независимую верификацию; merge/close доступен только после машиночитаемого вердикта; закрытие задач evidence-first (commit hash, PR, tests, linked artifact, verifier result) — никогда со слов агента | R1–R5 | 🟢 |
| D3 | Fail-closed gate отделяет планирование (T0/T1) от кода (T2/T3): сначала spec + readiness, потом код, потом независимая проверка. Никакой генерации кода до стабилизации артефактов (паттерн BMAD fail-closed gates + SPARC stage checks; guard/phase-машина ForgePlan нативно это enforce'ит) | R1–R5 | 🟢 |
| D4 | Fail-loop — state machine первого класса, не колонка доски: failure_class, retry_budget/count, repair_strategy, owner level, human_required, quarantine/reentry_condition; формальные причины (CI failed, gate refused, validation blocked, evidence decayed, lease expired, dependency blocked) управляют re-entry в scheduler | R1–R5 | 🟢 |
| D5 | Участие человека — policy-driven human-on-exception: high-risk approvals, architecture/security sign-off, policy overrides, повторные фейлы; явные категории auto/required/optional; bounded attention queue. Low-risk зелёный пайплайн идёт без per-diff human review (R3: «human запрещено микроменеджить») | R1–R5 | 🟢 |
| D6 | Каждый переход состояния и privileged action — в append-only audit log с evidence links (R3: hash-chained audit_events; R2: write-классификация read / safe-write / privileged-write с approval-or-guardian на privileged) | R1, R2, R3, R5 | 🟡 |
| D7 | Model/tier routing — depth/risk-driven: `calibrate` depth (Tactical/Standard/Deep/Critical) или risk-policy маппит риск-классы задач на allowed tiers, human requirements и обязательные gates/артефакты (Critical ⇒ полный artifact set + sign-offs). Это решённый скелет, в который вставляется eval-петля пользователя. NB: ни один отчёт не спроектировал сам eval-инструмент — общий пробел корпуса | R1, R2, R3, R4 | 🟡 |
| D8 | Rollout фазированный, contracts-before-intelligence: фаза 1 всегда фундамент/plumbing («no magic, all contracts»); один orchestration loop до полного swarm (R4: сначала только L0+L2); observability/evals — продуктовая фаза, не поздний add-on | R1–R4 | 🟢 |
| D9 | Issues — нормализованный task envelope для агентов (external ledger → fetch → normalize → reconcile → pick), а не парсинг чата ad hoc — названо главной сильной стороной исходной SDD-схемы | R5 (явно), R1/R2/R4 (implied) | 🟡 |
| D10 | Swarm-scale write-дисциплина ForgePlan: literal body strings в MCP (урок v0.32.1 `@file` silent-data-loss), pinned binary versions на runner-пулах, централизованные миграции (export → upgrade → migrate → health) | R4 (явно) | ⚪ |

## E. Анти-паттерны (запреты)

| # | Анти-паттерн | Почему | Кто |
|---|---|---|---|
| E1 | **Issues/labels/board как runtime database** | Гарантированные race conditions, drift четырёх «истин», потеря состояния, неидемпотентные переходы при параллельных агентах. Самое повторяемое предупреждение корпуса; R5: «дисквалифицирующая слабость» исходной схемы | 🟢 R1–R5 |
| E2 | **ForgePlan как оркестратор / второй artifact store внутри ForgeFarm** | Две конкурирующие истины; разрушает ADR-003. Вердикт всех: «wrap, don't replace» | 🟢 R1–R5 |
| E3 | **Прямые записи в LanceDB / raw markdown мимо CLI/MCP** | Рассинхрон derived-индекса с авторитетным trail; ломает audit/evidence chain. «ForgeFarm must not get clever» | 🟢 R1–R4 (R5 implicit) |
| E4 | **`.forgeplan/` как submodule в ежедневном flow** | Pinned-SHA pointer, раздвоенная история, сломанный атомарный PR-бандл, extra CI failure modes, недетерминированный setup для swarm | 🟢 R1–R4 |
| E5 | **Параллельные агенты без leases/claims** | git worktree сам не предотвращает конфликтные записи; без claim-резервации выходит merge-хаос. R3: lease-семантика — «самое недооценённое место самодельных оркестраторов» (echo PROB-060) | 🟢 R1–R5 |
| E6 | **Закрытие задач по self-report агента; merge без машинного вердикта** | «Готово» от агента недостоверно; без generator≠verifier система скатывается в непроверенные merges | 🟢 R1–R5 |
| E7 | **Код до стабилизации артефактов** | Ровно тот провал, от которого существуют BMAD/SPARC gates | 🟢 R1–R5 |
| E8 | **«Просто подключить память» — один retrieval pool** | Агент либо забывает важное, либо тащит мусор; hindsight начинает перекрывать официальные артефакты | 🟡 R1–R3 |
| E9 | **Несколько writers у критичных emitted-артефактов** | Ломает детерминизм, byte-identical reruns и trust-модель guardian-пайплайна. R2: главная слабость — соблазн «оркестратор тоже чуть подправит map.json» | 🟡 R1–R3 |
| E10 | **Ставка ядра на один «магический» framework** | Ни один framework не даёт domain model для leases/worktrees/git governance/policy gates; lock-in инвертирует управление. R3: LangChain-only — explicit «do not choose»; VoltAgent — preview-grade; Deep Agents — «nested primitive, not the brain» | 🟡 R1–R3 |
| E11 | **Untrusted код с write-токенами/секретами** | `pull_request_target` misuse; runners = RCE by design | 🟡 R1, R3, R4 |
| E12 | **Tiers = мощность модели; двусмысленный L0–L3 в внутренних enum'ах** | Power-tiers без role contract быстро создают хаос; L0–L3 несёт семантическую коллизию (SDD: L0=executors vs у пользователя L0=сильнейшие модели) | 🟡 R2, R3 |
| E13 | **Автоматизация на Forgejo Projects board API как primary state** | Under-documented, version-tied; доска вычисляется, не опрашивается | 🟡 R4, R1, R2 |
| E14 | **Полный swarm сразу; observability/evals «потом»** | «Красивая, но ненадёжная оболочка»: видно анимацию агентов, нельзя управлять качеством | 🟡 R4, R1, R2 |
| E15 | **Hand-rolling durable-workflow забот на голых очередях/cron** | Retry/pause/approval — norm case, не edge; им место в формальной state machine или Temporal (по flip-сигналу) | ⚪ R1, R2 |
