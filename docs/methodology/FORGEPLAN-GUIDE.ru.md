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
forgeplan route "Добавить OAuth2 аутентификацию"
# → Depth: Deep, Pipeline: PRD → Spec → RFC → ADR

# 3. Создать первый артефакт
forgeplan new prd "OAuth2 Authentication"

# 4. Посмотреть состояние
forgeplan health
```

> **Alias**: `fpl` = `forgeplan`. Создайте symlink: `ln -s $(which forgeplan) /usr/local/bin/fpl`

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
forgeplan route "описание задачи"
```

Движок анализирует ключевые слова (security → Deep+, breaking change → Deep+, cross-team → Standard+) и выдаёт рекомендацию мгновенно, без LLM.

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

Если MUST пусто — activate. Если нет — доработай.

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
| `forgeplan update <id>` | Обновить метаданные/body | `forgeplan update PRD-001 --status active` |
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
| `forgeplan estimate <id> --grade mid` | Подсветить конкретный грейд | `forgeplan estimate PRD-022 --grade junior` |
| `forgeplan estimate <id> --my-grade` | Грейд из config grade_profile | `forgeplan estimate PRD-022 --my-grade` |

### Lifecycle

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan review <id>` | Чеклист: готов к активации? | `forgeplan review PRD-001` |
| `forgeplan activate <id>` | Draft → Active (validation gate) | `forgeplan activate PRD-001` |
| `forgeplan supersede <id> --by <new>` | Active → Superseded + chain warnings | `forgeplan supersede PRD-001 --by PRD-002` |
| `forgeplan deprecate <id> --reason "..."` | Active → Deprecated | `forgeplan deprecate PRD-001 --reason "Cancelled"` |

**Правило:** Notes и Problems не требуют validation gate. PRD, RFC, ADR, Epic, Spec — MUST rules должны пройти.

### Dashboards и аналитика

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan health` | Полное здоровье проекта | `forgeplan health --compact` |
| `forgeplan status` | Краткий dashboard | `forgeplan status` |
| `forgeplan blindspots` | Артефакты без evidence, orphans | `forgeplan blindspots` |
| `forgeplan journal` | Timeline решений с R_eff | `forgeplan journal --risk` |
| `forgeplan fpf` | FPF dashboard: contexts + F-G-R + actions | `forgeplan fpf` |
| `forgeplan stale` | Артефакты с expired valid_until | `forgeplan stale` |
| `forgeplan decay` | Impact expired evidence на R_eff | `forgeplan decay` |
| `forgeplan progress [id]` | Checkbox progress bars | `forgeplan progress` |

### Routing и calibration

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan route "<description>"` | Rule-based depth + pipeline (no LLM) | `forgeplan route "Add OAuth2"` |
| `forgeplan route "<desc>" --explain` | + LLM объяснение | `forgeplan route "Add OAuth2" --explain` |
| `forgeplan calibrate [id]` | Suggest depth для существующего артефакта | `forgeplan calibrate PRD-001` |

### AI-powered (требуют LLM config)

| Команда | Что делает | Пример |
|---------|-----------|--------|
| `forgeplan generate <kind> "<desc>"` | AI генерация артефакта | `forgeplan generate prd "Payment system"` |
| `forgeplan reason <id>` | ADI reasoning cycle | `forgeplan reason PRD-001 --json` |
| `forgeplan decompose <id>` | PRD → RFC задачи через AI | `forgeplan decompose PRD-001` |
| `forgeplan capture "<decision>"` | Записать решение как Note/ADR | `forgeplan capture "Use Redis for cache"` |
| `forgeplan search <query> --semantic` | Semantic vector search | `forgeplan search "auth" --semantic` |

### MCP Server

```bash
forgeplan serve  # запустить MCP server (stdio transport)
```

71 MCP tools — все команды выше доступны через MCP protocol.

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

EvidencePack **обязательно** должен содержать structured fields в body:

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

**Без этих полей** R_eff parser не найдёт данные и выставит CL0 → R_eff = 0.1 вместо 1.0.

### Привязать evidence к артефакту

```bash
forgeplan link EVID-001 ADR-002 --relation informs
forgeplan score ADR-002
# → R_eff = 1.00 (was 0.00)
```

### Congruence Levels (CL)

| CL | Penalty | Когда |
|----|---------|-------|
| CL3 | 0.0 | Evidence собрано на целевой системе (benchmark на нашем коде) |
| CL2 | 0.1 | Похожий контекст (benchmark другого проекта на таком же стеке) |
| CL1 | 0.4 | Другой контекст (статья, документация, чужой опыт) |
| CL0 | 0.9 | Противоположный контекст (evidence из другой domain) |

### R_eff = min(evidence_scores)

R_eff = weakest link. Если есть 3 evidence и одно слабое — R_eff = слабое. НЕ average.

---

## Validation — как проверять качество

### Правила по depth

| Depth | PRD rules | RFC rules | ADR rules |
|-------|-----------|-----------|-----------|
| Tactical | 3 base rules | 3 base | 3 base |
| Standard | 9 rules (+ audience, density, leakage) | 5 rules (+ options, phases) | 3 rules |
| Deep | 16 rules (+ timeline, stakeholders, risks, acceptance) | 6 rules (+ risks) | 5 rules (+ invariants, rollback) |

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
Active ──supersede──→ Superseded (link на замену)
Active ──deprecate──→ Deprecated (с причиной)
```

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

# 6. Активировал
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

71 MCP tools всего. 6 core покрывают 90% workflow.

---

## Forge Mode — permission model для AI агентов

При работе с AI агентами (Claude Code, Cursor) в автономном режиме используйте **Forge Mode** — модель разрешений с 3 зонами доверия (FPF B.3 Trust Calculus):

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

Команда `/forge-cycle PRD-XXX` запускает 8-фазный цикл:

```
Phase 0: OBSERVE    → forgeplan health + stale + fpf (что происходит?)
Phase 1: ROUTE      → forgeplan route (какой depth?)
Phase 2: SPRINT     → /sprint (план волн)
Phase 3: BUILD      → /team-up (реализация с Rust skills)
Phase 4: AUDIT      → /audit (adversarial review, MUST find issues)
Phase 5: FIXES      → /team-up (исправления по аудиту)
Phase 6: EVIDENCE   → forgeplan new evidence + score + activate
Phase 7: COMMIT     → git commit + PR + hindsight
Phase 8: NEXT       → forgeplan health → следующая фича
```

**FPF auto-resolve**: при конфликтах/выборах агент автоматически применяет ADI cycle (Abduction → Deduction → Induction) + WLNK + Reversibility check. Спрашивает пользователя только при необратимых решениях.

---

## Подводные камни (из реального dogfood)

### 1. EvidencePack без structured fields → R_eff = 0.1

Parser ищет `verdict:`, `congruence_level:`, `evidence_type:` в body как plain text. Без них — CL0 по умолчанию.

**Решение:** Всегда добавляй `## Structured Fields` секцию.

### 2. Все артефакты в Draft forever

Если не запускать `forgeplan review` → `forgeplan activate`, все артефакты остаются в Draft навсегда. Health dashboard будет показывать "ALL DRAFT".

**Решение:** После заполнения артефакта — сразу review + activate.

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

**Решение:** activate ТОЛЬКО когда код написан + evidence создан. Если PRD описывает будущую работу — оставь в draft.

---

## Ссылки

| Документ | Описание |
|----------|----------|
| `docs/guides/HOW-TO-USE.md` | 10 правил методологии с примерами |
| `docs/guides/DEPTH-CALIBRATION.md` | Подробно про 4 уровня depth + escalation |
| `docs/guides/QUALITY-GATES.md` | Verification Gate + Adversarial Review |
| `docs/guides/ARTIFACT-MODEL.md` | Иерархия артефактов: Epic → PRD → Spec → RFC → ADR |
| `docs/guides/PRD-RFC-ADR-FLOW.md` | Decision tree: какой документ создать |
| `docs/guides/GLOSSARY.md` | 31 термин |
| `CLAUDE.md` | Инструкции для AI агента + CLI quick reference |
