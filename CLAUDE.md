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

- **v0.7.0** released — EPIC-001 complete
- **33 CLI команд**, **28 MCP tools**, **225 тестов**
- **20 dogfood артефактов** в LanceDB (5 active, 15 draft)
- **Phase 0–4** — DONE
- **Phase 5** (Desktop App, Tauri) — backlog

Подробности: `TODO.md` (текущие приоритеты).

## Как начать работу в новом чате

1. **Прочитай этот файл** — CLAUDE.md содержит CLI workflow, методологию, git-конвенции
2. **`forgeplan health`** — понять текущее состояние проекта (artifacts, blind spots, next actions)
3. **Для текущих задач** — `TODO.md`
4. **Полный гайд по CLI и методологии** — `docs/guides/FORGEPLAN-GUIDE.md`
5. **Для reference code** — `sources/` (read-only repos, см. таблицу ниже)
6. **Используй Hindsight** — `memory_recall("Forgeplan")` для быстрого восстановления контекста

### ОБЯЗАТЕЛЬНО перед работой над задачей:

```bash
forgeplan route "описание задачи"   # определи depth и pipeline
```

Если route говорит Standard+ → создай артефакт ПЕРЕД кодингом. Если Tactical → просто делай.

### ОБЯЗАТЕЛЬНО при создании артефакта (Shape → Validate → Code):

1. **`forgeplan new prd "Title"`** — создаёт stub из шаблона
2. **СРАЗУ заполни ВСЕ MUST секции** — Problem, Goals, Non-Goals, Target Users, Related, FR
3. **`forgeplan validate PRD-XXX`** — убедись что PASS (0 MUST errors)
4. **Только ПОСЛЕ validate PASS** — начинай писать код

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

**Работа не закончена, пока: PRD заполнен + validate PASS + evidence создан + R_eff > 0 + activated.**

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

### ОБЯЗАТЕЛЬНО на session start:

```bash
forgeplan health
```

Если health показывает **blind spots** (active без evidence) или **orphans** (без связей) — **FIX ИХ ПЕРВЫМИ**, до начала новой работы. Не копи долг.

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

1. **Shape → Validate → Code → Evidence → Activate** — полный цикл, не пропускай шаги
2. **Создавай артефакт → СРАЗУ заполняй MUST секции** — stub PRD = долг, который копится
3. **Evidence делает R_eff живым** — без evidence все scores = 0.0, health кричит "blind spot"
4. **Не активируй без кода** — active PRD без реализации = ложное обещание
5. **Не создавай все 10 типов** — реально используются 6: PRD, RFC, ADR, Note, Problem, Epic
6. **route перед работой** — определяет depth и pipeline, экономит время
7. **health на session start** — показывает orphans, blind spots; **fix их первыми**
8. **Работа не закончена пока**: PRD заполнен + validate PASS + evidence создан + R_eff > 0 + activated

## Как пользоваться методологией (quick reference)

> Полный гайд: `docs/guides/HOW-TO-USE.md`

### Routing — один вопрос определяет depth:
```
Тривиально, обратимо за день?  → Tactical: ничего или Note
Фича 1-3 дня, есть выбор?      → Standard: Brief/PRD → RFC
Необратимо, 1-2 недели?        → Deep: PRD → Spec → RFC → ADR
Кросс-команда, стратегия?       → Critical: Epic → PRD[] → Spec[] → RFC[] → ADR[]
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
├── docs/
│   ├── schemas/            ← Формальные правила артефактов
│   │   ├── PRD-SCHEMA.md   ← Обязательные секции PRD, depth calibration, validation
│   │   ├── EPIC-SCHEMA.md  ← Aggregated progress, children rules
│   │   └── SPEC-SCHEMA.md  ← API contracts, data models, versioning
│   ├── guides/
│   │   ├── FORGEPLAN-GUIDE.md   ← **ПОЛНЫЙ ГАЙД** — методология + CLI + evidence + lifecycle
│   │   ├── HOW-TO-USE.md        ← 10 правил методологии с примерами
│   │   ├── ARTIFACT-MODEL.md    ← Иерархия: Epic→PRD→Spec→RFC→ADR + lifecycle
│   │   ├── PRD-RFC-ADR-FLOW.md  ← Decision tree: какой документ создать
│   │   ├── DEPTH-CALIBRATION.md ← Tactical→Standard→Deep→Critical + auto-escalation
│   │   ├── QUALITY-GATES.md     ← Verification Gate + Adversarial Review + R_eff
│   │   └── GLOSSARY.md          ← 31 термин + lifecycle таблица
│   ├── epics/              ← Dogfood: EPIC-001-build-forgeplan.md
│   ├── prds/               ← Dogfood: PRD-001-forgeplan-cli.md
│   ├── adrs/               ← Dogfood: ADR-001..003 (Rust, LanceDB, DEC→ADR merge)
│   ├── references/
│   │   ├── REF-DOCS-ANALYSIS.md ← Анализ 10 методологий
│   │   └── SKILLS-AUDIT.md      ← 52 skills по 10 слоям + gaps
│   └── ref/                ← Raw reference docs (Word, Markdown) на русском
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
└── research/               ← Исследования методологий
```

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
| Complexity | Depth | Создаём |
|-----------|-------|---------|
| Quick fix, 1 файл | Tactical | Note или ничего |
| Фича 1-3 дня | Standard | PRD (tactical) → RFC |
| Новый модуль, 1-2 нед | Deep | PRD → Spec → RFC → ADR |
| Подсистема, кросс-команда | Critical | Epic → PRD[] → Spec[] → RFC[] → ADR[] |

### Workflow паттерны
- **Adversarial Review** (BMAD) — reviewer MUST find problems; 0 issues = re-review
- **Delta-specs** (OpenSpec) — describe ONLY changes: ADDED/MODIFIED/REMOVED
- **ADI cycle** (FPF) — Abduction (3+ hypotheses) → Deduction → Induction
- **Pipeline = guideline**, NOT rigid sequence (подтверждено FPF автором)
- **Contextual chain** — output каждой фазы = input следующей

## Storage: LanceDB primary

- **LanceDB** = sole source of truth (structured tables + vector embeddings)
- **Markdown** = projections generated at `forgeplan new` (git-tracked, read-only after creation)
- Mutations через `forgeplan update` обновляют только LanceDB, не markdown

```
.forgeplan/          ← создаётся forgeplan init в целевом проекте
├── config.yaml
├── lance/           ← LanceDB (gitignore)
├── prds/            ← markdown (git-tracked)
├── epics/, specs/, rfcs/, adrs/
├── problems/, solutions/
├── evidence/, notes/, refresh/
```

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
