# Unified Workflow: Forgeplan × Orchestra × Claude Code

> Три системы как единый организм. Каждая делает то, что умеет лучше всех.
> Данные живут в одном месте, ссылки — везде.

---

## Оглавление

1. [Тезис и обоснование](#1-тезис-и-обоснование)
2. [Три bounded contexts](#2-три-bounded-contexts)
3. [Custom Fields (единые для всех конфигураций)](#3-custom-fields)
4. [Status ↔ Phase маппинг](#4-status--phase-маппинг)
5. [Конфигурации](#5-конфигурации)
   - [Config A: Solo Dev + AI](#config-a-solo-dev--ai-agents)
   - [Config B: Small Team (2-5)](#config-b-small-team-2-5)
   - [Config C: Medium Team (5-15)](#config-c-medium-team-5-15)
6. [Greenfield Setup](#6-greenfield-setup)
7. [Brownfield Migration](#7-brownfield-migration)
8. [Migration между конфигурациями](#8-migration-между-конфигурациями)
9. [Session Start Protocol](#9-session-start-protocol)
10. [Lifecycle задачи](#10-lifecycle-задачи)
11. [Инструкции по ролям](#11-инструкции-по-ролям)
12. [Риски и митигации](#12-риски-и-митигации)
13. [Узкие места (bottlenecks)](#13-узкие-места)
14. [Anti-patterns](#14-anti-patterns)
15. [Quick Reference](#15-quick-reference)
16. [Playbook: сценарии ежедневной работы](#16-playbook-сценарии-ежедневной-работы)
17. [Чего НЕЛЬЗЯ делать (запреты)](#17-чего-нельзя-делать-запреты)
18. [Inbox Pattern: сбор и triage сигналов](#18-inbox-pattern-сбор-и-triage-сигналов)

---

## 1. Тезис и обоснование

### Проблема

Три инструмента работают изолированно:
- **Forgeplan** знает про артефакты и качество, но не трекает кто и когда делает
- **Orchestra** знает про задачи и людей, но не знает про методологию
- **Claude Code** исполняет код, но каждый чат начинает с нуля

Результат: двойная работа, потеря контекста, рассинхронизация.

### Решение

Три **bounded context** (FPF A.1.1) с чётким разделением ответственности и минимальным набором hand-off точек. Каждая система делает то, что умеет лучше всех, и не лезет на чужую территорию.

### Почему именно так (research)

**FPF A.1.1 U.BoundedContext**: "Make meaning local; make translation explicit." Каждая система — это semantic locale с собственным словарём. "Status" в Orchestra и "lifecycle" в Forgeplan — это РАЗНЫЕ понятия, даже если маппятся друг на друга. Нельзя смешивать.

**FPF A.7 Strict Distinction**: method ≠ work ≠ role. Forgeplan = method (КАК думать о работе). Orchestra = work (ЧТО делается КЕМ). Claude Code = role (КТО исполняет). Смешивание этих категорий — главный источник хаоса.

**FPF B.3 Trust Calculus**: Custom fields в Orchestra = low-trust proxy. Они показывают *ссылку* на артефакт, но за quality scoring отвечает Forgeplan. Не дублируем данные — доверяем каждой системе своё.

**FPF A.14 Mereology**: У Forge есть ДВЕ ортогональные оси — artifact hierarchy (Epic→PRD→RFC) и execution flow (Sprint→Wave→Task). Orchestra отражает *execution*, не дублирует artifact hierarchy.

### Ключевые принципы

1. **Single source of truth** — данные живут в одном месте, ссылки везде
2. **Fields на workspace-level** — переживут любой рефакторинг проектов
3. **Минимум дублирования** — не копируем то, что можно запросить
4. **Graceful degradation** — если Orchestra недоступна, Forgeplan работает автономно
5. **Progressive enhancement** — начинаем с Config A, растём по мере надобности

---

## 2. Три bounded contexts

| Система | Владеет | НЕ лезет в | Source of Truth для |
|---------|---------|------------|---------------------|
| **Forgeplan** | Артефакты, валидация, R_eff, evidence, lifecycle, depth, quality gates | Task tracking, assignees, due dates, коммуникация | Что делать, зачем, какого качества |
| **Orchestra** | Задачи, статусы, assignees, due dates, чек-листы, сообщения, проекты | Валидация артефактов, R_eff scoring, evidence chain | Кто делает, когда, в каком статусе |
| **Claude Code** | Skills, hooks, plugins, memory, agents, git workflow | Хранение данных (делегирует BC1 и BC2) | Как делать, контекст между сессиями |

### Что НЕ дублируем в Orchestra

| Данные | Живёт в | Почему не дублируем |
|--------|---------|---------------------|
| Содержимое артефакта | Forgeplan (LanceDB + .md) | Orchestra ≠ документооборот |
| R_eff score | Forgeplan | Вычисляемое, устаревает мгновенно |
| Validation results | Forgeplan | Динамическое |
| Evidence chain | Forgeplan (links) | Граф зависимостей в Forgeplan |
| Artifact body/sections | Forgeplan markdown | Структурированный контент |
| Git history | Git | `git log` / `git blame` авторитетны |

### Что ЖИВЁТ в Orchestra

| Данные | Зачем |
|--------|-------|
| Task name + Artifact ID field | Маппинг и быстрый поиск |
| Status (Backlog→Done) | Кто и когда |
| Phase (Shape→Done) | Где в pipeline |
| Sprint | Группировка по спринтам |
| Branch | Связка с git |
| Assignee | Кто ответственный |
| Due date | Дедлайны |
| Checklists | FR items для трекинга progress |
| Messages | Коммуникация в контексте задачи |

---

## 3. Custom Fields

**КРИТИЧНО**: Custom fields создаются на **workspace-level**. Это означает что они доступны в ЛЮБОМ проекте внутри workspace и переживут любой рефакторинг структуры проектов (migration A→B→C).

| Field | Тип | Значения | Описание | Обязательность |
|-------|-----|----------|----------|----------------|
| **Artifact** | `text` | `PRD-021`, `RFC-003`, `PROB-021` | ID артефакта в Forgeplan | Обязательно для artifact-linked tasks |
| **Type** | `option` | PRD / RFC / ADR / Epic / Spec / Problem / Evidence / Note | Тип артефакта | Обязательно если есть Artifact |
| **Depth** | `option` | Tactical / Standard / Deep / Critical | Глубина проработки из `forgeplan route` | Опционально |
| **Phase** | `option` | Shape / Validate / Code / Evidence / Done | Текущая фаза Forge pipeline | Рекомендуется |
| **Sprint** | `text` | `Sprint 9`, `Sprint 10` | Привязка к спринту | Рекомендуется |
| **Branch** | `text` | `fix/adi-quality-prob021` | Git branch | Опционально |

### Почему эти 6 и не больше

- **Artifact** — ключевая связка, без него нет маппинга
- **Type** — фильтрация "покажи все PRD" без чтения Forgeplan
- **Depth** — PM видит сложность без вникания в артефакт
- **Phase** — AI agent понимает где в pipeline без лишних запросов
- **Sprint** — группировка по времени, работает в любой конфигурации
- **Branch** — связка с git, AI может найти код по задаче

**НЕ добавляем**: R_eff (вычисляемое, мгновенно устаревает), Priority (уже есть стандартное поле), Tags (уже есть стандартное поле), Description/Body (это содержимое артефакта, живёт в Forgeplan).

---

## 4. Status ↔ Phase маппинг

Два поля отражают разные аспекты одной работы:
- **Status** — Orchestra native, видимый всем, про "состояние задачи"
- **Phase** — Forge pipeline, про "где в методологическом цикле"

| Orchestra Status | Forge Phase | Что происходит | Кто обновляет |
|-----------------|-------------|----------------|---------------|
| **Backlog** | Shape | Артефакт создан, секции заполняются | Создатель задачи |
| **To Do** | Validate | Артефакт validated (PASS), готов к работе | AI после `forgeplan validate` |
| **Doing** | Code | Код пишется, спринт в процессе | Разработчик или AI |
| **Review** | Evidence | Audit завершён, evidence создаётся | AI после `/audit` |
| **Done** | Done | Артефакт activated в Forgeplan | AI после `forgeplan activate` |

### Правило синхронизации

Если один обновлён — второй должен быть обновлён тоже. AI agent при обновлении Status автоматически обновляет Phase, и наоборот. При конфликте — Status побеждает (Orchestra = source of truth для execution state).

---

## 5. Конфигурации

### Как выбрать конфигурацию

```
Сколько людей работают над проектом?
│
├── 1 человек (+ AI agents) ──────────→ CONFIG A: Solo Dev
│
├── 2-5 человек ──────────────────────→ CONFIG B: Small Team
│   └── Есть ли чёткие области?
│       ├── Да (backend/frontend/...) → Config B с area projects
│       └── Нет (все fullstack) ──────→ Config B с одним project
│
└── 5-15 человек ─────────────────────→ CONFIG C: Medium Team
    └── Есть ли параллельные спринты?
        ├── Да (разные области/команды) → Config C полный
        └── Нет (один sprint для всех) → Config B достаточно
```

---

### Config A: Solo Dev + AI Agents

**Для кого**: один разработчик с AI агентами. Самый частый случай для Forgeplan.

#### Структура

```
Workspace: ForgePlan
└── Project: "Development"
    ├── [PRD-021] ADI Quality           Doing / Code      Sprint 9
    ├── [PROB-021] ADI prompt bugs      Review / Evidence  Sprint 9
    ├── [RFC-005] New routing           Backlog / Shape    Sprint 10
    ├── Desktop App research            Backlog / Shape    —
    └── ...
```

#### Характеристики

| Параметр | Значение |
|----------|----------|
| Проектов | 1 ("Development") |
| Max задач | ~50 комфортно, ~100 с Views |
| Assignee | Не нужен (всё = я) |
| Sprint tracking | Field "Sprint" на задаче |
| Views | Current Sprint, In Progress, By Type |
| Daily overhead | ~0 минут (AI делает Session Start) |
| Setup time | 15 минут |

#### Когда использовать

- Личный проект или pet project
- Solo разработка с AI-assisted workflow
- Начало нового проекта (greenfield) до привлечения команды
- Прототипирование и MVP фаза

#### Workflow

```
Утро:
  /briefing → что в работе, unread
  forgeplan health → blind spots

Работа:
  forgeplan route "задача" → depth
  forgeplan new prd "Title" → артефакт
  → Orch: создать задачу с fields
  /sprint или /wave → реализация
  → Orch: Status=Doing
  /audit → review
  → Orch: Status=Review
  forgeplan activate → done
  → Orch: Status=Done

Конец дня:
  Проверить /orch status
```

#### Saved Views

| View | Фильтр |
|------|--------|
| Current Sprint | Sprint = "Sprint N" AND Status ≠ Done |
| In Progress | Status = Doing OR Review |
| All PRDs | Type = PRD |
| Problems | Type = Problem |

---

### Config B: Small Team (2-5)

**Для кого**: небольшая команда разработчиков, возможно с PM. Каждый может работать над своей областью.

#### Структура

```
Workspace: ForgePlan
├── Project: "Core Platform"          ← backend, core crate, CLI
│   ├── [PRD-021] ADI Quality         @alice  Doing    Sprint 9
│   ├── [RFC-005] New routing         @bob    To Do    Sprint 9
│   └── [PROB-022] Parser edge case   @alice  Backlog  Sprint 10
│
├── Project: "Desktop App"            ← Tauri, React, UI
│   ├── [PRD-025] Desktop MVP         @carol  Doing    Sprint 9
│   └── [SPEC-001] UI Components      @carol  Backlog  Sprint 10
│
├── Project: "Backlog"                ← неразобранное, triage
│   ├── [PROB-023] Search ranking     —       Backlog  —
│   └── New feature idea              —       Backlog  —
│
└── Project: "Operations"             ← CI, infra, releases, non-artifact
    ├── Release v0.8.0 prep           @bob    To Do    Sprint 9
    └── CI pipeline optimization      —       Backlog  —
```

#### Характеристики

| Параметр | Значение |
|----------|----------|
| Проектов | 3-5 (по areas + Backlog + Operations) |
| Max задач | ~100 total, ~30 per project |
| Assignee | Обязателен — кто ответственный |
| Sprint tracking | Field "Sprint" (единый спринт для всех) |
| Views | Per-project defaults + workspace views |
| Daily overhead | ~5 минут (briefing + status check) |
| Setup time | 30 минут |

#### Когда использовать

- Команда 2-5 человек
- Есть чёткое разделение по областям (backend/frontend/infra)
- Один sprint cycle для всей команды
- PM нужна видимость по областям

#### Что меняется vs Config A

| Аспект | Config A | Config B |
|--------|----------|----------|
| Проекты | 1 | 3-5 по areas |
| Assignee | Не нужен | Обязателен |
| Backlog | В том же проекте | Отдельный проект |
| Operations | Нет | Отдельный проект |
| Task creation | Всё в "Development" | Нужно выбрать project |
| Cross-area work | N/A | Parent task + subtasks |

#### Правила работы с areas

1. **Задача принадлежит area** где основная работа. Если PRD требует и backend и frontend — основная task в "Core Platform", subtask в "Desktop App"
2. **Backlog** — задачи без sprint и без assignee. Triage = переместить в нужный project + назначить sprint
3. **Operations** — всё что не связано с артефактами: CI, releases, infra, docs
4. **Cross-area dependencies** — использовать Orchestra Relations (related entities) + `forgeplan blocked`

#### Routing правило для AI агента

```
При создании задачи:
  IF Type = PRD|RFC|ADR|Problem AND scope содержит "cli"|"core"|"backend"
    → Project: "Core Platform"
  ELIF Type = PRD|Spec AND scope содержит "ui"|"desktop"|"react"|"tauri"
    → Project: "Desktop App"
  ELIF Type = None (operational task)
    → Project: "Operations"
  ELSE
    → Project: "Backlog" (triage later)
```

#### Saved Views

| View | Scope | Фильтр |
|------|-------|--------|
| My Tasks | Workspace | Assignee = me AND Status ≠ Done |
| Current Sprint | Workspace | Sprint = "Sprint N" AND Status ≠ Done |
| All In Progress | Workspace | Status = Doing OR Review |
| Overdue | Workspace | Due date < today AND Status ≠ Done |
| Needs Triage | Backlog project | Assignee = none |

---

### Config C: Medium Team (5-15)

**Для кого**: команда с ролями (PM, Dev, QA, Designer). Параллельные sprint scopes по областям.

#### Структура

```
Workspace: ForgePlan
├── Project: "Core Platform"                    ← area
│   ├── Sub-project: "Core Sprint 10"           ← sprint scope
│   │   ├── [PRD-021] ADI Quality       @alice  Doing
│   │   ├── [RFC-005] Routing v2        @bob    To Do
│   │   └── [QA] Regression tests       @dave   Backlog
│   ├── Sub-project: "Core Sprint 11"           ← planning
│   │   └── (planning items)
│   └── Sub-project: "Core Backlog"             ← area backlog
│       └── [PROB-023] Search ranking   —       Backlog
│
├── Project: "Desktop App"                      ← area
│   ├── Sub-project: "Desktop Sprint 10"
│   │   └── [PRD-025] Desktop MVP      @carol  Doing
│   └── Sub-project: "Desktop Backlog"
│
├── Project: "Operations"                       ← cross-area
│   ├── Release v0.8.0 coordination     @pm     Doing
│   └── CI pipeline optimization        @eve    To Do
│
├── Channel: "Engineering"                      ← team-wide comms
├── Channel: "Standup"                          ← daily updates
└── Document: "Sprint 10 Goals"                 ← shared context
```

#### Характеристики

| Параметр | Значение |
|----------|----------|
| Проектов | 3-5 areas × sub-projects |
| Max задач | ~300 total |
| Assignee | Обязателен |
| Sprint tracking | **Sub-project** per sprint per area (не field!) |
| Views | Per-role views |
| Daily overhead | ~15 минут (standup + status + triage) |
| Setup time | 1-2 часа |
| Max nesting | 3 уровня (workspace → project → sub-project) — это ЛИМИТ Orchestra |

#### Когда использовать

- Команда 5-15 человек с разными ролями
- Параллельные sprint scopes (Core и Desktop работают независимо)
- PM нужна видимость cross-area
- QA involvement (Review status = QA queue)

#### Что меняется vs Config B

| Аспект | Config B | Config C |
|--------|----------|----------|
| Sprint tracking | Field | Sub-project |
| Sprint transition | Change field value | Create new sub-project |
| Parallel sprints | Shared sprint | Per-area sprints |
| Communication | Task messages | + Channels |
| Shared documents | N/A | Orchestra Documents |
| QA workflow | Review status | Review = QA queue |
| Nesting | Workspace → Project | Workspace → Project → Sub-project (MAX!) |

#### Роли и Views

| Роль | Что видит | Primary View |
|------|-----------|-------------|
| **Developer** | Свои задачи в текущем sprint | My Tasks + Current Sprint |
| **PM** | Все задачи по всем areas | Cross-area Sprint Overview |
| **QA** | Задачи в Review | Review Queue |
| **Designer** | Spec/PRD задачи | Type = Spec OR PRD |
| **Tech Lead** | Architecture tasks | Type = RFC OR ADR |

#### Sprint transition

```
Конец Sprint 10:
1. Создать "Core Sprint 11" sub-project
2. Незавершённые задачи → move_entity в "Core Sprint 11"
3. Новые задачи из "Core Backlog" → move в "Core Sprint 11"
4. "Core Sprint 10" sub-project → archive (не удалять!)

ВАЖНО: В Config C sprint = sub-project, НЕ field.
Не использовать Sprint field и sub-project одновременно (§14 Anti-patterns).
```

#### Ограничения

- **3 уровня nesting — МАКСИМУМ Orchestra**. Нельзя добавить ещё один уровень. Если нужно больше — использовать parent-child tasks внутри sub-project
- **Sprint sub-projects множатся** — за год 24+ sub-projects на area. Митигация: архивировать завершённые (showArchived=false по умолчанию)
- **Cross-area task** — живёт в одном sub-project, subtask-ссылка в другом. Relations для видимости

---

## 6. Greenfield Setup

Начинаешь проект с нуля. Нет артефактов, нет задач, чистый workspace.

### Шаг 1: Определи конфигурацию

```
Вопрос: сколько людей будут работать?
├── 1 → Config A
├── 2-5 → Config B
└── 5+ → Config C
```

### Шаг 2: Setup workspace

#### Config A: Greenfield

```bash
# 1. Workspace уже есть (или создать в Orchestra UI)

# 2. Создать custom fields (workspace-level)
# AI выполняет через MCP:
manage_field: create Artifact (text)
manage_field: create Type (option) → PRD, RFC, ADR, Epic, Spec, Problem, Evidence, Note
manage_field: create Depth (option) → Tactical, Standard, Deep, Critical
manage_field: create Phase (option) → Shape, Validate, Code, Evidence, Done
manage_field: create Sprint (text)
manage_field: create Branch (text)

# 3. Создать проект
create_entity: Project "Development"

# 4. Инициализировать Forgeplan
forgeplan init -y
forgeplan health

# 5. Создать первый артефакт + задачу
forgeplan route "описание проекта"
forgeplan new epic "Project Name"
→ Orch: create task "[EPIC-001] Project Name"
  Fields: Artifact=EPIC-001, Type=Epic, Phase=Shape
```

#### Config B: Greenfield

```bash
# 1-2. Те же custom fields (workspace-level)

# 3. Создать проекты по areas
create_entity: Project "Backend"
create_entity: Project "Frontend"
create_entity: Project "Backlog"
create_entity: Project "Operations"

# 4. Forgeplan init + health

# 5. Создать Epic + PRDs по areas
forgeplan new epic "Project Name"
forgeplan new prd "Backend API"
forgeplan new prd "Frontend UI"
→ Orch: tasks в соответствующих projects
```

#### Config C: Greenfield

```bash
# 1-2. Custom fields

# 3. Проекты + sub-projects
create_entity: Project "Backend"
  create_entity: Sub-project "Backend Sprint 1" (contextUid=Backend)
  create_entity: Sub-project "Backend Backlog" (contextUid=Backend)
create_entity: Project "Frontend"
  create_entity: Sub-project "Frontend Sprint 1"
  create_entity: Sub-project "Frontend Backlog"
create_entity: Project "Operations"
create_entity: Channel "Engineering"
create_entity: Channel "Standup"

# 4. Forgeplan init
# 5. Epic + area PRDs + tasks
```

### Шаг 3: Первый sprint

```
1. forgeplan route каждой задачи → определить depth
2. Создать артефакты (PRD, RFC по depth)
3. Создать задачи в Orchestra с fields
4. Назначить Sprint = "Sprint 1"
5. /sprint для начала работы
```

### Шаг 4: Настроить AI окружение

```
1. Проверить CLAUDE.md содержит Session Start Protocol
2. Проверить memory содержит unified workflow architecture
3. Проверить /sync-tasks работает с новым workspace
4. Первый /briefing → убедиться что видит задачи
```

---

## 7. Brownfield Migration

У тебя уже есть проект с артефактами в Forgeplan, но Orchestra пустая или используется по-другому.

### Сценарий 1: Forgeplan есть, Orchestra пустая

```bash
# 1. Setup custom fields (как в Greenfield)

# 2. Создать проект(ы) по конфигурации

# 3. Backfill: создать задачи для существующих артефактов
forgeplan list --status active    # → список active артефактов
forgeplan list --status draft     # → список draft артефактов

# Для каждого active артефакта:
→ Orch: create task "[ID] Title"
  Fields: Artifact=ID, Type=kind, Phase=Done, Status=Done

# Для каждого draft артефакта:
→ Orch: create task "[ID] Title"
  Fields: Artifact=ID, Type=kind, Phase=текущая, Status=текущий

# 4. Verify
/orch status → должен показать все артефакты
forgeplan health → сравнить с Orchestra
```

### Сценарий 2: Orchestra есть с задачами, Forgeplan есть с артефактами

```bash
# 1. Добавить custom fields к workspace

# 2. Для существующих задач — добавить Artifact field если есть маппинг
# Ручной процесс: найти соответствия task ↔ artifact

# 3. Для артефактов без задач — создать задачи

# 4. Для задач без артефактов — оценить нужен ли артефакт
#    forgeplan route "описание задачи"
#    Если Tactical → не нужен, оставить как есть
#    Если Standard+ → создать артефакт, привязать
```

### Сценарий 3: Переход с другого task tracker

```bash
# 1. Export задач из старого tracker (CSV/JSON)
# 2. Setup Orchestra (fields + projects)
# 3. Import через create_entity batch
# 4. Map artifact IDs где есть
# 5. Forgeplan остаётся как есть (source of truth для артефактов)
```

---

## 8. Migration между конфигурациями

### A → B: Solo → Small Team

**Когда**: к проекту присоединяется 2-й человек.

**Что делать**:
```
1. Custom fields уже на workspace-level → ничего не меняем
2. Переименовать "Development" → "Core" (или другое area name)
3. Создать дополнительные проекты по areas
4. Создать "Backlog" и "Operations"
5. Распределить задачи по проектам (move_entity)
6. Начать использовать Assignee field
7. Обновить /sync-tasks routing rules
```

**Effort**: 30 минут. **Risk**: Low — fields сохраняются, задачи перемещаются.

### B → C: Small Team → Medium Team

**Когда**: команда растёт до 5+, нужны параллельные sprint scopes.

**Что делать**:
```
1. Custom fields — без изменений
2. В каждом area-project создать sub-projects для спринтов:
   "Backend" → "Backend Sprint N", "Backend Backlog"
3. Переместить задачи из project root в sub-projects
4. Создать Channels для коммуникации
5. Настроить Views per role
6. Sprint field → опционально (sub-project = sprint)
```

**Effort**: 1-2 часа. **Risk**: Medium — нужно перемещать задачи, может потеряться history.

### C → B: Downsize (команда уменьшилась)

**Когда**: команда уменьшилась, overhead Config C не оправдан.

**Что делать**:
```
1. Объединить sub-projects в project root
2. Удалить пустые sub-projects
3. Вернуться к Sprint field вместо sub-projects
4. Channels можно оставить или архивировать
```

**Effort**: 30 минут. **Risk**: Low.

### B → A: Обратно в Solo

```
1. Объединить все area projects в один "Development"
2. Удалить пустые projects
3. Убрать Assignee (всё = я)
```

---

## 9. Session Start Protocol

**ОБЯЗАТЕЛЬНО при каждом новом чате Claude Code.**

```
STEP 1: CONTEXT RESTORE
├── CLAUDE.md загружается автоматически
├── memory_recall("Forgeplan") — Hindsight
└── Auto-memory (MEMORY.md) — загружается автоматически

STEP 2: PROJECT HEALTH (параллельно)
├── forgeplan health
│   → blind spots (active без evidence)
│   → orphans (без связей)
│   → stale artifacts
│
└── Orchestra query (active tasks)
    → что в Doing / Review
    → overdue tasks
    → unread messages

STEP 3: SYNTHESIS
"Сейчас в работе:
  • [PRD-021] ADI Quality — Doing, Phase: Code, Sprint 9
  • [PROB-021] prompt bugs — Review, Phase: Evidence
Health: 2 blind spots (RFC-003, ADR-005 без evidence)
Overdue: нет
Следующее: завершить PRD-021 → evidence → activate"

STEP 4: RECOMMEND
Конкретный next action по методологии:
  → Если есть blind spots: "Fix blind spots first"
  → Если есть Doing tasks: "Continue [task]"
  → Если всё Done: "Start next sprint task"
```

### Когда НЕ выполнять полный протокол

- Короткий вопрос ("как работает X?") → достаточно CLAUDE.md
- Continuation явного чата ("продолжи где остановился") → context уже есть
- Отладка бага → сразу в код, протокол после

---

## 10. Lifecycle задачи

### От идеи до Done

```
┌─────────┐     ┌──────────┐     ┌────────┐     ┌──────────┐     ┌──────┐
│ ROUTE   │────▶│  SHAPE   │────▶│  CODE  │────▶│ EVIDENCE │────▶│ DONE │
│         │     │          │     │        │     │          │     │      │
│route    │     │new + fill│     │sprint/ │     │audit +   │     │activ-│
│"задача" │     │validate  │     │wave    │     │evidence  │     │ate   │
│         │     │          │     │        │     │          │     │      │
│Orch:    │     │Orch:     │     │Orch:   │     │Orch:     │     │Orch: │
│—        │     │Backlog→  │     │Doing   │     │Review    │     │Done  │
│         │     │To Do     │     │        │     │          │     │      │
└─────────┘     └──────────┘     └────────┘     └──────────┘     └──────┘
```

### Детальные шаги

```
1. ROUTE
   forgeplan route "описание задачи"
   → Depth: Standard, Pipeline: PRD → RFC
   → Orchestra: ничего пока

2. CREATE (Forgeplan + Orchestra)
   forgeplan new prd "Title"           → PRD-XXX создан
   Orch: create task "[PRD-XXX] Title"
   Orch: set fields: Artifact=PRD-XXX, Type=PRD, Depth=Standard
   Orch: set Phase=Shape, Status=Backlog

3. SHAPE
   Заполнить MUST секции (Problem, Goals, FR, Non-Goals, Related)
   forgeplan validate PRD-XXX          → PASS
   Orch: Phase=Validate, Status=To Do

4. CODE
   /sprint или /wave для реализации
   Orch: Phase=Code, Status=Doing
   Orch: Branch=feat/xxx
   Orch: добавить Checklist с FR items из PRD

5. AUDIT + EVIDENCE
   /audit → 5-agent review
   forgeplan new evidence "..."        → EVID-XXX
   forgeplan link EVID-XXX PRD-XXX --relation informs
   Orch: Phase=Evidence, Status=Review

6. ACTIVATE
   forgeplan review PRD-XXX            → review PASSED
   forgeplan activate PRD-XXX          → draft → active
   Orch: Phase=Done, Status=Done

7. COMMIT + PR (если ещё не сделано)
   git commit + git push + gh pr create
   Orch: Branch field обновлён
```

### Tactical tasks (без артефакта)

```
forgeplan route "fix typo" → Tactical
→ Просто создать задачу в Orchestra БЕЗ Artifact field
→ Status: To Do → Doing → Done
→ No validate, no evidence, no activate
```

---

## 11. Инструкции по ролям

### Для разработчика (Human)

| Когда | Что делать |
|-------|-----------|
| Утро | `/briefing` → что в работе, overdue, unread |
| Перед задачей | `forgeplan route "описание"` → depth |
| Создание | `forgeplan new ...` + задача в Orch с fields |
| Работа | Двигай Status в Orchestra по мере прогресса |
| Код | `/sprint` или `/wave` для AI-assisted dev |
| Финиш | `forgeplan activate` + Orch: Status=Done |
| Конец дня | `/orch status` → всё ли актуально |

### Для PM / Tech Lead

| Когда | Что делать |
|-------|-----------|
| Обзор | `/orch projects` + `forgeplan health` — полная картина |
| Планирование | Создавай задачи с Artifact, Sprint, Priority fields |
| Приоритеты | Priority + Sprint fields в Orchestra |
| Качество | `forgeplan validate` + `forgeplan score` для R_eff |
| Коммуникация | `/orch msg` для обсуждений в контексте задачи |
| Sprint planning | Создать задачи для next sprint, назначить Assignee |
| Retro | `forgeplan health` → что сделано, что blind spot |

### Для QA

| Когда | Что делать |
|-------|-----------|
| Queue | View "Status = Review" — задачи для проверки |
| Тестирование | Проверить checklist (FR items), `cargo test` |
| Баги | `forgeplan new problem "Bug"` + задача в Orch |
| Approve | Orch: Status → Done, подтвердить evidence |

### Для AI агента (Claude Code main agent)

| Правило | Описание |
|---------|----------|
| **Session start** | ОБЯЗАТЕЛЬНО: Session Start Protocol |
| **Перед работой** | Проверить active tasks в Orchestra |
| **При создании артефакта** | Создать задачу в Orchestra с fields |
| **При смене Phase** | Обновить Phase + Status в Orchestra |
| **При activate** | Пометить задачу Done |
| **При commit** | Обновить Branch field |
| **Перед create** | `search_entities` по Artifact ID — не создавать дубли |
| **НИКОГДА** | Не `send_message` без явного запроса (safety rule) |
| **НИКОГДА** | Не `delete_entity` без подтверждения (destructive) |

### Для Sub-агентов (TeamCreate teammates)

| Правило | Описание |
|---------|----------|
| **Чтение** | Могут читать задачи из Orchestra для контекста |
| **Запись** | НЕ обновляют Orchestra (только main agent / team-lead) |
| **Scope** | Работают только с файлами в своём ownership |
| **Коммуникация** | Через team-lead, не напрямую в Orchestra |

---

## 12. Риски и митигации

### R1: Sync Drift (ВЫСОКИЙ)
**Описание**: Orchestra и Forgeplan рассинхронизируются — задача Done в Orch, но артефакт не activated.
**Вероятность**: Высокая (ручной sync = человеческий фактор)
**Импакт**: Средний (два источника правды → confusion)
**Митигации**:
- Session Start Protocol проверяет оба → детектит drift
- `/sync-tasks` enhanced — показывает diff
- Будущее: Hook на `forgeplan activate` → auto-mark Done в Orch

### R2: Field Bloat (СРЕДНИЙ)
**Описание**: Команда добавляет custom fields для каждого нового need.
**Вероятность**: Средняя (natural tendency)
**Импакт**: Низкий (noise, но не ломает)
**Митигация**: Строго 6 полей. Новое поле = обоснование + обновление этого гайда. R_eff НЕ дублируем.

### R3: Phase vs Status Confusion (ВЫСОКИЙ)
**Описание**: Два параллельных трекинга стадии, один обновлён а другой нет.
**Вероятность**: Высокая
**Импакт**: Средний (AI принимает решения на stale data)
**Митигация**: Чёткий маппинг (§4). AI обновляет оба при любом изменении. При конфликте Status побеждает.

### R4: AI создаёт дубли (СРЕДНИЙ)
**Описание**: AI agent создаёт задачу для артефакта который уже трекается.
**Вероятность**: Средняя (особенно в Config C)
**Импакт**: Низкий (шум, легко удалить)
**Митигация**: Перед `create_entity` ВСЕГДА `search_entities` по Artifact ID.

### R5: Onboarding Friction (ВЫСОКИЙ для Config C)
**Описание**: Новый человек не понимает разделение Forgeplan/Orchestra/Claude Code.
**Вероятность**: Высокая
**Импакт**: Высокий (ломает conventions, создаёт шум)
**Митигация**: Этот гайд + onboarding checklist + AI помогает через Session Start. Первая неделя = buddy system.

### R6: Orchestra Downtime (НИЗКИЙ)
**Описание**: Orchestra API недоступен.
**Вероятность**: Низкая
**Импакт**: Средний (теряем task tracking)
**Митигация**: Forgeplan работает автономно. Claude Code tasks как fallback. Sync после восстановления.

### R7: Sprint Scope Creep (СРЕДНИЙ)
**Описание**: Задачи добавляются в sprint без route/shape.
**Вероятность**: Средняя (давление дедлайнов)
**Импакт**: Средний (нарушение методологии, tech debt)
**Митигация**: AI agent проверяет "есть ли Artifact?" при Status → Doing. Tactical tasks допустимы без артефакта.

### R8: Nesting Limit (Config C only)
**Описание**: Orchestra поддерживает max 3 уровня. Нельзя добавить ещё один.
**Вероятность**: Низкая (нужно только при Config C)
**Импакт**: Высокий (структурное ограничение)
**Митигация**: Использовать parent-child tasks внутри sub-project. Не пытаться добавить 4-й уровень проектов.

---

## 13. Узкие места

| Bottleneck | Описание | Влияние | Решение |
|-----------|----------|---------|---------|
| **Manual dual-create** | Создать артефакт + задачу = 2 действия | Friction на каждую задачу | Auto-sync в `/forge-cycle`. AI создаёт оба |
| **Phase update** | Забыть обновить Phase field | Stale data для AI | AI обновляет при смене Status |
| **Sprint transition** | Перенос незакрытых задач | Overhead каждые 1-2 недели | A/B: change Sprint field. C: move to new sub-project |
| **Cross-area deps** | Задача блокирует другую area | Visibility gap | Orchestra Relations + `forgeplan blocked` |
| **Context window** | AI тратит tokens на Orch queries | Slower responses | Cache workspace overview в session start |
| **Backfill existing** | 20+ артефактов без задач в Orch | Migration effort | Batch script или AI agent один раз |

---

## 14. Anti-patterns

| Anti-pattern | Почему плохо | Правильно |
|-------------|-------------|-----------|
| Дублировать PRD content в Orchestra description | Два источника правды, drift | Только Artifact ID в field |
| Трекать R_eff в Orchestra field | Устаревает мгновенно | `forgeplan score` по запросу |
| Создавать Standard+ задачу без артефакта | Работа без обоснования | Route → Shape → Task |
| `send_message` без запроса пользователя | Safety violation, spam | Только по явной просьбе |
| Sub-agents обновляют Orchestra | Race conditions, конфликты | Только main agent |
| Игнорировать Session Start Protocol | Потеря контекста, дубли | ВСЕГДА выполнять |
| Проект на каждую задачу | Overhead, потеря обзора | Проект = area, не задача |
| Sprint field + Sprint sub-project одновременно | Confusion: где правда? | Выбрать одно по конфигурации |
| Перемещать задачи между projects без причины | Теряется history | Move только при migration |
| Архивировать вместо Done | Задача исчезает из views | Done = видна, Archived = скрыта |

---

## 15. Quick Reference

### Forgeplan (методология)
```bash
forgeplan health              # состояние проекта
forgeplan route "..."         # определить depth
forgeplan new prd "Title"     # создать артефакт
forgeplan validate PRD-XXX    # проверить качество
forgeplan score PRD-XXX       # R_eff scoring
forgeplan activate PRD-XXX    # draft → active
forgeplan list                # список артефактов
forgeplan blocked             # граф зависимостей
```

### Orchestra (задачи)
```bash
/orch status                  # обзор workspace
/orch create "Task name"      # новая задача (interactive)
/orch task <uid>              # детали задачи
/orch msg <uid> "message"     # сообщение в чат задачи
/orch today                   # задачи на сегодня
/orch overdue                 # просроченные задачи
/briefing                     # утренний брифинг
/sync-tasks                   # синхронизация
```

### Claude Code (execution)
```bash
/forge-cycle                  # полный цикл (route → PR)
/sprint                       # wave-based sprint с research
/wave                         # быстрые waves из контекста
/build path/to/reports/       # реализация из research
/audit                        # 5-agent code review
/commands                     # список всех команд
/research "вопрос"            # быстрый поиск (5 агентов)
/deep-research "тема"         # глубокое исследование
```

### MCP tools (для AI agents)

```
# Orchestra
mcp__orch__get_workspace_overview()     — обзор workspace
mcp__orch__query_entities()             — поиск с фильтрами
mcp__orch__create_entity()              — создать задачу/проект
mcp__orch__set_fields()                 — обновить fields
mcp__orch__manage_field()               — создать/изменить field definition
mcp__orch__search_entities()            — поиск по имени
mcp__orch__get_entity()                 — детали entity
mcp__orch__read_messages()              — прочитать сообщения
mcp__orch__get_checklists()             — чек-листы задачи

# Forgeplan
forgeplan_health()                      — состояние проекта
forgeplan_route()                       — определить depth
forgeplan_new()                         — создать артефакт
forgeplan_validate()                    — проверить качество
forgeplan_score()                       — R_eff scoring
forgeplan_activate()                    — draft → active
forgeplan_list()                        — список артефактов
forgeplan_search()                      — поиск артефактов
forgeplan_link()                        — связать артефакты
```

---

## 16. Playbook: сценарии ежедневной работы

### Начало дня

```
Ты: открываешь Claude Code
AI: выполняет Session Start Protocol
AI: "Доброе утро. В работе:
     • [PROB-021] ADI Quality — Doing, Phase: Code, Sprint 9
     Health: 1 blind spot (RFC-003 без evidence)
     Рекомендация: завершить PROB-021, затем fix blind spot"
Ты: "Ок, продолжаю PROB-021"
→ AI подхватывает контекст и работает
```

### Нашёл баг

```
Ты: "Нашёл баг — search не находит артефакты с кириллицей"

AI: forgeplan route "search bug with cyrillic"
    → Tactical (quick fix, обратимо)

AI: создаёт задачу в Orchestra:
    "[BUG] Search cyrillic" — Status: To Do, Tags: Bug
    НЕТ Artifact (Tactical = без артефакта)

Ты: фиксишь баг
AI: cargo test → pass
    Orch: Status → Done
    git commit
```

### Нашёл серьёзный баг (нужно расследование)

```
Ты: "R_eff scoring даёт неправильные результаты при CL0"

AI: forgeplan route "R_eff scoring incorrect for CL0 evidence"
    → Standard (нужно понять причину, может затронуть другие scoring)

AI: forgeplan new problem "R_eff incorrect at CL0"  → PROB-XXX
    Заполняет: Problem, Impact, Reproduction Steps
    forgeplan validate PROB-XXX → PASS

AI: создаёт задачу в Orchestra:
    "[PROB-XXX] R_eff incorrect at CL0"
    Fields: Artifact=PROB-XXX, Type=Problem, Depth=Standard, Phase=Shape
    Status: Backlog, Sprint: Sprint 9, Priority: High

AI: расследует, фиксит, создаёт evidence
    → Orch: Phase: Code → Evidence → Done
```

### Новая идея / фича

```
Ты: "Хочу добавить экспорт артефактов в PDF"

AI: forgeplan route "PDF export for artifacts"
    → Standard (новая фича, 1-3 дня)
    → Pipeline: PRD → RFC

AI: forgeplan new prd "PDF Export"  → PRD-XXX
    Заполняет MUST секции
    forgeplan validate PRD-XXX → PASS

AI: создаёт задачу:
    "[PRD-XXX] PDF Export"
    Fields: Type=PRD, Depth=Standard, Phase=Validate
    Status: To Do, Sprint: Sprint 10 (не текущий — backlog)

Ты: "Давай в следующем спринте"
AI: Sprint=Sprint 10, Status=Backlog
```

### Начинаю следующий спринт

```
Ты: "Начинаем Sprint 10"

AI: 1. Проверяет незакрытые задачи Sprint 9:
       /orch query → Sprint=Sprint 9, Status≠Done
       "2 задачи не закрыты: [RFC-003] и [PROB-023]"

    2. Спрашивает: "Переносим в Sprint 10 или закрываем?"
       Ты: "RFC-003 переноси, PROB-023 закрывай — не актуально"

    3. AI обновляет:
       RFC-003: Sprint → Sprint 10
       PROB-023: Status → Done (или forgeplan deprecate)

    4. forgeplan health → что ready для Sprint 10?
       "4 задачи в Backlog с Sprint 10:
        [PRD-XXX] PDF Export — To Do
        [RFC-003] Layered arch — переносим из Sprint 9
        ..."

    5. Предлагает приоритизацию по Priority field
```

### Закончил задачу (полный цикл)

```
AI: код написан, тесты проходят

AI: /audit → 5-agent review
    Findings: 2 LOW, 0 HIGH
    Фиксит LOW findings

AI: forgeplan new evidence "Tests pass, 15 new tests, audit clean"
    forgeplan link EVID-XXX PRD-XXX --relation informs
    forgeplan score PRD-XXX → R_eff = 1.00

AI: forgeplan review PRD-XXX → PASSED
    forgeplan activate PRD-XXX → active

AI: Orch: Phase=Done, Status=Done ✓

AI: git commit + git push + gh pr create
    Orch: Branch=feat/xxx

AI: "PRD-XXX завершён. R_eff=1.00, activated, PR создан."
```

### Конфликт: задача блокирует другую

```
Ты: "Не могу делать PRD-025 пока RFC-005 не готов"

AI: forgeplan link PRD-025 RFC-005 --relation blocked-by
    Orch: добавляет relation между задачами
    PRD-025 Status → Backlog (blocked)

    forgeplan blocked → показывает граф зависимостей

Ты: "Тогда делаю RFC-005 первой"
AI: RFC-005 Status → Doing, Phase → Code
```

### Конфликт: что-то сломалось после merge

```
Ты: "cargo test падает после merge в dev"

AI: НЕ создаёт артефакт (это hotfix)
    → Исследует, находит причину
    → git checkout -b fix/broken-tests
    → Фиксит

AI: создаёт задачу:
    "[BUG] Tests broken after merge" — Tags: Bug, Priority: High
    Status: Doing → Review → Done

    git commit + git push + gh pr create --base dev
```

### Хочу обсудить задачу с командой

```
Ты: "Напиши в чат задачи PRD-021 что я начал работу"

AI: mcp__orch__send_message(chatUid=задача, message="Начата работа...")
    ← ТОЛЬКО по явному запросу!

Ты: "Что писали в чате PROB-021?"
AI: mcp__orch__read_messages(entityUid=задача)
    → Показывает историю
```

### Конец дня

```
Ты: "Что сегодня сделано?"

AI: Orch: query tasks updated today
    "Сегодня:
     ✅ [PROB-021] ADI Quality → Done
     🔄 [PRD-XXX] PDF Export → Shape (PRD заполнен)
     🔵 [RFC-003] Layered arch → без изменений"

    forgeplan health:
    "Active: 5, Draft: 15, Blind spots: 1"

Ты: "Ок, на завтра?"
AI: "Рекомендация: RFC-003 blind spot (нужен evidence),
     затем [PRD-XXX] PDF Export в Sprint 10"
```

### Ретроспектива / обзор спринта

```
Ты: "Подведи итоги Sprint 9"

AI: Orch: query Sprint=Sprint 9
    "Sprint 9:
     Done: 5 задач (3 PRD, 1 PROB, 1 RFC)
     Не завершены: 1 (перенесена в Sprint 10)
     Время: 2 недели"

    forgeplan health:
    "Activated: 3 новых (PRD-021, PROB-021, RFC-004)
     R_eff avg: 0.85
     Blind spots: 1 (RFC-003)"

    "Рекомендация для Sprint 10: закрыть blind spot RFC-003,
     начать PRD-XXX PDF Export"
```

## 17. Чего НЕЛЬЗЯ делать (запреты)

| Ситуация | Запрет | Почему | Правильно |
|----------|--------|--------|-----------|
| Баг найден | Начинать код без route | Может быть не Tactical | `forgeplan route` сначала |
| Standard+ задача | Писать код без PRD | Нет обоснования | Shape → Validate → Code |
| Задача готова | `forgeplan activate` без evidence | R_eff = 0, blind spot | Создать evidence первым |
| Нужно обсудить | `send_message` самовольно | Safety rule Orchestra | Только по запросу юзера |
| Задача не нужна | `delete_entity` | Destructive | Status=Done или deprecate |
| Спринт закончился | Удалять старые задачи | Теряется history | Done или Archive |
| Merge конфликт | `git push --force` | Blocked hook | Resolve конфликт |
| Тесты падают | Коммитить | `commit-test-check` hook | Починить тесты |
| Active артефакт устарел | Удалять | Теряется lineage | `forgeplan supersede` или `deprecate` |
| AI создаёт задачу | Не проверять дубли | Шум в трекере | `search_entities` сначала |

---

## 18. Inbox Pattern: сбор и triage сигналов

### Проблема

Сигналы (идеи, решения, наблюдения) возникают в разных местах:
- Переписка в чате Orchestra
- Звонки и встречи
- Git history (коммиты без артефактов)
- AI наблюдения (дубли в коде, flaky tests)
- Forgeplan health (stale artifacts, blind spots)

Если их не собирать — решения теряются, идеи забываются, tech debt копится.

### Решение: Inbox при Session Start

```
Сигналы из разных источников
│         │          │          │
Chat Orch │   Git    │  Звонки  │  AI фоновый
    │     │    │     │    │     │      │
    ▼     ▼    ▼     ▼    ▼     ▼      ▼
    └──────────────────────────────────┘
                     │
                     ▼
          ┌──────────────────┐
          │     INBOX        │ ← AI собирает (read-only)
          │  (session start) │
          └────────┬─────────┘
                   │
                   ▼
          ┌──────────────────┐
          │   TRIAGE         │ ← Человек решает
          │   (с AI помощью) │
          └────────┬─────────┘
                   │
      ┌────────────┼────────────┐
      ▼            ▼            ▼
 Отбросить    Note/Memory    Артефакт
 (шум)        (контекст)     + Задача
```

### Как AI собирает Inbox (автоматически при session start)

```
STEP 1: СБОР (read-only, безопасно)
├── mcp__orch__get_unread_chats()     → новые сообщения
├── mcp__orch__get_mentions()         → @упоминания
├── git log --since="last session"    → новые коммиты
├── forgeplan health                  → stale, blind spots
└── memory_recall                     → контекст прошлой сессии

STEP 2: КЛАССИФИКАЦИЯ (AI предлагает, человек валидирует)
"📬 Inbox (5 сигналов):

 1. 💬 @alice в чате PROB-021: 'Может добавить кэш?'
    → Предложение: новая фича (PRD?) или tactical fix

 2. 🔀 3 коммита на dev без артефакта (от @bob)
    → Предложение: нужен route, или это Tactical

 3. ⚠️ forgeplan health: RFC-003 stale (60 дней)
    → Предложение: renew или deprecate

 4. 🤖 AI наблюдение: дублирование в scoring (90 LOC)
    → Предложение: рефакторинг задача

 5. 📞 (ручной ввод) 'На звонке решили: PostgreSQL'
    → Предложение: ADR

 Что делаем с каждым?"

STEP 3: ЧЕЛОВЕК РЕШАЕТ
"1 → PRD, 2 → игнор, 3 → deprecate, 4 → Note, 5 → ADR"

STEP 4: AI ВЫПОЛНЯЕТ решения
```

### Типы сигналов и что с ними делать

| Источник | Тип сигнала | Возможное действие | Кто решает |
|----------|------------|-------------------|-----------|
| **Чат Orchestra** | Идея, предложение | Note → PRD (если Standard+) | Человек |
| **Чат Orchestra** | Решение ("давай так") | ADR или Note | Человек |
| **Чат Orchestra** | Баг-репорт | Problem → задача | Человек |
| **Звонок/встреча** | Архитектурное решение | ADR | Человек (ввод после звонка) |
| **Звонок/встреча** | Новая фича | PRD | Человек (ввод после звонка) |
| **Звонок/встреча** | Изменение приоритетов | Sprint update | Человек |
| **Git** | Коммиты без артефакта | Route → может нужен PRD | AI предлагает, человек решает |
| **Git** | Flaky tests | Problem | AI предлагает, человек решает |
| **forgeplan health** | Stale artifact | Renew или deprecate | AI предлагает, человек решает |
| **forgeplan health** | Blind spot | Создать evidence | AI предлагает, человек решает |
| **AI наблюдение** | Дубли в коде | Note или рефакторинг задача | AI предлагает, человек решает |
| **AI наблюдение** | Security issue | Problem (High priority) | AI предлагает, человек решает |

### Как фиксировать решения со звонков

| Подход | Усилие | Как |
|--------|--------|-----|
| **Сказать AI** | Низкое | "На звонке решили: [1] PostgreSQL [2] дедлайн 15/04 [3] Alice migration" → AI создаёт артефакты |
| **Написать в чат задачи** | Низкое | Написать резюме в Orchestra → AI прочитает при session start |
| **Meeting notes документ** | Среднее | Document в Orchestra "Meeting 2026-04-03" → AI парсит |
| **Транскрипция** | Высокое | Otter.ai / Fireflies → скормить AI для экстракции решений |

**Рекомендация**: "Сказать AI" — самый быстрый и надёжный. AI знает контекст, создаёт правильные артефакты.

### Что может работать в фоне (safety matrix)

| Действие | В фоне? | Причина |
|----------|---------|---------|
| Читать чаты Orchestra | ✅ Да | Read-only |
| Читать git log | ✅ Да | Read-only |
| forgeplan health | ✅ Да | Read-only |
| Классифицировать сигналы | ✅ Да | Подготовка для triage |
| Сохранить в Memory/Hindsight | ✅ Да | Non-destructive |
| **Создать артефакт** | ❌ Нет | Нужно подтверждение |
| **Создать задачу** | ❌ Нет | Нужно подтверждение |
| **Отправить сообщение** | ❌ Нет | Safety rule |
| **Удалить/архивировать** | ❌ Нет | Destructive |
| **Изменить Status/Phase** | ❌ Нет | Нужно подтверждение |

**Принцип**: AI СОБИРАЕТ и ПРЕДЛАГАЕТ. Человек РЕШАЕТ и ПОДТВЕРЖДАЕТ. AI ВЫПОЛНЯЕТ.

### Проблемы и решения

#### P1: Слишком много сигналов (inbox overflow)

**Проблема**: После выходных 30+ сообщений, 20 коммитов, 5 stale artifacts. Inbox огромный.

**Решение**: AI приоритизирует:
1. 🔴 **Требуют действия**: mentions, overdue tasks, stale artifacts
2. 🟡 **Полезно знать**: решения в чатах, новые коммиты
3. ⚪ **Фон**: AI наблюдения, minor issues

Показывает 🔴 сразу, 🟡 по запросу, ⚪ только если спросят.

#### P2: Дублирование — то же самое в чате и в git

**Проблема**: Alice написала в чат "сделала кэширование" И закоммитила. AI видит два сигнала.

**Решение**: AI дедуплицирует при классификации:
- Проверяет совпадение по времени + автору + теме
- Показывает как один сигнал с двумя источниками

#### P3: Контекст звонка теряется

**Проблема**: На звонке обсудили 5 вещей, после звонка помнят 2.

**Решение**: Привычка "5 минут после звонка":
```
Сразу после звонка:
  Ты: "Зафиксируй с звонка:
    1. Решили: PostgreSQL вместо SQLite (причина: concurrent writes)
    2. Решили: дедлайн Phase 5 — конец апреля
    3. Задача: Alice делает migration план
    4. Идея: добавить real-time sync (обсудить позже)
    5. Отменили: не делаем GraphQL API"

  AI: создаёт ADR для п.1, обновляет Sprint/due dates для п.2,
      создаёт задачу для п.3, Note для п.4, deprecate для п.5
```

#### P4: AI наблюдения не точны

**Проблема**: AI говорит "дубли в коде" но это не дубли, а намеренный паттерн.

**Решение**: AI наблюдения = самый низкий приоритет (⚪). Человек решает, AI не настаивает. False positive → AI запоминает (Memory) что это не дубли.

#### P5: Несколько людей — кто делает triage?

**Проблема**: В Config B/C — кто обрабатывает inbox? Каждый своё или один PM?

**Решение по конфигурациям**:
- **Config A** (Solo): ты = triage owner
- **Config B** (Small Team): каждый делает свой inbox (своё mentions, свои задачи). PM делает cross-area triage
- **Config C** (Medium Team): PM/Tech Lead делает общий triage на standup. Devs делают свой personal inbox

#### P6: Сигнал пришёл ночью, утром уже неактуален

**Проблема**: В чате вчера обсуждали подход, утром уже решили по-другому.

**Решение**: AI при сборе inbox смотрит на весь тред, не на отдельные сообщения. Показывает последнее состояние обсуждения, не каждое промежуточное сообщение.

### Inbox в Session Start Protocol (обновлённый)

```
STEP 1: CONTEXT RESTORE
├── CLAUDE.md + memory_recall

STEP 2: INBOX COLLECTION (NEW — read-only, фоновый)
├── Orch: unread chats, mentions
├── Git: commits since last session
├── Forgeplan: health changes
└── AI: наблюдения из кода (если были)

STEP 3: PROJECT HEALTH
├── forgeplan health
└── Orch: active tasks, overdue

STEP 4: INBOX TRIAGE (если есть сигналы)
"📬 Inbox (N сигналов): [приоритизированный список]
 Что делаем?"
→ Человек решает

STEP 5: SYNTHESIS + RECOMMEND
"Сейчас в работе: ... Следующее: ..."
```

---

## Changelog

| Дата | Версия | Изменения |
|------|--------|-----------|
| 2026-04-03 | v1.0 | Initial: architecture, 3 configs, greenfield/brownfield, migration, roles, risks |
| 2026-04-03 | v1.1 | Added: Playbook (10 scenarios), Prohibitions table, CLAUDE.md integration |
| 2026-04-03 | v1.2 | Added: Inbox Pattern (signal collection, triage, safety matrix, 6 problems+solutions) |
