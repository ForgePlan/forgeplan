# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Что это за проект

**Forgeplan** — универсальная Rust-платформа (CLI + Desktop App) для ведения любого проекта от идеи до реализации через структурированные артефакты с quality scoring, semantic search и evidence tracking.

**Формула**:
```
Forgeplan = Quint-code (decision engine, R_eff scoring, evidence decay)
          + BMAD (PRD workflow, 13-step validation, adversarial review)
          + OpenSpec (artifact DAG, delta-specs, custom schemas)
          + FPF (reasoning framework, ADI cycle, trust calculus)
          + git-adr (Rust CLI patterns, clap, templates)
          + LanceDB (embedded DB: tables + vectors в одном)
          + Tauri (desktop app: React UI + shared Rust core)
```

**CLI**: `forgeplan` (alias: `fpl`)
**Desktop**: Tauri 2.0 + React (shared Rust core)
**Язык документации**: русский. Код: Rust с английскими идентификаторами.

## Текущий статус

- **v0.17.0** released — **EPIC-003 complete** (Search, Discovery, Intelligence)
- **~56 CLI команд**, **~47 MCP tools**, **1109 тестов** (+280 от v0.16)
- **0 warnings** на обоих feature configs (default + `semantic-search`)
- EPIC-001 (foundation) ✅ | EPIC-002 (v2.0 vision) ✅ | **EPIC-003 (v0.17.0)** ✅
- **7 PRDs активированы** в v0.17.0: PRD-035 (tags + discover), PRD-039 (BM25 search),
  PRD-040 (scoring intelligence), PRD-041 (FPF rules), PRD-042 (FPF KB vector search,
  supersedes PRD-018), PRD-043 (methodology integrity), PRD-044 (не используется)
- **NOTE-044** (Sprint Checklist Framework) + **NOTE-045** (deferred debts) как
  reusable quality gates для будущих спринтов
- **FPF KB** поддерживает semantic search через BGE-M3 (feature-gated, graceful fallback)
- **Phase 5** (Desktop App, Tauri) — backlog

Подробности: `TODO.md` (текущие приоритеты), `CHANGELOG.md` (история релизов).

## Как начать работу в новом чате

1. **Прочитай этот файл** — CLAUDE.md содержит CLI workflow, методологию, git-конвенции
2. **`forgeplan health`** — понять текущее состояние проекта (artifacts, blind spots, next actions)
3. **Для текущих задач** — `TODO.md`
4. **Полный гайд по CLI и методологии** — `docs/methodology/FORGEPLAN-GUIDE.md`
5. **Для reference code** — `sources/` (read-only repos, см. таблицу ниже)
6. **Используй Hindsight** — `memory_recall("Forgeplan")` для быстрого восстановления контекста

### ОБЯЗАТЕЛЬНО перед работой над задачей:

```bash
forgeplan route "описание задачи"   # определи depth и pipeline
```

Если route говорит Standard+ → создай артефакт ПЕРЕД кодингом. Если Tactical → просто делай.

### ОБЯЗАТЕЛЬНО при создании артефакта (Shape → Validate → ADI → Code):

1. **`forgeplan new prd "Title"`** — создаёт stub из шаблона
2. **СРАЗУ заполни ВСЕ MUST секции** — Problem, Goals, Non-Goals, Target Users, Related, FR
3. **`forgeplan validate PRD-XXX`** — убедись что PASS (0 MUST errors)
4. **ADI reasoning** (для Standard+ depth):
   ```bash
   forgeplan reason PRD-XXX           # 3+ гипотезы, justified confidence
   ```
   - Прочитай hypotheses — есть ли лучший подход чем первая мысль?
   - Если все гипотезы сходятся → уверенно кодь
   - Если есть конкурирующие подходы → обсуди с пользователем перед кодом
   - Для Deep/Critical: ADI **ОБЯЗАТЕЛЕН**, нельзя пропускать
   - Для Tactical: пропускай ADI
5. **Только ПОСЛЕ validate PASS + ADI** — начинай писать код

**НЕ оставляй PRD-заглушки.** Stub PRD без Problem/Goals = "решение без обоснования".

### ОБЯЗАТЕЛЬНО после реализации (Code → Evidence → Activate):

1. **Создай EvidencePack** с фактами (тесты, LOC, dogfood результаты):
   ```bash
   forgeplan new evidence "Описание что подтверждено"
   # Добавь в body: verdict: supports, congruence_level: 3, evidence_type: test
   forgeplan link EVID-XXX PRD-XXX --relation informs
   ```
2. **Проверь R_eff** — `forgeplan score PRD-XXX` → должен быть > 0
3. **Review и activate** — `forgeplan review PRD-XXX` → `forgeplan activate PRD-XXX`
4. **Обнови прогресс** — чекбоксы FR `[x]` в PRD/RFC

**Работа не закончена, пока: PRD заполнен + validate PASS + ADI (для Standard+) + evidence создан + R_eff > 0 + activated.**

> **ПОЛНЫЙ ЦИКЛ (Standard+ depth) — не пропускай шаги:**
> ```
> 1. Session Start: memory_recall → forgeplan health → orch query
> 2. Route: forgeplan route "задача" → определить depth
> 3. Shape: forgeplan new prd → заполнить MUST секции
> 4. Validate: forgeplan validate → PASS
> 5. ADI: forgeplan reason → 3+ гипотезы (Deep/Critical: ОБЯЗАТЕЛЕН)
> 6. Branch: git checkout -b feat/xxx
> 7. Code: реализация + тест на каждую pub fn
> 8. Test: cargo test → 0 failures
> 9. Fmt: cargo fmt → cargo fmt --check = 0 diffs
> 10. Lint: cargo check → 0 warnings
> 11. Audit: /audit (2+ агента) → Fix all HIGH/CRITICAL
> 12. Evidence: forgeplan new evidence → link → score (R_eff > 0)
> 13. Activate: forgeplan activate
> 14. PR: git push → gh pr create --base dev
> 15. Merge: gh pr merge (merge commit, НЕ squash)
> 16. Sync: orch task → Done + memory_retain в Hindsight
> 17. Progress: TODO.md + RFC/PRD чекбоксы
> ```
> **Tactical depth**: Route → Branch → Code → Test → Fmt → Lint → Commit. Без артефакта, ADI, evidence, PR.

### ОБЯЗАТЕЛЬНО smoke test после каждого спринта:

```bash
# 0. Format + Lint
cargo fmt                               # Форматирование
cargo fmt -- --check                    # Проверка: 0 diffs
cargo check                             # Компиляция: 0 warnings, 0 errors

# 1. Unit tests
cargo test                              # ВСЕ должны PASS

# 2. Workspace init (AI всегда использует -y!)
forgeplan init -y                       # НИКОГДА без -y в AI контексте

# 3. Core operations
forgeplan new prd "Smoke Test"          # Создание артефакта
forgeplan validate PRD-XXX              # Валидация работает
forgeplan score PRD-XXX                 # F-G-R scoring работает

# 4. Новые фичи (PRD-016+)
forgeplan blocked                       # Граф зависимостей
forgeplan order                         # Topological sort

# 5. FPF Knowledge Base (PRD-021)
forgeplan fpf ingest                    # 204 секции загружены
forgeplan fpf search "trust"            # Поиск находит B.3

# 6. LLM integration
GEMINI_API_KEY=<key> forgeplan reason PRD-XXX --fpf  # ADI + FPF context
```

**Если любой шаг fail — НЕ коммитить. Починить сначала.**

### ВАЖНО для AI агентов:
- **`forgeplan init`** — ВСЕГДА с `-y` флагом (без interactive prompt)
- **Config после init** — проверить `.forgeplan/config.yaml`, настроить LLM provider
- **`.forgeplan/` в gitignore** — workspace данные НЕ трекаются, config теряется при reinit
- **LanceDB migration** — новые columns требуют reinit workspace (`rm -rf .forgeplan && forgeplan init -y`)

### ОБЯЗАТЕЛЬНО при написании Rust кода:

1. **Перед сложными паттернами** — активируй Rust skills:
   - `Skill("rust-expert")` — ownership, lifetimes, async, error handling
   - `Skill("m01-ownership")` — borrow checker issues
   - `Skill("m06-error-handling")` — Result, Option, anyhow patterns
   - `Skill("m07-concurrency")` — async/Send/Sync issues
2. **Каждая новая `pub fn` = тест сразу** — НЕ переходи к следующей функции без теста. Hook `commit-test-check.sh` блокирует коммит без тестов.
3. **После написания кода** — `cargo fmt` + `cargo test` обязательны. Не коммить если тесты fail или fmt dirty
4. **Перед коммитом** — `cargo fmt` (форматирование) + `cargo check` (линтинг). Hook `pre-commit-fmt.sh` блокирует коммит без форматирования
5. **После значительных изменений** — `/audit` с Rust skills (минимум 2 агента)
5. **Используй `/fpf-simple`** для архитектурных решений и trade-off анализа
6. **Используй `/forge`** для structured workflow (route → create → validate → code)

### ОБЯЗАТЕЛЬНО ��а session start (Unified Workflow Protocol):

```bash
# 1. Память — восстановить контекст
memory_recall("Forgeplan")              # Hindsight: что было в прошлых сессиях

# 2. Методология — состояние проекта
forgeplan health                        # Blind spots, orphans, stale

# 3. Задачи — что в работе (если Orchestra доступна)
# mcp__orch__query_entities(type: "task", status: "in_progress")

# 4. Синтез — определить следующее действие
```

Если health показывает **blind spots** (active без evidence) или **orphans** (без связей) — **FIX ИХ ПЕРВЫМИ**, до начала новой работы. Не копи долг.

### ОБЯЗАТЕЛЬНО: Unified Workflow (Forgeplan × Orchestra × Hindsight)

Три системы работают как одна:
- **Forgeplan** = ЧТО делать и ПОЧЕМУ (артефакты, quality, evidence)
- **Orchestra** = КТО делает и КОГДА (задачи, сроки, назначения)
- **Hindsight** = ПАМЯТЬ (контекст между сессиями)

**Правила синхронизации:**
1. Новый артефакт (PRD/RFC/PROB) → создать task в Orchestra (если доступна)
2. `forgeplan activate` → mark task Done в Orchestra
3. PR merged → обновить Orchestra task + `memory_retain` в Hindsight
4. Конец спринта → `memory_retain` с итогами в Hindsight
5. Если Orchestra недоступна — записать в TODO.md что нужно синхронизи��овать
6. **Brownfield**: если много артефактов завершено до подключения Orchestra — создать одну milestone задачу `[EPIC-XXX] Title — N artifacts completed (pre-Orchestra)` вместо N отдельных Done-задач. Установить Phase=Done, Status=Done, Sprint="Sprint 1-N"

**Правила создания задач в Orchestra:**

Naming:
- С артефактом: `[ARTIFACT-ID] описание` — `[PRD-019] MCP session state machine`
- Bug без артефакта: описание + Tags: Bug — `Embed feature fix — fastembed API v5`
- Feature без артефакта: описание + Tags: Feature — `Distribution — brew, GH Actions`

Fields (обязательные):
- **Status** — Backlog / To Do / Doing / Review / Done
- **Phase** — Shape / Validate / Code / Evidence / Done (маппинг: Backlog=Shape, Doing=Code, Done=Done)
- **Depth** — Tactical / Standard / Deep / Critical (из `forgeplan route`)
- **Artifact** — ID артефакта (только если есть: `PRD-019`, `PROB-021`)
- **Type** — тип артефакта (только если есть Artifact: PRD / RFC / ADR / Problem / Evidence)
- **Sprint** — текущий спринт (проставлять при взятии в работу)
- **Branch** — git branch (проставлять при создании ветки)
- **Tags** — Bug / Feature / Docs / Update (для задач без артефакта)

**Полный гайд**: `docs/methodology/UNIFIED-WORKFLOW.md`

## Как пользоваться Forgeplan CLI (MCP-first)

> Forgeplan — MCP-first tool. Основной потребитель = AI агент через MCP server.
> CLI = secondary interface для human inspection.

### Core workflow (6 шагов):

```bash
# 1. Session start — понять состояние проекта
forgeplan health

# 2. Перед работой — определить depth и pipeline
forgeplan route "описание задачи"
# → Depth: Standard, Pipeline: PRD → RFC, Confidence: 85%

# 3. Создать артефакт
forgeplan new prd "Auth System"

# 4. Проверить качество
forgeplan validate PRD-001
# → MUST: Missing Problem section
# → SHOULD: density < 50 words

# 5. Когда готов — review и activate
forgeplan review PRD-001
# → Review PASSED — ready to activate
forgeplan activate PRD-001
# → draft → active

# 6. Подтвердить решение evidence
forgeplan new evidence "Benchmark results for auth approach"
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan score PRD-001
# → R_eff = 1.00 (was 0.00)
```

### EvidencePack — как создавать (ВАЖНО):

EvidencePack ОБЯЗАТЕЛЬНО должен содержать structured fields в body для корректного R_eff scoring:

```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
```

| Field | Значения | Описание |
|-------|----------|----------|
| `verdict` | supports / weakens / refutes | Подтверждает, ослабляет или опровергает решение |
| `congruence_level` | 0-3 | CL3=same context (best), CL0=opposed context (worst) |
| `evidence_type` | measurement / test / benchmark / audit | Тип доказательства |

Без structured fields R_eff parser не найдёт данные и выставит CL0 (penalty 0.9).

### Lifecycle commands (ADR-005 v2):

```bash
forgeplan review <id>              # проверить готовность
forgeplan activate <id>            # draft → active (validation gate)
forgeplan supersede <id> --by <new> # active → superseded (TERMINAL)
forgeplan deprecate <id> --reason "..." # active/stale → deprecated (TERMINAL)
forgeplan renew <id> --reason --until  # stale → active (extend validity)
forgeplan reopen <id> --reason         # stale/active → deprecated + NEW draft (lineage)
```

State machine:
```
draft → active → superseded (terminal)
               → deprecated (terminal)
               → stale → active (renew)
                       → deprecated + NEW draft (reopen)
```

**Terminal**: deprecated и superseded — никогда не переходят в другие статусы.
**Stale**: артефакт устарел (valid_until expired). `renew` продлевает, `reopen` создаёт новый.

Notes и Problems не требуют validation gate для activation.
PRD, RFC, ADR, Epic, Spec — MUST rules должны пройти.

### Validator aliases:

Validator принимает синонимы для секций:
- `## Problem` = `## Motivation` = `## Problem Statement` = `## Background`
- `## Goals` = `## Success Criteria` = `## Objectives`
- `## Non-Goals` = `## Out of Scope` = `## Product Scope`
- `## Related` = `## Related Artifacts` = `## Dependencies`
- `## Target Users` = `## Target Audience` = `## Users`

### Dogfood insights (из реального использования):

1. **Shape → Validate → ADI → Code → Evidence → Activate** — полный цикл, не пропускай шаги. ADI обязателен для Standard+ depth
2. **Создавай артефакт → СРАЗУ заполняй MUST секции** — stub PRD = долг, который копится
3. **Evidence делает R_eff живым** — без evidence все scores = 0.0, health кричит "blind spot"
4. **Не активируй без кода** — active PRD без реализации = ложное обещание
5. **Не создавай все 10 типов** — реально используются 6: PRD, RFC, ADR, Note, Problem, Epic
6. **route перед работой** — определяет depth и pipeline, экономит время
7. **health на session start** — показывает orphans, blind spots; **fix их первыми**
8. **Работа не закончена пока**: PRD заполнен + validate PASS + ADI (Standard+) + evidence создан + R_eff > 0 + activated

## Как пользоваться методологией (quick reference)

> Полный гайд: `docs/methodology/HOW-TO-USE.md`

### Routing — один вопрос определяет depth:
```
Тривиально, обратимо за день?  → Tactical: ничего или Note (без ADI)
Фича 1-3 дня, есть выбор?      → Standard: Brief/PRD → RFC (ADI рекомендуется)
Необратимо, 1-2 недели?        → Deep: PRD → Spec → RFC → ADR (ADI ОБЯЗАТЕЛЕН)
Кросс-команда, стратегия?       → Critical: Epic → PRD[] → Spec[] → RFC[] → ADR[] (ADI + review)
```

### 5 артефактов = 5 вопросов:
| Вопрос | Артефакт | Когда НЕ нужен |
|--------|----------|----------------|
| ЧТО и зачем? | PRD / Brief | Баг-фикс, рефакторинг |
| КАК ТОЧНО работает? | Spec | Нет API / data model changes |
| КАК СТРОИМ? | RFC | Архитектура очевидна, <1 дня |
| ПОЧЕМУ именно это? | ADR | Решение тривиально и обратимо |
| ГРУППИРОВКА? | Epic | Задача = один PRD |

### Правила:
- **Pipeline = guideline, НЕ бюрократия** — не создавай все 10 типов на каждую задачу
- **[Actor] can [capability]** — формат FR, без технологий в требованиях
- **Ребёнок ссылается на родителя** — PRD→Epic, RFC→PRD, ADR→RFC
- **Supersede, не удаляй** — старый артефакт получает status: Superseded
- **Quality gates по depth** — tactical: ничего, standard: Verification Gate, deep+: Adversarial Review

### Progress Tracking (ОБЯЗАТЕЛЬНО):
После завершения блока работ (реализация FR, закрытие фазы, создание артефакта) — **предложи пользователю обновить прогресс** в следующих местах:
1. **RFC** — чекбоксы Implementation Phases (`- [ ]` → `- [x]`) + progress bar
2. **PRD** — progress bar по FR (сколько FR реализовано)
3. **Epic** — Children таблица (progress %), aggregated progress bar
4. **PLAN.md** — Phase progress bar + чекбоксы задач
5. **TODO.md** — переместить завершённые задачи в Done ✅, обновить P0

Формула: **работа не закончена, пока прогресс не отражён в артефактах.**

### Forge Mode (permission model)

**Три зоны доверия** (FPF B.3 Trust Calculus applied to CLI permissions):

| Зона | Что | Режим | Примеры |
|------|-----|-------|---------|
| **Green** | Read-only + build + test + forgeplan | Авто-разрешено | `cargo test`, `forgeplan health`, `git status` |
| **Yellow** | Файлы + git add/commit | Авто-разрешено (acceptEdits) | Write, Edit, `git add`, `git commit` |
| **Red** | Необратимые действия | **BLOCKED hook** | `git push --force`, `rm -rf /`, `cargo publish` |

**Настройка:**
- `settings.local.json` — whitelist permissions (wildcard patterns: `Bash(cargo:*)`, `Bash(git:*)`)
- `.claude/hooks/forge-safety-hook.sh` — PreToolUse blacklist (blocked patterns)
- Режим Claude Code: `acceptEdits` (файлы авто, bash через whitelist)

**Blacklisted commands** (blocked даже в yolo mode):
- `git push --force` / `git push -f`
- `git reset --hard`
- `git clean -fd`
- `rm -rf /` / `rm -rf ~`
- `cargo publish` (explicit manual action)
- `DROP TABLE`

**Команда `/forge-cycle`** — полный FPF-aligned цикл: Observe → Route → Shape → Sprint → Build → Audit → Fix → Evidence → Commit → PR → Activate.

### Git-конвенции

#### Формат коммита (Conventional Commits + Forgeplan):
```
<type>(<scope>): <description>

[body — что и почему, на русском]

Refs: RFC-001, FR-001..004
```

#### Types:
| Type | Когда | Пример |
|------|-------|--------|
| `feat` | Новая функциональность (FR-*) | `feat(cli): implement forgeplan init` |
| `docs` | Артефакты методологии (RFC, PRD, ADR) | `docs(rfc): add RFC-001 CLI architecture` |
| `fix` | Баг-фикс | `fix(frontmatter): handle missing closing ---` |
| `refactor` | Рефакторинг без изменения поведения | `refactor(store): extract slugify` |
| `test` | Тесты | `test(workspace): add init roundtrip tests` |
| `chore` | Build, deps, CI | `chore(deps): add tempfile dev-dependency` |
| `progress` | Обновление прогресса артефактов | `progress: update Phase 3A tracking` |

#### Scope = модуль или артефакт:
- Код: `cli`, `core`, `store`, `template`, `scoring`, `workspace`, `config`
- Артефакты: `rfc`, `prd`, `adr`, `epic`

#### Branching Strategy (dev-based):
```
main                              ← production (tagged releases: v0.8.0, v0.9.0)
  │
dev                               ← integration branch (all features merge here)
  ├── feat/prd-018-openspec-dag   ← feature branch (from dev)
  ├── fix/search-ranking          ← bugfix branch (from dev)
  └── docs/rfc-002-lancedb       ← docs-only branch (from dev)
  │
release/v0.9.0                    ← release candidate (from dev → main)
```

| Ветка | Создаётся из | Мерджится в | Стратегия |
|-------|-------------|-------------|-----------|
| `feat/*`, `fix/*`, `docs/*` | **dev** | **dev** | Squash merge via PR |
| `release/v0.x.0` | **dev** | **main** + **dev** | Merge commit (сохраняет историю) |
| `hotfix/*` | **main** | **main** + **dev** | Cherry-pick |

Формат имени: `{type}/{slug}` — `feat/prd-018-openspec-dag`, `fix/search-ranking`

#### ОБЯЗАТЕЛЬНО перед созданием ветки:
```bash
git checkout dev && git pull origin dev   # ВСЕГДА вытягивать перед новой веткой
git checkout -b feat/my-feature
```
**НЕ создавать ветки из stale dev.** Всегда `git pull` первым.

#### КРИТИЧНО: Dependent sprint branch base verification

**Урок из Sprint 13.1.5 (2026-04-07):** если новый sprint зависит от кода другого sprint'а (ещё не merged), ОБЯЗАТЕЛЬНО проверить что base branch содержит нужные коммиты ПЕРЕД стартом. Иначе teammates упрутся в "код не существует" и придётся rebase + re-spawn.

**Проверка перед стартом dependent sprint'а:**
```bash
# Убедиться что нужный PR/commit уже в base branch
git log release/v0.17.0 --oneline | grep "PRD-043\|feat(integrity)"
# Если нет — либо ждать merge, либо branched FROM dependent feature branch, либо rebase после merge
```

**Правильная цепочка:**
```
PR-A (foundation) → merge → release/v0.17.0 ← base для dependent PR-B
```

**Неправильная цепочка (то что было в Sprint 13.1.5):**
```
release/v0.17.0 (без PRD-043) ← base для hardening sprint, который фиксит PRD-043 код
   ↓
   hardening branch не содержит check_stub — fixers корректно отказались работать
```

**Починка:** `git rebase release/v0.17.0` ПОСЛЕ merge зависимости, resolve конфликтов, re-spawn заблокированных fixers.

**Positive observation:** teammates правильно сообщили "BLOCKER — target code не существует" вместо false-green отчётов. Это показывает что strict file ownership + "run cargo test before reporting done" работают — teammates не делают фейковую работу.

#### Lifecycle ветки:
```
1. git checkout dev && git pull origin dev        # обязательно pull!
2. git checkout -b feat/my-feature
3. ... работа, коммиты ...
4. git push origin feat/my-feature
5. gh pr create --base dev → squash merge в dev (НЕ удалять ветку)
6. git checkout dev && git pull
```

#### Lifecycle релиза:
```
1. git checkout dev && git pull
2. git checkout -b release/v0.x.0
3. cargo test, финальные фиксы на ветке
4. gh pr create --base main → merge commit в main
5. git checkout main && git pull
6. git tag -a v0.x.0 -m "Release v0.x.0" && git push origin v0.x.0
7. git checkout dev && git merge main          # sync tag back to dev
8. git push origin dev
```

#### Правила коммитов:
- **Refs обязательны** — каждый коммит ссылается на артефакт (RFC, FR, ADR)
- **Один коммит = одна логическая единица** — не мешать feat + docs + refactor
- **Description на английском** (для совместимости), body на русском (для контекста)
- **Не коммить напрямую в main или dev** — всегда через feature branch + PR

#### PR pipeline (ОБЯЗАТЕЛЬНО — PR создаётся ТОЛЬКО после всех шагов):

```
Code → Audit → Fix → Test → Fmt → Lint → PR
```

1. **Code** — реализация фичи/фикса на feature branch
2. **Audit** — минимум 2 агента (code review + test coverage), `/audit` со skills
3. **Fix** — исправить все HIGH/CRITICAL findings из аудита
4. **Test** — `cargo test` ВСЕ pass (кроме known preexisting failures)
5. **Fmt** — `cargo fmt` (форматирование) → `cargo fmt -- --check` = 0 diffs. Hook `pre-commit-fmt.sh` блокирует коммит без форматирования
6. **Lint** — `cargo check` = 0 warnings, 0 errors. Git pre-commit hook блокирует если не компилируется
7. **Verify** — ручная проверка каждого фикса/фичи (не поверхностно!)
8. **PR** — только после шагов 1-7

**НЕ создавать PR сразу после кода.** PR = "я проверил, протестировал, отаудитировал, отформатировал, всё работает".

#### PR formatting:
- **ОБЯЗАТЕЛЬНО перед PR**: проверить TODO.md — все P0 checkboxes должны быть `[x]`. Hook `pr-todo-check.sh` блокирует PR с незакрытыми P0.
- **PR title** = `[ARTIFACT-ID] description` — `[PRD-018] OpenSpec DAG integration`
- **PR body** = Summary (bullets) + Refs (артефакты) + Test plan + Audit results
- **feat/* → dev**: Merge commit (НЕ squash!) — squash теряет поздние коммиты
- **НИКОГДА не пушить в ветку после merge PR** — коммиты будут потеряны
- **Перед merge**: убедиться что ВСЕ коммиты pushed: `git log origin/dev..HEAD`
- **После merge**: сразу `git checkout dev && git pull` и проверить что изменения на месте
- **release/* → main**: Merge commit (сохраняет историю RC) — `gh pr create --base main`
- **НЕ удалять ветки после merge** — feature и release branches сохраняются как история
- **После merge в main**: tag + sync dev from main (`git checkout dev && git merge main`)
- **НЕ коммить напрямую в main** — только через release branch
- **НЕ коммить напрямую в dev** — только через feature branch + PR

#### Релизы и тегирование:
- **Формат тега**: `v{major}.{minor}.{patch}` — `v0.8.0`, `v0.9.0`, `v1.0.0`
- **Когда тегировать**: после merge release/* в main
- **ОБЯЗАТЕЛЬНО тегировать каждый релиз** — без тега релиз не считается выпущенным
- **Процесс**:
  1. `dev` → `release/v0.x.0` (RC branch)
  2. Тесты + финальные фиксы на release branch
  3. PR в main → merge commit
  4. `git tag -a v0.x.0 -m "Release v0.x.0: описание"` на main
  5. `git push origin v0.x.0`
  6. Sync: `git checkout dev && git merge main && git push origin dev`
- **Release notes**: автогенерация из conventional commits (`gh release create`)
- **Binary**: `cargo build --release`

#### Worktrees (параллельная работа):
```bash
# Создать worktree для параллельной задачи (hotfix во время фичи)
git worktree add ../forgeplan-fix fix/frontmatter-parser

# Вернуться и удалить после merge
git worktree remove ../forgeplan-fix
```
- **Когда**: hotfix во время долгой фичи; параллельная работа агентов (isolation: "worktree")
- **Правило**: worktree = временный, удалять после merge

### ЗАПРЕЩЁННЫЕ действия (CRITICAL):

**НИКОГДА не удалять `.forgeplan/` без backup:**
```bash
# ПРАВИЛЬНО:
forgeplan export --output backup.json   # ОБЯЗАТЕЛЬНО перед любым reinit
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)  # backup copy
# потом уже можно reinit

# ЗАПРЕЩЕНО:
rm -rf .forgeplan   # ← ПОТЕРЯ ВСЕХ АРТЕФАКТОВ, EVIDENCE, LINKS!
```

**ОБЯЗАТЕЛЬНО при reinit:**
1. `forgeplan export` → сохранить JSON
2. `cp -r .forgeplan .forgeplan-backup-ДАТА`
3. Только потом `rm -rf .forgeplan && forgeplan init -y`
4. `forgeplan import backup.json` → восстановить

**AI агенты:**
- `forgeplan init` → ВСЕГДА с `-y` (non-interactive mode)
- НИКОГДА `rm -rf .forgeplan` без `forgeplan export` первым
- После init → настроить `.forgeplan/config.yaml` (LLM provider)

### ОБЯЗАТЕЛЬНО smoke test после каждого спринта:

```bash
cargo test                              # ВСЕ должны PASS
forgeplan init -y                       # Workspace создаётся
forgeplan new prd "Smoke Test"          # Артефакт создаётся
forgeplan validate PRD-XXX              # Валидация работает
forgeplan score PRD-XXX                 # F-G-R scoring работает
forgeplan blocked                       # Граф зависимостей
forgeplan order                         # Topological sort
forgeplan fpf ingest                    # FPF KB загружается
forgeplan fpf search "trust"            # Поиск работает
```

**Если любой шаг fail — НЕ коммитить. Починить сначала.**

## Структура проекта

```
ForgePlan/
├── CONTEXT.md              ← НАЧНИ ЗДЕСЬ — полный контекст для нового чата
├── VISION.md               ← Архитектура: data model, tech stack, screens, phases
├── PLAN.md                 ← 49 задач, 5 фаз с progress bars
├── TODO.md                 ← Текущие приоритеты P0/P1/P2
├── COMPLETENESS-CHECK.md   ← Gap analysis: 52 компонента, 10 слоёв
├── SOURCES.md              ← Карта всех источников
│
├── docs/                   ← Production documentation (см. docs/README.md — индекс)
│   ├── README.md           ← **ИНДЕКС** — карта всей документации
│   ├── methodology/        ← Методология (10 файлов)
│   │   ├── FORGEPLAN-GUIDE.md   ← **ПОЛНЫЙ ГАЙД** — методология + CLI + evidence + lifecycle
│   │   ├── HOW-TO-USE.md        ← 10 правил методологии с примерами
│   │   ├── ARTIFACT-MODEL.md    ← Иерархия: Epic→PRD→Spec→RFC→ADR + lifecycle
│   │   ├── PRD-RFC-ADR-FLOW.md  ← Decision tree: какой документ создать
│   │   ├── DEPTH-CALIBRATION.md ← Tactical→Standard→Deep→Critical + auto-escalation
│   │   ├── QUALITY-GATES.md     ← Verification Gate + Adversarial Review + R_eff
│   │   ├── UNIFIED-WORKFLOW.md  ← Forgeplan × Orchestra × Hindsight
│   │   ├── USAGE-BY-ROLE.md     ← Как использовать по ролям
│   │   ├── METHODOLOGY-COURSE.md ← Полный курс
│   │   └── GLOSSARY.md          ← 31 термин + lifecycle таблица
│   ├── operations/         ← Setup + hooks + devops
│   │   ├── AGENT-ENFORCEMENT.md ← Правила для AI агентов
│   │   ├── AGENT-HOOKS.md       ← PreToolUse/PostToolUse hooks
│   │   └── REPO-PROTECTION-GUIDE.md ← Branch protection, safety
│   └── schemas/            ← Формальные правила артефактов (PRD, EPIC, SPEC)
│
├── .forgeplan/             ← **Forgeplan workspace** (markdown tracked, lance/cache/config — local)
│   ├── adrs/               ← ADR-001..005 (source of truth, ADR-003)
│   ├── rfcs/               ← RFC-001..006
│   ├── prds/               ← PRD-002..025 (и новые — только через `forgeplan new prd`)
│   ├── epics/              ← EPIC-001, EPIC-002
│   ├── specs/              ← SPEC-*
│   ├── evidence/           ← EvidencePacks (138+ файлов)
│   ├── problems/           ← ProblemCards
│   ├── solutions/          ← SolutionPortfolios
│   ├── notes/              ← Notes
│   ├── refresh/            ← RefreshReports
│   ├── memory/             ← Decision memory
│   ├── lance/              ← ⚠️ gitignored — derived LanceDB index (пересобирается: forgeplan scan-import)
│   ├── .fastembed_cache/   ← ⚠️ gitignored — embedding cache
│   └── config.yaml         ← ⚠️ gitignored — local LLM API keys
│
├── .local/                 ← **gitignored** — локальные заметки
│   ├── research/           ← Raw source materials (BMAD, FPF, Quint-code .docx)
│   ├── planning/           ← Website v1 концепты, sprint plans, аналитика
│   └── sessions/           ← Session briefings, E2E test plans
│
├── templates/              ← Markdown шаблоны (_TEMPLATE.md) — все с YAML frontmatter
│   ├── prd/                ← PRD (обогащён BMAD 13-step validation)
│   ├── brief/              ← Product Brief (lightweight tactical PRD)
│   ├── epic/               ← Epic
│   ├── spec/               ← Specification
│   ├── rfc/                ← RFC (с Implementation Phases)
│   ├── adr/                ← ADR (на deep+ включает DDR: invariants, rollback)
│   ├── problem/            ← ProblemCard (signal, Anti-Goodhart indicators)
│   ├── solution/           ← SolutionPortfolio (variants, weakest link)
│   ├── note/               ← Note (auto-expires 90 days)
│   ├── evidence/           ← EvidencePack (verdict, CL, valid_until → R_eff)
│   └── refresh/            ← RefreshReport (re-evaluation of stale artifacts)
│
├── sources/                ← Reference implementations (READ-ONLY, не редактировать!)
│   ├── quint-code/         ← Go — data model, R_eff scoring, SQLite schema
│   ├── git-adr/            ← Rust — CLI patterns (clap), templates
│   ├── OpenSpec/           ← TypeScript — artifact DAG, delta-specs
│   ├── BMAD-METHOD/        ← Markdown — PRD workflow, 13 validation steps
│   ├── adr-tools/          ← Bash — original ADR CLI
│   └── ccpm/               ← Markdown — Claude Code project management
│
├── crates/                 ← Rust workspace (core + cli + mcp)
├── website/                ← **Official website** (Astro + Starlight + React + GSAP)
│   └── README.md           ← Архитектура, pin strategy, gotchas, design system
└── research/               ← Исследования методологий
```

### Website (PRD-024)

Официальный лендинг + docs portal. **Подробности**: `website/README.md`

Критическое знание:
- **ОДИН GSAP ScrollTrigger pin** на страницу. Для остальных — CSS `position: sticky`
- **Astro scoped CSS** ломает parent→child селекторы — выносить в `global.css`
- **prefers-reduced-motion**: если добавлять — показывать начальное состояние, не финальное
- **Tokens**: цвета в `website/src/tokens.ts` (единый источник для JS), CSS vars в `global.css`

## Артефакты (10 типов)

### Из Quint-code (5):
| Kind | Prefix | Описание |
|------|--------|----------|
| Note | `note-` | Микро-решение |
| ProblemCard | `prob-` | Проблема с контекстом |
| SolutionPortfolio | `sol-` | 2-3+ варианта (weakest link scoring) |
| EvidencePack | `evid-` | Тесты, benchmarks, measurements |
| RefreshReport | `ref-` | Переоценка stale решений |

### Новые для Forgeplan (5):
| Kind | Prefix | Описание |
|------|--------|----------|
| PRD | `prd-` | Product Requirements Document |
| Epic | `epic-` | Группирует PRD[], RFC[], ADR[] |
| Spec | `spec-` | API contracts, data models |
| RFC | `rfc-` | Архитектурное предложение с фазами |
| ADR | `adr-` | Architecture Decision Record (на deep+ включает DDR-поля: invariants, rollback, valid_until) |

### Иерархия
```
Epic (стратегия) → PRD[] (что и зачем) → Spec[] (контракты) + RFC[] (как строим) + ADR[] (почему так)
```

### Lifecycle flow
```
Small task  → RFC only
Medium task → PRD → RFC → Sprint
Large task  → Epic → PRD[] → Spec[] → RFC[] → ADR[] → Sprint[]
```

## Ключевые формулы и паттерны

### R_eff scoring (из Quint-code)
```
R_eff = min(evidence_scores) — trust = weakest link, НИКОГДА average
```
- Evidence Decay: `valid_until` TTL, expired evidence = 0.1 (stale, not absent)
- CL penalty: CL3=0.0, CL2=0.1, CL1=0.4, CL0=0.9
- DerivedStatus: UNDERFRAMED → FRAMED → EXPLORING → COMPARED → DECIDED → APPLIED

### Depth Calibration
| Complexity | Depth | Создаём | ADI |
|-----------|-------|---------|:---:|
| Quick fix, 1 файл | Tactical | Note или ничего | — |
| Фича 1-3 дня | Standard | PRD (tactical) → RFC | рекомендуется |
| Новый модуль, 1-2 нед | Deep | PRD → Spec → RFC → ADR | **обязателен** |
| Подсистема, кросс-команда | Critical | Epic → PRD[] → Spec[] → RFC[] → ADR[] | **обязателен + review** |

### Workflow паттерны
- **Adversarial Review** (BMAD) — reviewer MUST find problems; 0 issues = re-review
- **Delta-specs** (OpenSpec) — describe ONLY changes: ADDED/MODIFIED/REMOVED
- **ADI cycle** (FPF) — Abduction (3+ hypotheses) → Deduction → Induction
- **Pipeline = guideline**, NOT rigid sequence (подтверждено FPF автором)
- **Contextual chain** — output каждой фазы = input следующей

## Storage: Markdown primary, LanceDB derived (ADR-003)

- **Markdown files** в `.forgeplan/{adrs,rfcs,prds,epics,specs,evidence,problems,solutions,notes,refresh,memory}/` = **source of truth** (git-tracked)
- **LanceDB** в `.forgeplan/lance/` = derived index layer — rebuildable через `forgeplan scan-import`, **gitignored**
- **Config** `.forgeplan/config.yaml` = local LLM keys, **gitignored**
- **Cache** `.forgeplan/.fastembed_cache/` = embedding cache, **gitignored**

```
.forgeplan/
├── adrs/               ← tracked (source of truth)
├── rfcs/, prds/, epics/, specs/
├── evidence/, problems/, solutions/
├── notes/, refresh/, memory/
│
├── lance/              ← ⚠️ gitignored (derived index)
├── .fastembed_cache/   ← ⚠️ gitignored (cache)
└── config.yaml         ← ⚠️ gitignored (local)
```

**Fresh clone workflow:**
```bash
git clone <repo> && cd forgeplan
forgeplan init -y                # creates empty .forgeplan/lance/ locally
forgeplan scan-import            # rebuilds index from tracked markdown
forgeplan list                   # verify
```

**Rules:**
- **Always edit via `forgeplan` CLI** — `forgeplan new`, `forgeplan update`, etc. Direct markdown edits work but require `forgeplan scan-import` to rebuild LanceDB.
- **Never commit `.forgeplan/lance/`** — it's derived, rebuildable, and can drift between devs.
- **Never commit `.forgeplan/config.yaml`** — contains LLM API key env refs.

## Rust Architecture (реализовано)

```
crates/
├── forgeplan-core/               ← SHARED LIBRARY (12.8K LOC, 194 теста)
│   ├── artifact/                 ← types, frontmatter parser
│   ├── config/                   ← .forgeplan/config.yaml
│   ├── db/                       ← LanceDB store (CRUD, relations, search)
│   ├── depth/                    ← depth calibration heuristics
│   ├── embed/                    ← fastembed (BGE-M3, behind feature flag)
│   ├── fpf/                      ← FPF engine: bounded contexts, explore-exploit
│   ├── graph/                    ← mermaid dependency graph
│   ├── health/                   ← project health dashboard
│   ├── journal/                  ← decision journal with R_eff
│   ├── lifecycle/                ← review → activate → supersede/deprecate/stale/renew/reopen (ADR-005)
│   ├── link/                     ← typed artifact relationships
│   ├── llm/                      ← LLM integration (generate, reason, route, capture)
│   ├── progress/                 ← checkbox parser + ASCII progress bars
│   ├── projection/               ← markdown projection (LanceDB → .md)
│   ├── routing/                  ← rule-based Smart Routing v2 (no LLM)
│   ├── scoring/                  ← R_eff + F-G-R quality scoring
│   ├── search/                   ← keyword + semantic search
│   ├── stale/                    ← expired valid_until detection
│   ├── template/                 ← tera template engine
│   ├── validation/               ← depth-aware rules (30+ per kind)
│   └── workspace/                ← .forgeplan/ directory management
├── forgeplan-cli/                ← CLI binary (33 commands, clap derive)
└── forgeplan-mcp/                ← MCP server (26 tools, rmcp, stdio transport)
```

## Reference Code — что откуда портировать

| Что портируем | Откуда | Куда (Rust) |
|--------------|--------|-------------|
| Data model (ArtifactKind, Meta, Link) | `sources/quint-code/src/mcp/internal/artifact/types.go` | `crates/forgeplan-core/src/artifact/types.rs` |
| R_eff scoring (52 LOC) | `sources/quint-code/src/mcp/internal/reff/reff.go` | `crates/forgeplan-core/src/scoring/reff.rs` |
| SQLite schema (9 tables) | `sources/quint-code/src/mcp/schema.sql` | Адаптация под LanceDB tables |
| CLI patterns (clap) | `sources/git-adr/src/cli/` | `crates/forgeplan-cli/src/commands/` |
| Template engine | `sources/git-adr/src/core/templates.rs` | `crates/forgeplan-core/src/template/` |
| PRD validation (13 steps) | `sources/BMAD-METHOD/src/bmm-skills/2-plan-workflows/create-prd/` | `crates/forgeplan-core/src/validation/` |
| Artifact DAG, delta-specs | `sources/OpenSpec/src/core/` | `crates/forgeplan-core/src/artifact/` |
| Slash commands UX | `sources/quint-code/src/mcp/cmd/commands/*.md` | CLI UX design |

## Non-Goals

- НЕ project management (не Jira/Linear)
- НЕ CI/CD, НЕ SaaS, НЕ code generator
- Local-first, single binary, git для sync
