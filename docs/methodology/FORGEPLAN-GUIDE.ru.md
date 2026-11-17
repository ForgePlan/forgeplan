[English](FORGEPLAN-GUIDE.md) · [Русский](FORGEPLAN-GUIDE.ru.md)

# Forgeplan — Полный практический гайд

> Один документ: методология + команды + примеры + подводные камни.
> Для человека и AI агента.

---

## Что такое Forgeplan

Forgeplan помогает **думать перед кодом**: выбрать глубину, описать и проверить решение, реализовать его, собрать доказательства, затем провести ревью и активировать. CLI/MCP автоматизируют операции с артефактами; инженерную работу выполняют люди и агенты.

**Не Jira.** Не project management. Не task tracker. Forgeplan — это **structured knowledge base** для инженерных решений.

AI-агенты могут работать через MCP, люди и агенты — через CLI. Ни один интерфейс сам по себе не проверяет корректность реализации.

---

## Установка

### 1. AI Skill (для любого AI агента)

```bash
# Установить встроенные инструкции /forge для Claude Code (нужен бинарь ниже)
forgeplan setup-skill
```

Команда записывает `~/.claude/skills/forge/SKILL.md`, но не устанавливает агентов маркетплейса, `/forge-cycle`, MCP-конфигурацию или защитные хуки. Для полного процесса установите опубликованный плагин `forgeplan-workflow` и нужные проекту наборы специалистов.
```
/forge "Добавить OAuth2 аутентификацию"
```

### 2. CLI Binary

```bash
# macOS (Homebrew)
brew install forgeplan/tap/forgeplan
# Homebrew 6.0+: если отклонено с "untrusted tap", выполните `brew trust forgeplan/tap` один раз и повторите.

# Из исходников (Rust)
cargo install forgeplan

# Или скачать binary из GitHub Releases
# https://github.com/ForgePlan/forgeplan/releases
```

### 3. MCP Server (для AI агентов)

Добавить в `.mcp.json` проекта:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"]
    }
  }
}
```

---

## Быстрый старт (5 минут)

```bash
# 1. Инициализировать workspace
forgeplan init

# 2. Определить что делать
forgeplan route "Add OAuth2 authentication" --level 0
# → Depth: Deep, Pipeline: PRD → Spec → RFC → ADR

# 3. Создать первый артефакт
forgeplan new prd "OAuth2 Authentication"

# 4. Посмотреть состояние
forgeplan health
```

> **Alias**: релизные установщики предоставляют `fpl`. После установки из исходников `forgeplan setup --skip-model --skip-hooks` создаёт его рядом с бинарём без загрузки модели и установки git-хуков.

---

## Методология: 3 вопроса вместо бюрократии

### Вопрос 1: "Какой depth?"

Задай себе один вопрос: **"Это обратимо за день?"**

| Ответ | Depth | Что создавать | Пример |
|-------|-------|--------------|--------|
| Да, тривиально | **Tactical** | Ничего или Note | Fix typo, update config |
| Нет, есть выбор | **Standard** | PRD → RFC | Новая фича, 1-3 дня |
| Нет, затрагивает многих | **Deep** | PRD → Spec → RFC → ADR | Новый модуль, 1-2 недели |
| Стратегия, кросс-команда | **Critical (политика маркетплейса)** | Epic + согласованные Deep-направления | Новая подсистема |

Или используй автоматический routing:

```bash
forgeplan route "описание задачи"
```

`--level 0` принудительно включает локальный поиск ключевых слов. Без него CLI 0.38 пробует настроенную LLM и при недоступности возвращается к правилам; автоматический режим может дополнить Deep-результат LLM рассуждением ADI. В MCP `forgeplan_route` 0.38 **нет параметра `level`**. Формулировка CLI help «keywords (default)» уже фактического поведения.

**Конвейер ядра 0.38**: Tactical — без артефактов, Standard — PRD → RFC, Deep — PRD → Spec → RFC → ADR. Ядро разбирает `critical` как Deep и не выдаёт отдельный Critical-маршрут. **Опубликованная методология маркетплейса** добавляет SOL (сравнение подходов) до проектирования и REFRESH (результат согласования) на Deep/Critical. Не смешивайте требования процесса с выводом бинаря. См. [зафиксированные источники](#зафиксированные-источники).

### Вопрос 2: "Какой артефакт?"

| Артефакт | Отвечает на вопрос | Когда НЕ нужен |
|----------|-------------------|----------------|
| **PRD** | ЧТО и зачем? | Баг-фикс, рефакторинг |
| **RFC** | КАК строим? | Архитектура очевидна, < 1 дня |
| **ADR** | ПОЧЕМУ так решили? | Решение тривиально и обратимо |
| **Spec** | КАК ТОЧНО работает? | Нет API / data model changes |
| **Epic** | Как группировать? | Задача = один PRD |

### Вопрос 3: "Готов ли артефакт?"

```bash
forgeplan review PRD-001
# → MUST: Missing Problem section
# → SHOULD: density < 50 words
# → Ready to activate? NO
```

Исправьте MUST-находки и все дополнительные ошибки активации. Для PRD/Epic/Spec/RFC/ADR review также проверяет длину тела, признаки незаполненного шаблона и связь с доказательствами. Машинный PASS — не независимое ревью и не доказательство работоспособности кода.

### Главное правило

**Pipeline = guideline, НЕ бюрократия.** Не создавай все 10 типов на каждую задачу. Tactical depth = просто делай. Standard = PRD + RFC. Только Deep+ требует полный pipeline.

---

## Все команды (по категориям)

### Создание и управление артефактами

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan init` | Создать .forgeplan/ workspace | `forgeplan init` |
| `forgeplan new <kind> "<title>"` | Создать артефакт из шаблона | `forgeplan new prd "Auth System"` |
| `forgeplan get <id>` | Прочитать артефакт | `forgeplan get PRD-001` |
| `forgeplan update <id>` | Обновить метаданные/body | `forgeplan update PRD-001 --body @prd.md` (lifecycle-статусы — через `activate` / `supersede` / `deprecate`) |
| `forgeplan delete <id>` | Удалить артефакт | `forgeplan delete PRD-001 --yes` |
| `forgeplan list` | Список артефактов | `forgeplan list --type prd --status active` |

**Виды артефактов (kind):** `prd`, `epic`, `spec`, `rfc`, `adr`, `note`, `problem`, `solution`, `evidence`, `refresh`

Это распространённые типы процесса решений; ядро 0.38 поддерживает также локальный `memory` и шесть brownfield-типов (`use_case`, `glossary`, `invariant`, `scenario`, `hypothesis`, `domain_model`). Канонический Markdown находится в `.forgeplan/`; LanceDB — перестраиваемый индекс запросов. Предпочитайте CLI/MCP для записи; `reindex` восстанавливает индекс из Markdown, а `scan-import` импортирует другие документы.

### Связи и граф

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan link <src> <tgt>` | Связать артефакты | `forgeplan link RFC-001 PRD-001 --relation based_on` |
| `forgeplan graph` | Mermaid dependency graph | `forgeplan graph` |

**Типы связей (--relation):** `informs`, `based_on`, `supersedes`, `contradicts`, `refines`

### Качество и валидация

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan validate [id]` | Проверить полноту | `forgeplan validate PRD-001` |
| `forgeplan score [id]` | R_eff quality score | `forgeplan score PRD-001` |
| `forgeplan fgr [id]` | F-G-R scores (Formality, Granularity, Reliability) | `forgeplan fgr` |
| `forgeplan estimate <id>` | Effort estimate по грейдам (Jun/Mid/Sen/PS/AI) | `forgeplan estimate PRD-022` |
| `forgeplan estimate <id> --grade mid` | Подсветить конкретный грейд | `forgeplan estimate PRD-022 --grade junior` |
| `forgeplan estimate <id> --my-grade` | Грейд из config grade_profile | `forgeplan estimate PRD-022 --my-grade` |

### Lifecycle

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan review <id>` | Чеклист: готов к активации? | `forgeplan review PRD-001` |
| `forgeplan activate <id>` | Draft → Active (validation gate) | `forgeplan activate PRD-001` |
| `forgeplan supersede <id> --by <new>` | Active → Superseded + chain warnings | `forgeplan supersede PRD-001 --by PRD-002` |
| `forgeplan deprecate <id> --reason "..."` | Active → Deprecated | `forgeplan deprecate PRD-001 --reason "Cancelled"` |

**Правило:** полные проверки активации действуют для PRD, RFC, ADR, Epic и Spec. Лёгкие типы, например Note и Problem, их пропускают; базовая валидация не означает оценки качества.

### Dashboards и аналитика

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan health` | Полное здоровье проекта | `forgeplan health --compact` |
| `forgeplan status` | Краткий dashboard | `forgeplan status` |
| `forgeplan blindspots` | Артефакты без evidence, orphans | `forgeplan blindspots` |
| `forgeplan journal` | Timeline решений с R_eff | `forgeplan journal --risk` |
| `forgeplan fpf dashboard` | FPF dashboard: contexts + F-G-R + actions | `forgeplan fpf dashboard` |
| `forgeplan stale` | Артефакты с expired valid_until | `forgeplan stale` |
| `forgeplan decay` | Impact expired evidence на R_eff | `forgeplan decay` |
| `forgeplan progress [id]` | Checkbox progress bars | `forgeplan progress` |

### Routing и calibration

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan route "<description>" --level 0` | Локальный выбор глубины по словам + конвейер ядра | `forgeplan route "Add OAuth2" --level 0` |
| `forgeplan route "<desc>" --level 1` | Классификация LLM с возвратом к правилам при недоступности | `forgeplan route "Add OAuth2" --level 1` |
| `forgeplan calibrate [id]` | Suggest depth для существующего артефакта | `forgeplan calibrate PRD-001` |

### AI-генерация и локальный семантический поиск

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan generate <kind> "<desc>"` | AI генерация артефакта | `forgeplan generate prd "Payment system"` |
| `forgeplan reason <id>` | ADI reasoning cycle | `forgeplan reason PRD-001 --json` |
| `forgeplan decompose <id>` | PRD → RFC задачи через AI | `forgeplan decompose PRD-001` |
| `forgeplan capture "<decision>"` | Записать решение как Note/ADR | `forgeplan capture "Use Redis for cache"` |
| `forgeplan search <query> --semantic` | Локальные эмбеддинги, не вызов LLM; нужны модель и индекс | `forgeplan search "auth" --semantic` |

### MCP Server

```bash
forgeplan serve  # запустить MCP server (stdio transport)
```

Сервер 0.38 предоставляет 73 инструмента. CLI и MCP пересекаются, но не идентичны; параметры и доступность уточняйте в справочнике соответствующего интерфейса.

---

## Estimate Engine — оценка трудозатрат

### Зачем

Превращает документацию (FR в PRD, Phases в RFC) в эстимейты трудозатрат. Не нужна отдельная Excel-таблица — estimate живёт рядом с артефактами.

### Базовая команда

```bash
forgeplan estimate PRD-022
```

Выводит таблицу:

```
Estimate for PRD-022: AI Estimation Engine
Confidence: 40%

  ID      Description                  Cmpl   Jun    Mid  Senior    PS     AI
  ---------------------------------------------------------------------------
  FR-001  User can run estimate          3    16h    12h    8.0h  5.6h   1.0h
  FR-002  System extracts work items     3    16h    12h    8.0h  5.6h   1.0h
  FR-003  Fibonacci complexity           2    10h   7.5h    5.0h  3.5h   0.7h
  ---------------------------------------------------------------------------
  TOTAL                                  8    42h    32h     21h   15h   2.7h
                                              5.3d   3.9d   2.6d  1.8d  0.3d
```

### Флаги

```bash
forgeplan estimate PRD-022 --grade middle   # подсветить конкретный грейд
forgeplan estimate PRD-022 --my-grade       # грейд из config.yaml (домен-aware)
forgeplan estimate PRD-022 --json           # машинный вывод
```

### Модель расчёта

**Base = Senior** (baseline ×1.0). Все грейды — множители от Senior:

| Грейд | Множитель | Пример (Medium=8h Senior) |
|-------|-----------|---------------------------|
| Junior | ×2.0 | 16h |
| Middle | ×1.5 | 12h |
| **Senior** | **×1.0** | **8h** (baseline) |
| Principal | ×0.7 | 5.6h |
| AI | task-type | 1.0h (PureCoding) |

**AI считается по-другому** — учитывает тип задачи:

| Тип задачи | AI множитель | Пример (8h base) | С review (+30%) |
|------------|-------------|-------------------|-----------------|
| PureCoding | ×0.10 | 0.8h | **1.04h** |
| CodingInfra | ×0.25 | 2.0h | 2.6h |
| DesignCoding | ×0.30 | 2.4h | 3.1h |
| PureInfra | ×0.50 | 4.0h | 5.2h |
| Coordination | ×1.00 | 8.0h | 10.4h |

**Fibonacci complexity** (1, 2, 3, 5, 8, 13) → base Senior hours (3h, 5h, 8h, 13h, 21h, 34h).

**Confidence** зависит от полноты артефакта:
- Есть FR в PRD: +30%
- Есть Implementation Phases в RFC: +25%
- Есть Spec: +15%
- Есть evidence из прошлых задач: +20%

### Настройка в config.yaml

Раскомментируй и настрой под себя:

```yaml
# .forgeplan/config.yaml
estimate:
  grade_profile:
    backend: middle        # твой грейд в бэкенде
    frontend: junior       # твой грейд во фронте
    devops: senior         # твой грейд в devops
    ai_ml: principal       # твой грейд в AI/ML
    default: senior        # fallback для незнакомых доменов
  grade_multipliers:       # override defaults если нужно
    junior: 2.0
    middle: 1.5
    senior: 1.0
    principal: 0.7
    ai: 0.4
  ai_task_multipliers:     # скорость AI по типам задач
    pure_coding: 0.10      # AI делает кодинг в ~10x быстрее
    coding_infra: 0.25     # код + инфраструктура
    design_coding: 0.30    # дизайн + реализация
    pure_infra: 0.50       # чистая инфра (K8s, CI/CD)
    coordination: 1.00     # meetings — AI не помогает
  review_overhead: 0.30    # +30% к AI time на human review
  safety_margin: 0.50      # предупреждать если спринт > 50%
```

После настройки `--my-grade` автоматически подставит правильный грейд:

```bash
forgeplan estimate PRD-022 --my-grade
# → "Using grade: Middle (domain: backend, from config grade_profile)"
```

### Multi-grade профиль: зачем

Ты можешь быть **Senior в DevOps** и **Junior во Frontend** одновременно. Одна задача на K8s занимает 5h (Senior), а такая же по сложности задача на React — 10h (Junior). Forgeplan учитывает это через `grade_profile`.

### Рабочий цикл с estimate

```bash
# 1. Создал PRD с FR
forgeplan new prd "Auth System"
# → заполнил FR-001..FR-005

# 2. Оценил трудозатрат
forgeplan estimate PRD-022
# → Senior: 52h (6.5 дней), AI: 6.9h (0.9 дня)

# 3. Создал RFC, дополнил estimate
forgeplan estimate RFC-005
# → 12 phase steps, confidence +25%

# 4. Планируешь спринт с safety margin 40-50%
# Senior capacity = 80h/sprint → берём задач на 40h max
```

---

## Evidence и R_eff — как подтверждать решения

### Зачем

Без evidence R_eff = 0.0 у всех артефактов. Health dashboard кричит "At Risk". Решения приняты на словах, не на фактах.

### Как создать EvidencePack

```bash
forgeplan new evidence "Benchmark: LanceDB vs SQLite insert performance"
```

### ВАЖНО: Structured Fields

Явно укажите вердикт, соответствие контекста и тип доказательства. Ниже учебный пример, не опубликованный бенчмарк:

```markdown
## Measurements

Протестировал insert 1000 records:
- LanceDB: 5ms average
- SQLite + faiss: 12ms average

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: benchmark
```

| Field | Значения | Описание |
|-------|----------|----------|
| `verdict` | `supports` / `weakens` / `refutes` | Подтверждает, ослабляет или опровергает |
| `congruence_level` | `0`-`3` | CL3 = same context (лучший). CL0 = opposed context (penalty 0.9) |
| `evidence_type` | `measurement` / `test` / `benchmark` / `audit` | Тип доказательства |

В 0.38 отсутствие/неизвестное значение `verdict` либо отсутствие и `congruence_level`, и корректного `source_tier` приводит к CL0. Для свежего supports-пакета рекурсивная оценка тогда не выше 0.1: test/measurement = 0.1, benchmark/audit = 0 после штрафа за тип. Weakens/refutes также могут получить 0. Отсутствие только `evidence_type` даёт `measurement`, а не CL0. Указывайте все три поля явно, без inline-комментариев `#`.

### Привязать evidence к артефакту

```bash
forgeplan link EVID-001 ADR-002 --relation informs
forgeplan score ADR-002
# Если есть только свежий supports/CL3/benchmark из примера и нет более слабой зависимости: R_eff = 0.90
```

### Congruence Levels (CL)

| CL | Penalty | Когда |
|----|---------|-------|
| CL3 | 0.0 | Evidence собрано на целевой системе (benchmark на нашем коде) |
| CL2 | 0.1 | Похожий контекст (benchmark другого проекта на таком же стеке) |
| CL1 | 0.4 | Другой контекст (статья, документация, чужой опыт) |
| CL0 | 0.9 | Противоположный контекст (evidence из другой domain) |

### R_eff = min(evidence_scores)

R_eff = weakest link. Если есть 3 evidence и одно слабое — R_eff = слабое. НЕ average. Участвует только *текущая* эвиденция: superseded/deprecated пакеты исключаются из min (ADR-020) — устаревший слабый пакет вытесняй через `supersede <старый> --by <новый>`, а не правкой его verdict.

Пользовательская команда `forgeplan score` использует **рекурсивный** расчёт. Свежий пакет даёт `max(0, verdict_score − CL_penalty − type_penalty)`: test/measurement вычитают 0, benchmark — 0.1, audit — 0.2. Истёкший учитываемый пакет получает 0.1 до этих вычитаний. Минимум собственных доказательств сравнивается с рекурсивными оценками зависимостей: для `based_on`/`informs` вычитается ещё 0.1, для `refines` — 0. Терминальные доказательства и draft/терминальные зависимости исключаются; зависимости Note/Memory пропускаются. Поэтому бенчмарк выше даёт `1 − 0 − 0.1 = 0.90`, не 1.00.

Плоская совместимая helper-функция `r_eff()` не учитывает штраф типа и обход зависимостей; её формулу нельзя выдавать за результат CLI. Источники: [рекурсивный учёт типа](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/reff.rs#L232-L254), [минимум зависимостей](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/reff.rs#L424-L499) и [расчёт CLI](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/score.rs#L130-L142).

---

## Validation — как проверять качество

### Правила по depth

| Тип / глубина | Обязательное содержимое тела в 0.38 |
|---------------|------------------------------------|
| PRD, любая глубина | Problem, Goals, Non-Goals, Functional Requirements, Related Artifacts |
| PRD, Standard+ | Также Target Users; плотность Problem и отсутствие технологий в FR — SHOULD на Standard, MUST на Deep |
| PRD, Deep | Также Timeline, Stakeholders, Acceptance Criteria, Risk Assessment, Success Metrics; доменные проверки могут добавлять требования |
| RFC | Summary, Motivation, Proposed Direction; Risks на Deep. Options и Implementation Phases — SHOULD |
| ADR | Context, Decision, Consequences; Invariants и Rollback на Deep (иначе SHOULD) |
| Epic | Vision, Outcomes, Children, Phases, Progress |
| Spec | Summary, Related Artifacts и структурный или поведенческий контракт |

Для всех типов действуют и базовые правила. В 0.38 наличие/измеримость NFR и `### Requirement` без вложенного `#### Scenario` проверяются как SHOULD. Это минимум, не замена содержательному ревью.

### Validator aliases

Validator принимает синонимы:

| Ожидает | Принимает также |
|---------|----------------|
| `## Problem` | `## Motivation`, `## Problem Statement`, `## Background` |
| `## Goals` | `## Success Criteria`, `## Objectives` |
| `## Non-Goals` | `## Out of Scope`, `## Product Scope` |
| `## Related` | `## Related Artifacts`, `## Dependencies` |
| `## Target Users` | `## Target Audience`, `## Users`, `## Audience` |

### Что проверяет validation

- **MUST** — блокирует activation. Обязательные секции, frontmatter поля.
- **SHOULD** — предупреждение. Плотность текста, отсутствие tech leakage в FR.
- **COULD** — совет. FR format `[Actor] can [capability]`.

---

## Lifecycle — от Draft до Active

```
Draft ──review──→ Draft (если MUST failures)
Draft ──activate──→ Active (если MUST пройдены)
Active ──supersede──→ Superseded (связь с заменой; терминальный)
Active ──deprecate──→ Deprecated (с причиной; терминальный)
Active ──stale──→ Stale ──renew──→ Active
Stale ──reopen──→ Deprecated (старый) + новый Draft
```

Истечение `valid_until` обнаруживают `stale` и scoring; не рассчитывайте на фоновое переписывание статуса. Используйте lifecycle-команды, не `update --status active|superseded|deprecated`. Рекомендательная `phase` и состояние сессии существуют отдельно от статуса.

### Типичный flow

```bash
# 1. Создал артефакт
forgeplan new prd "Payment Processing"

# 2. Заполнил body (Problem, Goals, Non-Goals, FR, Related, Target Users)

# 3. Проверил
forgeplan review PRD-001
# → MUST fix: Missing Problem section

# 4. Доработал body
forgeplan update PRD-001 --body @/tmp/prd-001-body.md

# 5. Повторил review
forgeplan review PRD-001
# → Review PASSED — ready to activate

# 6. После реальных проверок, связанных доказательств и требуемого независимого ревью
forgeplan activate PRD-001
# → draft → active
```

### build-on-draft warning

Если RFC ссылается на PRD который ещё в Draft — review покажет warning:
```
⚠ build-on-draft: depends on PRD-001 which is still Draft
```

Это не блокирует activation, но сигнализирует о незрелой зависимости.

---

## Интеграция с AI агентами

### Вариант 1: Skill + MCP (рекомендуется)

```bash
# Установить только встроенный /forge для Claude Code
forgeplan setup-skill
```

Для других сред нужны собственные инструкции/MCP-настройки. Команды маркетплейса — контракты для оркестратора, а не дополнительные CLI-подкоманды.

После установки:
```
/forge "Добавить OAuth2 аутентификацию"
```

### Вариант 2: MCP Server напрямую

В `.mcp.json` проекта (Claude Code, Cursor):

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"]
    }
  }
}
```

### Вариант 3: Rules файлы (для агентов без MCP)

| Агент | Файл | Что добавить |
|-------|------|-------------|
| Claude Code | `CLAUDE.md` | Секция "Как пользоваться Forgeplan CLI" (см. этот проект) |
| Cursor | `.cursorrules` | Те же правила в формате Cursor |
| Codex | `AGENTS.md` | Инструкции для Codex |
| Gemini CLI | `.gemini/rules` | Правила для Gemini |

### Core workflow (6 tools)

```
1. forgeplan_health     → session start: что происходит в проекте?
2. forgeplan_route      → "что создавать?" depth + pipeline
3. forgeplan_new        → создать артефакт
4. forgeplan_validate   → проверить качество
5. forgeplan_review     → готов к активации?
6. forgeplan_activate   → draft → active
```

Эти шесть инструментов помогают сориентироваться и управлять статусом. Для работы нужны также обновления тела, связи, доказательства и независимые проверки; число инструментов не измеряет полноту процесса.

---

## Forge Mode — permission model для AI агентов

Forge Mode — политика разрешений проекта/среды, а не защитная граница, устанавливаемая бинарём Forgeplan. Разделяйте операции по последствиям, а не общим разрешением `forgeplan:*` или `git:*`.

| Операция | Кто отвечает |
|----------|--------------|
| Читать артефакты / состояние | Агент соблюдает разрешения среды на чтение |
| Править, коммитить, выполнять проверки | Политика проекта определяет полномочия и команды |
| Удалять, обходить проверки, публиковать, делать push | Получить разрешение, требуемое проектом/средой |

`setup-skill` устанавливает только инструкции. `setup` может установить git-хуки синхронизации индекса; это не общая защита shell-команд. Ни `acceptEdits`, ни чёрный список, ни автономный промпт не разрешают обходить независимое ревью или согласие пользователя.

### /forge-cycle — полный FPF-aligned dev cycle

`/forge-cycle` входит в опубликованный плагин `forgeplan-workflow`, а не CLI 0.38. Оркестратор выполняет операции, обозначенные `/sprint`, `/team-up` и `/audit`; шаблон не вызывает вложенные slash-команды автоматически.

Опубликованный контракт включает:

1. Прочитать состояние проекта/сессии и определить задачу; при наличии настроенной памяти прочитать её.
2. Оформить сырую идею как Brief **Note**, затем выбрать глубину и создать/заполнить настоящие артефакты.
3. Standard+: ADI-доказательства, зафиксированная глубина, `/gate-check` перед реализацией; Deep+ — также декомпозиция и оценка.
4. Реализовать с claims и разделением файлов, выполнить проверки проекта, получить доказательства независимых ревьюеров.
5. Связать доказательства реализации; Deep+ — `/gate-check` после реализации.
6. Standard+: guardian проверяет всю цепочку доказательств; оркестратор выполняет review и activate, не автор и не guardian.
7. Коммит/PR — только в пределах разрешений; запись поставки — после merge. Deep+ `/wrap` сохраняет реальные результаты согласования в REFRESH.

`/gate-check` выдаёт вердикт по порогам, но не читает код и не заменяет guardian. Пользователь нужен также для неразрешённых требований, изменения глубины, обхода проверок и полномочий — не только необратимых решений. Ответственность и параллельная работа разобраны в [HOW-TO-USE](HOW-TO-USE.ru.md).

---

## Подводные камни (из реального dogfood)

### 1. EvidencePack без structured fields → R_eff = 0.1

Парсер читает отдельные строки полей, не строки таблиц и не примеры в комментариях. Без вердикта или объявления контекста он снижает доверие; без одного лишь типа выбирает measurement.

**Решение:** Всегда добавляй `## Structured Fields` секцию.

### 2. Все артефакты в Draft forever

Если не запускать `forgeplan review` → `forgeplan activate`, все артефакты остаются в Draft навсегда. Health dashboard будет показывать "ALL DRAFT".

**Решение:** После заполнения проведите review; активируйте, когда готовы доказательства, реализация и применимые проверки.

### 3. Validator требует секции которых нет в body

Body в LanceDB хранится БЕЗ frontmatter. Validator получает frontmatter из record fields (id, status, kind), а секции ищет в body. Если при создании через `forgeplan new` вы заполнили только Summary + FR — validator скажет "Missing Problem, Goals, Non-Goals".

**Решение:** Заполняйте все MUST секции для вашего depth. Или используйте aliases (Motivation вместо Problem, Out of Scope вместо Non-Goals).

### 4. `forgeplan update --body` принимает @filepath

```bash
forgeplan update PRD-001 --body @/tmp/new-body.md
```

Не нужно копировать контент в командную строку.

### 5. 10 типов артефактов, но реально нужны 6

Из dogfood опыта: PRD, RFC, ADR, Note, Problem, Epic — реально используются. EvidencePack, Spec, SolutionPortfolio, RefreshReport — для зрелых проектов с большим количеством артефактов.

Это историческое наблюдение dogfood, не современное разрешение пропускать доказательства. Evidence нужны и обычным Standard-решениям; опубликованная методология Deep/Critical добавляет SOL и REFRESH.

### 6. PRD-заглушки: "создал ID, забыл заполнить"

**Антипаттерн:** `forgeplan new prd "Title"` → сразу пишешь код → PRD остаётся stub навсегда.

Результат: `forgeplan validate` показывает 5 MUST errors, PRD нельзя activate, нет обоснования решения.

**Решение:** Shape → Validate → Code. После `forgeplan new` — СРАЗУ заполни MUST секции (Problem, Goals, Non-Goals, Target Users, Related). Запусти `forgeplan validate` и убедись что PASS. Только потом кодь.

### 7. Код готов, но нет Evidence → R_eff = 0.0

**Антипаттерн:** реализовал PRD полностью (200+ тестов), но не создал EvidencePack. Health кричит "blind spot", R_eff = 0.0.

**Решение:** Code → Evidence → Activate. После реализации:
```bash
forgeplan new evidence "Что подтверждено: тесты, LOC, dogfood"
# Добавь structured fields в body
forgeplan link EVID-XXX PRD-XXX --relation informs
forgeplan score PRD-XXX   # → R_eff > 0
forgeplan activate PRD-XXX
```

### 8. Active без кода = ложный статус

**Антипаттерн:** активировали PRD до начала реализации. Health не показывает проблем, но артефакт — пустое обещание.

**Решение:** В цикле реализации активируйте после готовности обещанного поведения и доказательств. Runtime проверяет структуру/связи, не факт запуска кода; отдельно проверенное решение без реализации должно быть явно ограничено таким объёмом.

---

## Ссылки

| Документ | Описание |
|----------|----------|
| [HOW-TO-USE](HOW-TO-USE.ru.md) | Практические инструкции |
| [DEPTH-CALIBRATION](DEPTH-CALIBRATION.ru.md) | Глубина и эскалация |
| [QUALITY-GATES](QUALITY-GATES.ru.md) | Машинные проверки и независимое ревью |
| [ARTIFACT-MODEL](ARTIFACT-MODEL.ru.md) | Иерархия артефактов |
| [PRD-RFC-ADR-FLOW](PRD-RFC-ADR-FLOW.ru.md) | Какой документ создавать |
| [GLOSSARY](GLOSSARY.ru.md) | Терминология |

## Зафиксированные источники

Сверено с **Forgeplan 0.38.0** (`d56466b10ddf788c62a6e8acffd0e0f578d0c1d1`) и публичным маркетплейсом **1.187.1** (`b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7`), не черновиками разработки:

- [CLI routing](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/route.rs#L15-L38) и [конвейер ядра](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/routing/pipeline.rs#L7-L19).
- [Правила валидации](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/validation/rules.rs#L353-L550), [проверки активации](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/lifecycle/mod.rs#L139-L220), [разбор Evidence](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/evidence.rs#L57-L204).
- [Опубликованная методология](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/skills/forgeplan-methodology/SKILL.md#L71-L103), [контракт цикла](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/commands/forge-cycle.md#L549-L573) и [передача guardian](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/commands/forge-cycle.md#L1096-L1168).
