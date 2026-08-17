# UI и наблюдаемость: доски, движение задач, метрики, layers

> Полная карта операторских поверхностей и метрик ForgeFarm — что оператор
> видит на экране, как движутся задачи, какие цифры считаются. Сведение
> R1 (4 surfaces), R2 (7 surfaces), R3 (Control Room + 12-lane kanban + HAQ),
> R4 (derived board mapping) + Herdr-паттерны (05) — с фазовой приоритизацией,
> не ломающей решение «MVP = 2 поверхности» (развилка №6).

## 0. Принцип, который делает всё это возможным

**Каждая поверхность — проекция projection DB + audit stream. UI не имеет
собственного состояния и ничего не мутирует напрямую** (только API control
plane). Поэтому «доска», «движение карточек», «метрики» — это не отдельные
подсистемы, а разные рендеры одного event-потока: каждый переход state
machine пишет `audit_event` → SSE/WebSocket-стрим → карточка двигается на
экране в реальном времени. Live-движение достаётся бесплатно из инварианта
«каждый переход = audit_event».

## 1. Доска (Board) — да, и она живая

- **Колонки = проекция state machine** (маппинг R4): Backlog (нет lease) ·
  Ready (не заблокирован DAG) · Shaping / Coding / Evidence / Review-PR
  (session.phase) · Awaiting-Verifier · Human (HAQ) · Fail (формальная
  причина) · Done. Полный референс — 12 lanes R3.
- **Движение видно live:** карточка переезжает по событию перехода
  (SSE из audit stream), не по refresh. На карточке: task id, tier-бейдж
  (T0–T3), lease owner + TTL-таймер, linked артефакты, risk class, cost рана.
- **Swimlanes переключаемы:** по tier (T0–T3 — «интерфейс для layers»,
  см. §3), по репо/store (constellation), по методологической строке
  (BMAD/SPARC/RIPER/TDD…), по executor (CC/Codex/OpenCode).
- Доска **вычисляется, не редактируется**: drag&drop карточки — это не
  «перенос стикера», а команда control plane (например, approve), и только
  там, где переход легален.

## 2. Метрики (Control Room) — да, задачи в минуту/час и не только

Готовый список R3 + расширение. Всё считается из projection DB + audit
(события имеют timestamps — throughput выводится, ничего дополнительно
инструментировать не надо):

**Поток (то, о чём спрошено):**
- tasks started / completed **per hour** (и per day); runs per hour;
- queue depth per lane (сколько ждёт в Ready/Awaiting-Verifier/HAQ);
- **lead time**: issue → merged PR (медиана/p95) — главная DORA-подобная
  метрика фабрики; stage timings (сколько задача живёт в каждой фазе);
- throughput **per tier** (T0…T3 отдельно) и per executor.

**Здоровье:**
- active / idle агенты; lease expirations (зомби-раны); retries и fail-loop
  count по failure_class; drift incidents (5 контуров); HAQ depth
  (**bounded** — рост = красный флаг); evidence coverage; artifact health
  (blindspots/stale из `fpl health`).

**Деньги (и вход eval-контура):**
- cost by tier / model / provider (час/день/задача); tokens burn;
- cost per merged PR; budget envelope остатки per task;
- gate-pass rate per (model × task-class) — live-витрина eval-кортежа.

**Автономность:**
- % ранов без human-вмешательства; interventions per task;
- self-developed доля изменений (метрика self-hosting, Т-2).

## 3. Интерфейс для layers (T0–T3) — да

Два рендера одного и того же:
- **Lane-view:** swimlane на tier — видно, что сейчас думает T0, что
  проверяет T1, что кодит T2, что чинит T3; у каждой lane свой throughput
  и queue depth (узкое место фабрики видно сразу);
- **Pipeline-view задачи:** горизонтальная лента конкретной задачи
  T0 → T1 → T2 ⇄ T3 → merge с gate-вердиктами между стадиями (что прошло,
  что вернулось, где human). Это «как задачи передвигаются» на уровне
  одной задачи, дополняющее доску (уровень потока).

## 4. Связи с артефактами — да, это графовая поверхность

**Artifact & Task Graph** (R1 Artifact Explorer + R3): рендер typed links —
`Issue→planned_by→PRD/RFC/ADR`, `Run→reads→Artifact[]`,
`Run→produces→PR/evidence`, `VerifierRun→assesses→Run` — поверх
`fpl graph --json` (+ `--span` для constellation, store-qualified slugs).
С карточки задачи — клик в артефакт (тело, R_eff, evidence chain); с
артефакта — все задачи/раны, которые его читали/производили. Отличие от
`@forgeplan/web` viewer: **слияние артефактного графа с runtime-графом**
(это и есть шаг вперёд, названный в R2).

## 5. Worktrees и раны — да

- **Worktree panel** (вид Worktree Governor): живые worktrees × (ран, ветка,
  tier, lease, disk usage, three-state вердикт resolved/recoverable/refuse);
  quarantine-лист с причинами. Disk usage per worktree — обязателен
  (урок 20–72GB `target/`).
- **Run Inspector:** timeline типизированных RunEvents конкретного рана
  (tool calls, file writes, test results, gate requests), стоимость,
  верификаторский вердикт, ссылки на PR/артефакты/worktree. Трассы — из
  `tracing` spans (OTel-совместимо с Phase 1).

## 6. «Что ещё нужно» — определяю

| Поверхность | Что даёт | Фаза |
|---|---|---|
| **`ff top`** (терминал, из Herdr H-3) | state-bar всех ранов + HAQ по SSH/с телефона | **3** |
| **Human Attention Queue** | единственное место, где человек нужен: approve/reject с контекстом (diff, evidence, verdict) | **3** (CLI) → 4 (web) |
| **DAG-view** | граф зависимостей задач (`fpl order` + runtime blockedBy): критический путь, что разблокируется следующим | 4 |
| **Eval dashboard** | матрица (model × task-class): gate-pass, cost, interventions, n; активные/просроченные EVID; diff routing-таблицы | 4 (вместе с дистилляцией) |
| **Audit explorer** | поиск по hash-chained audit_events: «кто/что/почему перевёл задачу X»; replay истории задачи | 4 |
| **Fail Lab** | триаж fail-loop: кластеризация по failure_class, повторяемость | 5, flip-сигнал: HAQ стабильно >5–10 |
| **Governance Console** | policies, write-gates, allowlists, autonomy profile | 5 |
| **Memory Explorer** | что в episodic/retrieval памяти, что решило bundle | 5 |

## 7. Фазовая карта (не ломает развилку №6)

- **Phase 1 (spine):** никакого UI — `audit_events` читаемы через psql/CLI.
- **Phase 3:** `ff top` + `ff attention list/approve` (терминал). Это первый
  «экран», и он остаётся навсегда самым быстрым входом.
- **Phase 4 (web, ровно 2 поверхности):** **Board** (живая доска §1 со
  swimlanes §3) + **Run Inspector/HAQ** (§5 + approve). Метрики §2 — сначала
  строка сверху Board, не отдельный дашборд.
- **Phase 5 (по flip-сигналам):** DAG-view, Eval dashboard, Audit explorer,
  Fail Lab, Governance, Memory Explorer.

Технически (R2, когда дойдёт до web): SSE/WebSocket-стрим поверх projection
DB; стек не фиксируем до Phase 4 — но контракт «UI = чистая проекция +
команды через API» зафиксирован с Phase 1, поэтому web-слой добавляется без
переделки ядра.
