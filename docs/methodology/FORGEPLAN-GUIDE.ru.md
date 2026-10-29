[English](FORGEPLAN-GUIDE.md) · [Русский](FORGEPLAN-GUIDE.ru.md)

# Forgeplan — Полный практический гайд

> Один документ: методология + команды + примеры + подводные камни.
> Для человека и AI агента.

---

## Что такое Forgeplan

Forgeplan заставляет **думать перед кодингом**. Вместо "открыл IDE → написал код → задеплоил" получается "определил depth → создал артефакт → проверил качество → подтвердил evidence → закодил".

**Не Jira.** Не project management. Не task tracker. Forgeplan — это **structured knowledge base** для инженерных решений.

**Основной потребитель**: AI агент (Claude Code, Cursor) через MCP server. CLI — для human inspection.

---

## Установка

### 1. AI Skill (для любого AI агента)

```bash
# Установить /forge skill для Claude Code, Cursor, Codex, Gemini и 40+ агентов
npx skills add ForgePlan/forgeplan --skill forge
```

Skill установится в выбранные агенты. После этого в чате с AI:
```
/forge "Добавить OAuth2 аутентификацию"
```

### 2. CLI Binary

```bash
# macOS (Homebrew)
brew install forgeplan/tap/forgeplan
# Homebrew 6.0+: если отклонено с "untrusted tap", выполните `brew trust forgeplan/tap` один раз и повторите.

# Из checkout исходников (не рецепт установки из crates.io)
git clone https://github.com/ForgePlan/forgeplan.git && cd forgeplan
cargo install --path crates/forgeplan-cli --features semantic-search

# Или скачать binary из GitHub Releases
# https://github.com/ForgePlan/forgeplan/releases
```

Релизные бинарники включают семантический поиск; обычная Cargo-сборка выключает
feature по умолчанию. Веса отдельные: `forgeplan setup` готовит кэш BGE-M3.
Для офлайн-инференса нужен полный snapshot. Приоритет кэша:
`FORGEPLAN_MODEL_CACHE` → `~/.forgeplan/models` → platform/legacy fallback
при отсутствии home; `HF_HOME` не переопределяет кэш tract.
В действительно новом workspace `init -y --with-model` явно запрашивает
скачивание. Уже инициализированный workspace может завершить init раньше.

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

Используйте новый каталог вне существующего workspace. Обнаружение идёт вверх
по родителям: `init -y` может ничего не сделать, найдя родительский `.forgeplan/`.
Проверьте показанный путь до записи. Для клона с tracked артефактами см.
[указатель документации](../README.ru.md): локальный config и reindex.

```bash
# 1. Инициализировать workspace
forgeplan init

# 2. Определить что делать
forgeplan route "Добавить OAuth2 аутентификацию" --level 0
# Изучите рекомендацию; это не результат выполнения или приёмки.

# 3. Создать первый артефакт
forgeplan new prd "OAuth2 Authentication"

# 4. Посмотреть состояние
forgeplan health
```

> **Alias**: `fpl` = `forgeplan`. После source-install его создаёт `forgeplan setup --skip-model`.

---

## Методология: 3 вопроса вместо бюрократии

### Вопрос 1: "Какой depth?"

Задай себе один вопрос: **"Это обратимо за день?"**

| Ответ | Depth | Что создавать | Пример |
|-------|-------|--------------|--------|
| Да, тривиально | **Tactical** | Ничего или Note | Fix typo, update config |
| Нет, есть выбор | **Standard** | PRD → RFC | Новая фича, 1-3 дня |
| Нет, затрагивает многих | **Deep** | PRD → Spec → RFC → ADR | Новый модуль, 1-2 недели |
| Стратегия, кросс-команда | **Critical** | Epic → PRD[] → RFC[] → ADR[] | Новая подсистема |

Или используй автоматический routing:

```bash
forgeplan route "описание задачи" --level 0
```

`--level 0` явно выбирает keyword routing без LLM. Обычный `route` может попытаться вызвать настроенную LLM и дальнейший анализ; отсутствие `--explain` не гарантирует офлайн-работу.

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

Если MUST findings остались, доработайте документ. Их отсутствие необходимо
для обычной gated activation, но недостаточно: активация отдельно проверяет
длину, evidence и заглушки. Review не меняет status.

### Главное правило

**Pipeline = guideline, НЕ бюрократия.** Не создавай все 10 типов на каждую задачу. Tactical depth = просто делай. Standard = PRD + RFC. Только Deep+ требует полный pipeline.

---

## Все команды (по категориям)

### Раскладка workspace и доступность интерфейсов

Примеры команд ниже не доказывают универсальную совместимость CLI/MCP.
Оговорки Constellation относятся к scoped-сборке `0.39.0+constellation.s1.1`,
не к опубликованной `0.38.0`.

| Операция | Совместимый legacy layout | Migrated Constellation layout |
|---|---|---|
| CLI `validate` | Успешно проверен | Успешно проверен |
| Full MCP `forgeplan_validate` | Интерфейс валидации | Успешно упражнён на migrated артефактах |
| Остальные зарегистрированные full tools | Проверять контракт | Не выводить поддержку из регистрации |

См. [пользовательский гайд](https://forgeplan.dev/ru/docs/guides/constellation/),
[справочник интерфейсов](https://forgeplan.dev/ru/docs/reference/constellation/)
и [переносимый пилот](../operations/CONSTELLATION-PILOT.ru.md).

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
| `forgeplan estimate <id> --grade middle` | Подсветить конкретный грейд | `forgeplan estimate PRD-022 --grade junior` |
| `forgeplan estimate <id> --my-grade` | Грейд из config grade_profile | `forgeplan estimate PRD-022 --my-grade` |

### Lifecycle

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan review <id>` | Чеклист: готов к активации? | `forgeplan review PRD-001` |
| `forgeplan activate <id>` | Draft → Active (validation gate) | `forgeplan activate PRD-001` |
| `forgeplan supersede <id> --by <new>` | Active → Superseded + chain warnings | `forgeplan supersede PRD-001 --by PRD-002` |
| `forgeplan deprecate <id> --reason "..."` | Active → Deprecated | `forgeplan deprecate PRD-001 --reason "Cancelled"` |

**Правило:** PRD, RFC, ADR, Epic и Spec имеют validation gates. Notes и Problems
не используют тот же schema gate; дополнительные lifecycle checks всё равно
зависят от типа и глубины. Успешный `validate` не равен готовности к активации.

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
| `forgeplan route "<description>" --level 0` | Keyword-only depth + pipeline, без LLM | `forgeplan route "Add OAuth2" --level 0` |
| `forgeplan route "<desc>" --explain` | + LLM объяснение | `forgeplan route "Add OAuth2" --explain` |
| `forgeplan calibrate [id]` | Suggest depth для существующего артефакта | `forgeplan calibrate PRD-001` |

### AI-powered (требуют LLM config)

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan generate <kind> "<desc>"` | AI генерация артефакта | `forgeplan generate prd "Payment system"` |
| `forgeplan reason <id>` | ADI reasoning cycle | `forgeplan reason PRD-001 --json` |
| `forgeplan decompose <id>` | PRD → RFC задачи через AI | `forgeplan decompose PRD-001` |
| `forgeplan capture "<decision>"` | Записать решение как Note/ADR | `forgeplan capture "Use Redis for cache"` |

### Локальный semantic search (LLM provider не нужен)

`forgeplan search "auth" --semantic` требует сборку с `semantic-search`
и полный совместимый кэш модели, а не `llm` config. На migrated layout нужны
также опубликованный embedding profile и eligible vectors; forced semantic
ошибки не означают обещанный keyword fallback. Выполните явно `index`, затем
`embed`. Для автоматического drain дополнительно нужны активный
`config.embedding` (`model: bge-m3`) и live maintenance; ручной `embed`
эту настройку не сохраняет.

### MCP Server

```bash
forgeplan serve  # запустить MCP server (stdio transport)
```

В scoped Constellation-сборке зарегистрированы 76 full tools и отдельные 9 tools
профиля `constellation-read`. `full` — default, с сохранением записи.
Startup maintenance и request workspace выбираются по-разному. `--workspace`
задаёт server default, но явный binding `FORGEPLAN_WORKSPACE` приоритетнее для
startup maintenance. Request предпочитает явный workspace, затем environment,
server default и cwd. Read-profile handle не запускает maintenance и не
принимает произвольные пути.

---

## Estimate Engine — оценка трудозатрат

### Зачем

Превращает документацию (FR в PRD, Phases в RFC) в эстимейты трудозатрат. Не нужна отдельная Excel-таблица — estimate живёт рядом с артефактами.

### Базовая команда

```bash
forgeplan estimate PRD-022
```

Иллюстративный estimate output (эвристические часы, не измеренные длительности):

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

**Confidence** — эвристика полноты, не измеренная точность прогноза. Baseline 10%, затем:
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

Заполняйте structured fields и реальные наблюдения. Числа ниже **иллюстративны**, это не выполненный benchmark; не копируйте их как evidence:

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

Missing/unknown verdict или отсутствие CL declaration приводит к CL0; plain undeclared pack получает около 0.1. `evidence_type` отдельно имеет default measurement. Реальный R_eff зависит также от expiry, прочих evidence и зависимостей; следующий пример одного benchmark не гарантирует 1.00 в recursive report (benchmark modifier 0.1).

### Привязать evidence к артефакту

```bash
forgeplan link EVID-001 ADR-002 --relation informs
forgeplan score ADR-002
# Изучите фактический report: score, weakest link, expiry и dependency factors.
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

---

## Validation — как проверять качество

### Правила по depth

| Depth | PRD rules | RFC rules | ADR rules |
|-------|-----------|-----------|-----------|
| Tactical | Core PRD sections и shared checks | Summary, motivation, direction | Context, decision, consequences |
| Standard | + audience; density/leakage SHOULD | + alternatives и phases | Core sections |
| Deep | + timeline, stakeholders, risks, acceptance, metrics | + risks | + invariants и rollback |

Это summary секций, не фиксированное число rules. Shared rules и дополнительные
measurability/NFR/traceability checks зарегистрированы в `validation/rules.rs`.

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

- **MUST** — обязательные реализованные schema checks; обычно блокируют gated activation.
- **SHOULD** — предупреждения; severity плотности зависит от глубины (50 слов: Standard SHOULD, Deep MUST).
- **COULD** — совет, включая `adversarial-too-clean` в опциональных adversarial checks.

Рекомендации автору шире исполняемых проверок: наличие секции не доказывает
измеримость, причинность и согласованность. Проверка наличия `id`/`status`
сама по себе не подтверждает допустимость всех lifecycle values.

---

## Lifecycle — от Draft до Active

```text
draft --activate--> active
active --> superseded | deprecated | stale
stale --renew--> active
stale --reopen--> deprecated + new draft
stale --deprecate--> deprecated
```

`superseded` и `deprecated` — terminal. `review` status не меняет.
Approved, Implemented и Closed — описания процесса, не значения сохраняемого
status. Session phases и маркеры artifact phase учитываются отдельно.
Активация включает length/evidence/stub gates сверх валидации.

### Типичный flow

Иллюстративная последовательность для legacy layout, не наблюдённый transcript.
Перед последним шагом заполните содержимое нужной глубины и реальные evidence,
которые требует активация. Для migrated layout выбирайте интерфейс по таблице выше.

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
# Изучить findings и activation checklist; не подставлять выдуманный PASS.

# 6. Активировал
forgeplan activate PRD-001
# Успех возможен только при прохождении всех применимых activation gates.
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
# Установить skill для всех поддерживаемых агентов
npx skills add ForgePlan/forgeplan --skill forge
```

Поддерживается 40+ агентов: Claude Code, Cursor, Codex, Gemini CLI, GitHub Copilot, Cline, Continue, Windsurf, и другие.

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

Selected build содержит 76 full-profile tools и 9 отдельных read-profile tools. Эти шесть — учебная подборка, не измеренное покрытие 90% workflow и не приёмка всех зарегистрированных tools.

---

## Forge Mode — permission model для AI агентов

**Forge Mode** — локальная конвенция разрешений, не upstream trust algorithm FPF B.3. Config ниже иллюстративен; реальные permissions и загрузку hooks проверяют в host. Он не устанавливает controls и не отменяет ограничения конкретного задания.

| Зона | Что | Режим | Примеры |
|------|-----|-------|---------|
| **Green** | Read-only + build + test + forgeplan | Авто-разрешено | `cargo test`, `forgeplan health`, `git status` |
| **Yellow** | Создание/редакция файлов, git add/commit | Авто-разрешено (acceptEdits) | Write, Edit, `git add`, `git commit` |
| **Red** | Необратимые действия | **BLOCKED** | `git push --force`, `rm -rf /`, `cargo publish` |

### Настройка (Claude Code)

1. **Whitelist** в `settings.local.json` — wildcard patterns:
```json
{
  "permissions": {
    "allow": [
      "Bash(cargo:*)", "Bash(forgeplan:*)", "Bash(git:*)",
      "Bash(ls:*)", "Bash(find:*)", "Bash(grep:*)",
      "mcp__hindsight__memory_recall", "mcp__hindsight__memory_retain"
    ]
  }
}
```

2. **Safety hook** в `.claude/hooks/forge-safety-hook.sh` — PreToolUse blacklist:
```bash
# Blocked даже в yolo mode:
# git push --force, git reset --hard, rm -rf /, cargo publish
```

3. **Режим Claude Code**: `acceptEdits` (файлы авто, bash через whitelist)

### /forge-cycle — полный FPF-aligned dev cycle

Native prompt `/forge-cycle PRD-XXX` описывает девять numbered phases (0–8):

```
Phase 0: OBSERVE    → forgeplan health + stale + fpf dashboard (что происходит?)
Phase 1: ROUTE      → forgeplan route (какой depth?)
Phase 2: SPRINT     → /sprint (план волн)
Phase 3: BUILD      → /team-up (реализация с Rust skills)
Phase 4: AUDIT      → /audit (adversarial review, MUST find issues)
Phase 5: FIXES      → /team-up (исправления по аудиту)
Phase 6: EVIDENCE   → forgeplan new evidence + score + activate
Phase 7: COMMIT     → git commit + PR + hindsight
Phase 8: NEXT       → forgeplan health → следующая фича
```

Native AUDIT сохраняет квоту замечаний; она не доказывает наличие дефектов.
Этот документ policy не меняет и G3 не закрывает.

**Локальная auto-resolve guidance** использует ADI + weakest-link + обратимость. Native Build stop conditions включают также одинаково слабые варианты и влияние на другие PRD/сквозные вопросы, не только необратимость. Три альтернативы — локальная prompt-эвристика, не универсальный upstream-порог FPF.

---

## Подводные камни (из реального dogfood)

### 1. EvidencePack без structured fields → нет надёжного evidence signal

Parser ищет `verdict:`, `congruence_level:`, `evidence_type:` в body как plain text. Без них congruence level не определён; конкретный R_eff зависит от того, оценивается ли pack как leaf, через другой артефакт или при отсутствии eligible evidence.

**Решение:** Всегда добавляй `## Structured Fields` секцию.

### 2. Все артефакты в Draft forever

Если не запускать `forgeplan review` → `forgeplan activate`, все артефакты остаются в Draft навсегда. Health dashboard будет показывать "ALL DRAFT".

**Решение:** Проверьте заполненный артефакт, устраните применимые gates и активируйте только готовый документ.

### 3. Validator требует секции которых нет в body

Контракт body зависит от интерфейса. Текущий captured `ArtifactRecord`
обычного чтения сохраняет весь Markdown с frontmatter; CLI `get --json`
возвращает его в `body`. Full MCP pinned `forgeplan_constellation_get`
возвращает прозу во вложенном `artifact.body`. Эти payload не взаимозаменяемы.
Валидатору всё равно нужны обязательные секции: шаблон только с Summary + FR
остаётся неполным.

**Решение:** Заполняйте все MUST секции для вашего depth. Или используйте aliases (Motivation вместо Problem, Out of Scope вместо Non-Goals).

### 4. `forgeplan update --body` принимает @filepath

```bash
forgeplan update PRD-001 --body @/tmp/new-body.md
```

Не нужно копировать контент в командную строку.

### 5. 10 типов артефактов, но реально нужны 6

Десять типов здесь — вводная подборка, не полная core taxonomy. Evidence нужны по применимым gates, не только зрелым проектам. Rust core поддерживает также Memory и шесть brownfield extraction kinds; CLI/MCP schemas имеют собственные subsets.

### 6. PRD-заглушки: "создал ID, забыл заполнить"

**Антипаттерн:** `forgeplan new prd "Title"` → сразу пишешь код → PRD остаётся stub навсегда.

Результат: validation может найти пропущенные секции/placeholders; точный набор findings зависит от глубины и содержимого. Stub не готов к активации.

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

**Решение:** activate ТОЛЬКО когда код написан + evidence создан. Если PRD описывает будущую работу — оставь в draft.

---

## Ссылки

| Документ | Описание |
|----------|----------|
| [HOW-TO-USE.ru.md](HOW-TO-USE.ru.md) | Правила методологии с примерами |
| [DEPTH-CALIBRATION.ru.md](DEPTH-CALIBRATION.ru.md) | Глубина и escalation |
| [QUALITY-GATES.ru.md](QUALITY-GATES.ru.md) | Verification и adversarial review |
| [ARTIFACT-MODEL.ru.md](ARTIFACT-MODEL.ru.md) | Иерархия артефактов |
| [PRD-RFC-ADR-FLOW.ru.md](PRD-RFC-ADR-FLOW.ru.md) | Какой документ создавать |
| [GLOSSARY.ru.md](GLOSSARY.ru.md) | Термины и различия upstream/product |
| `CLAUDE.md` | Инструкции для AI агента + CLI quick reference |
