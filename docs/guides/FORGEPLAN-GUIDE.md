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

26 MCP tools — все команды выше доступны через MCP protocol.

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

28 MCP tools всего. 6 core покрывают 90% workflow.

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
