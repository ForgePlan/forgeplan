[English coming soon] · [Русский](PLAYBOOK-AUTHORING.ru.md)

# Playbook Authoring Guide (v0.26.0+)

Гайд для авторов playbook'ов — декларативных YAML-сценариев, которые
оркестрируют внешние плагины/скиллы/агентов и ингестят их output в
forge-граф (PRD/ADR/Epic/Note/Spec).

## Что такое Playbook

**Playbook** — это `.yaml`-файл, описывающий **multi-step workflow**:
триггеры запуска, последовательность шагов с делегациями, mapping для
ингеста результатов, install-хинты на случай отсутствия плагина.

Не путать со **скриптом**: playbook не содержит исполняемого кода —
только декларативные шаги. Реальную работу делают **внешние плагины**
(c4-architecture, autoresearch), **скиллы** (forge-history-miner),
**агенты** Claude Code, либо **встроенные** операции forgeplan-core
(`ingest`, `new`, `validate`, `activate`, `search`).

Источник истины формата — [SPEC-003](../../.forgeplan/specs/SPEC-003-playbook-yaml-schema.md).
Этот документ — пользовательский guide; для правил валидации см.
SPEC-003 §Errors.

## Когда писать playbook

Playbook нужен, когда:

- Workflow состоит из **2+ шагов** с зависимостями (сначала C4,
  потом ingest, потом summary).
- Часть шагов делегируется **внешним инструментам** (плагин, скилл,
  агент).
- Output одного шага должен быть **ингестирован** в forge-артефакты
  через mapping (см. [INGEST-MAPPINGS.ru.md](INGEST-MAPPINGS.ru.md)).
- Сценарий **повторяемый** между проектами и/или должен распространяться
  через marketplace pack.

Когда playbook **не нужен**: одиночные команды (`forgeplan new prd`),
ad-hoc shell-скрипты, разовые исследования.

## Где хранить

| Локация | Назначение |
|---|---|
| `<pack>/playbooks/*.yaml` | Канонические playbook'и распространяются через marketplace pack |
| `.forgeplan/playbooks/*.yaml` | Workspace-локальные playbook'и (не публикуются) |
| `marketplace/playbooks/*.yaml` (этот репозиторий) | Канонические playbook'и Forgeplan |

`forgeplan playbook list` обходит все три локации (pack registry +
workspace) и показывает доступные имена.

## Структура YAML — top-level поля

Полный референс — SPEC-003 §"Top-level fields". Минимально:

```yaml
schema_version: "1.0"           # SPEC-003 schema version (строка, semver)
name: brownfield-code           # уникальный kebab-case идентификатор
title: "Brownfield code-first onboarding"
description: |
  Пятишаговый workflow: detect → C4 → ingest → mine → summary.
triggered_by:                   # см. ниже — recommendation engine
  has_git: true
  commit_count_min: 50
requires:                       # plugin/skill prerequisites
  plugins:
    - name: c4-architecture
      version: ">=1.0"
steps:                          # список шагов (≥1, обязательно)
  - id: ...
    delegate_to: { ... }
```

### `triggered_by` — recommendation engine

Сигналы для PRD-067 FR-5 — `forgeplan init` сравнивает их с детектом
проекта (есть ли `.git`, сколько коммитов, есть ли `docs/`,
`.obsidian/`, `Cargo.toml`) и ранжирует подходящие playbook'и.

```yaml
triggered_by:
  empty_repo: false
  has_git: true
  commit_count_min: 100
  has_docs: false
  has_obsidian: false
  has_cargo_toml: true
```

Все поля **optional** — указывайте только то, что действительно
характеризует целевой проект. Слишком общие триггеры → playbook
рекомендуется везде → пользователь игнорирует hint.

### `requires` — prerequisites

Декларативная заявка на плагины и скиллы. `forgeplan plugins doctor`
читает это поле и предупреждает, если что-то не установлено
**до** запуска.

```yaml
requires:
  plugins:
    - name: c4-architecture
      version: ">=1.0"        # semver-диапазон
  skills:
    - name: forge-history-miner
      pack: brownfield-code-pack
```

Если плагина нет на машине, конкретный шаг падает с **install-хинтом**
из `fallback_hint` (см. ниже).

## Структура YAML — `steps`

Каждый step — объект с полями (см. SPEC-003 §"Step object"):

| Поле | Тип | Обязательно | Назначение |
|---|---|:--:|---|
| `id` | string (kebab-case) | да | Уникален в playbook'е |
| `delegate_to` | object | да | Один из 5 типов делегации |
| `input` | object | нет | Step-specific параметры (передаются delegate'у) |
| `produces_at` | path | нет | Куда delegate пишет output (для ingest) |
| `mapping` | string | нет | Имя mapping'а (SPEC-004) для `produces_at` |
| `requires` | array of step IDs | нет | DAG ordering — что должно завершиться первым |
| `fallback_hint` | string | нет | Команда установки, если delegate отсутствует |
| `on_error` | enum | нет | `abort` (default) / `continue` |
| `timeout_seconds` | int | нет | Per-step override default'а dispatcher'а (300s general / 600s plugin / 180s command/skill). Schema 1.1+ (PRD-072 FR-8) |
| `budget_usd` | float | нет | USD cap для `claude --print` invocations (Plugin/Agent paths). Schema 1.2+ (ADR-011 §Decision). По умолчанию `$1.00` — слишком низок для adversarial review playbook'ов; ставьте `5.00` для audit-style steps. **Внимание**: claude enforces budget post-hoc — observed 1.05-1.25× overrun над cap (PROB-050 A-25). |
| `allowed_tools` | array of tool names | нет | Whitelist tool names для `claude --print`. По умолчанию `[Read, Glob, Grep]` (least-privilege analytic). Расширяйте до `[Read, Write, Bash]` для production-builder steps. Schema 1.2+ |

`requires` использует **ID шагов**, не артефактов и не файлов.
Цикл в `requires` или ссылка на несуществующий ID — load error
(SPEC-003 §Errors).

**Schema versioning** — `schema_version` совместима с `^1.0` (semver caret).
Поля `timeout_seconds` (1.1) и `budget_usd` + `allowed_tools` (1.2) —
**additive, не breaking**: playbook'и без них загружаются идентично schema
1.0. Loader предупреждает (`WARN`) о неизвестных полях, не fails.

## Пять типов делегации

`delegate_to` — strict typed: ровно один из пяти вариантов.
Произвольный shell **запрещён**, кроме явного opt-in типа `command`.

### 1. `plugin` — внешний Claude Code plugin

```yaml
- id: run-c4-architecture
  delegate_to:
    type: plugin
    name: c4-architecture
    target: c4-code           # plugin-internal target
  input:
    scope: full-repo
  produces_at: "C4-Documentation/"
  fallback_hint: "claude plugin install c4-architecture"
```

Когда использовать: вы интегрируете **зрелый плагин** из marketplace
(c4-architecture, autoresearch, ddd-domain-expert), который сам по себе
production-ready. Forgeplan не дублирует его генерацию документов —
только оркестрирует и ингестит.

### 2. `agent` — Claude Code subagent

```yaml
- id: review-design
  delegate_to:
    type: agent
    name: c4-component
  input:
    target_dir: "src/payments/"
```

Когда использовать: для шагов, где **нет готового плагина**, но есть
описанный agent (`AGENT.md`) с нужным workflow.

### 3. `skill` — agent-skills capability

```yaml
- id: run-history-miner
  delegate_to:
    type: skill
    name: forge-history-miner
    pack: brownfield-code-pack
  fallback_hint: "forgeplan skill install brownfield-code-pack"
```

Когда использовать: когда capability компактнее agent'а (один SKILL.md,
1–2 prompt'а). Forgeplan-skills устанавливаются через
`forgeplan skill install` и доступны всем агентам в workspace.

### 4. `command` — произвольный shell (opt-in)

```yaml
- id: backup-before-mutation
  delegate_to:
    type: command
    command: ["git", "stash", "push", "-u"]
```

Когда использовать: **последний resort** — если другая делегация
не подходит. Forgeplan validator подсвечивает все `command`-шаги
(`detect_command_delegates()` для аудита) — review их обязателен.

### 5. `forgeplan_core` — встроенная операция

```yaml
- id: ingest-c4
  delegate_to:
    type: forgeplan_core
    target: ingest             # ingest | new | validate | activate | search
  mapping: c4-to-forge
  produces_at: "C4-Documentation/"
```

Когда использовать: для шагов, которые делает сам forgeplan-core
без внешних зависимостей (`ingest`, `new`, `validate`, `activate`,
`search`). Безопасно: ничего не запускает, не делегирует — прямой
вызов внутреннего API.

## DAG-ordering через `requires`

```yaml
steps:
  - id: detect-c4-need
    delegate_to: { type: forgeplan_core, target: validate }

  - id: run-c4-architecture
    delegate_to: { type: plugin, name: c4-architecture, target: c4-code }
    requires: [detect-c4-need]

  - id: ingest-c4
    delegate_to: { type: forgeplan_core, target: ingest }
    mapping: c4-to-forge
    requires: [run-c4-architecture]
```

Loader строит граф из `requires` и валидирует:

- Ссылка на **несуществующий step ID** → load error (SPEC-003 §Errors).
- **Цикл** в графе → load error с показом цепочки.
- Параллельность: в v1 шаги выполняются sequentially (PRD-065
  Non-Goals); v2 будет планировать параллель по DAG.

## `fallback_hint` — install-команды

Если плагин/скилл, на который ссылается шаг, **не установлен**, runtime
**останавливает** playbook и эмитит:

```
Error: step `run-c4-architecture` requires plugin `c4-architecture`,
       not installed.
Fix: claude plugin install c4-architecture
```

`fallback_hint` обязан быть **точной командой** для текущей платформы.
Не пишите «установите c4-architecture из marketplace» — пишите
полную shell-строку. AC-4 PRD-065 проверяет именно это поведение.

## Конвенции naming

- `name` и `id` — **kebab-case** (`brownfield-code`, `run-c4-architecture`).
- Файл playbook'а: `<name>.yaml` (`brownfield-code.yaml`).
- Header-comment с purpose + version + cross-links — обязательно
  (см. `marketplace/playbooks/brownfield-code.yaml` как образец).
- `schema_version: "1.0"` — пока единственный поддерживаемый.
  При выходе v2 SPEC-003 объявит migration policy.

## Валидация и запуск

```bash
# Проверка YAML — без выполнения
forgeplan playbook validate marketplace/playbooks/brownfield-code.yaml
# → OK: brownfield-code (5 steps)

# Просмотр steps без выполнения
forgeplan playbook show brownfield-code
forgeplan playbook run brownfield-code --yes --dry-run

# Реальный запуск
forgeplan playbook run brownfield-code --yes

# Запустить только один шаг (для отладки)
forgeplan playbook run brownfield-code --yes --step run-c4-architecture
```

`--yes` обязателен в non-interactive режиме (CI, AI agents) —
без него runtime требует интерактивное подтверждение.

## Errors — что отвергает loader

Полная матрица — SPEC-003 §Errors. Кратко:

| Условие | Severity |
|---|---|
| Отсутствует обязательное поле (`name` / `title` / `steps`) | ERROR |
| Пустой `steps` массив | ERROR |
| Неизвестный `delegate_to.type` | ERROR (показывает 5 валидных) |
| `requires:` ссылается на несуществующий step ID | ERROR |
| Цикл в `requires:` графе | ERROR (показывает цикл) |
| Плагин из `requires:` не установлен | WARN при load, ERROR при run |
| Неизвестное YAML-поле на top-level | ERROR (`deny_unknown_fields`) |
| Неизвестное поле внутри step | WARN (forward compat) |
| `schema_version` > runtime | ERROR (suggest upgrade) |
| `produces_at` есть, но `mapping` нет | WARN (output не ingestится) |
| `mapping` есть, но `produces_at` нет | ERROR |

## Subprocess lifecycle (Phase 6)

С v0.27.0 четыре делегации (`plugin`, `agent`, `skill`, `command`)
запускают **реальный subprocess**, а не mock-stub. Пятая (`forgeplan_core`)
по-прежнему остаётся direct internal call в текущем процессе. Эта
секция объясняет invariants, которым должен соответствовать pack
author при выборе делегации и параметров шага.

Источник истины — [ADR-010](../../.forgeplan/adrs/ADR-010-phase-6-subprocess-invocation-via-tokio-process-with-kill-on-drop-and-timeout.md)
+ [EVID-090](../../.forgeplan/evidence/EVID-090-spike-2-tokio-process-subprocess-invocation-validated-for-phase-6-dispatchers.md)
(Spike-2 measurement, CL3).

**Обновление v0.28.0 ([ADR-011](../../.forgeplan/adrs/ADR-011-plugin-agent-dispatchers-invoke-claude-print-directly.md))**:
для **Plugin** и **Agent** delegations subprocess invocation теперь идёт
через `claude --print --agent <slug>` (real `claude` CLI), не через
fictional `task-tool` / `claude-code-plugin` бинари из ADR-010. Argv
shape:

```text
claude --print --agent <slug> --output-format json \
       --max-budget-usd <budget_usd> \
       [--add-dir <produces_at>] \
       --allowedTools <T1> <T2> ...
```

Агенту prompt передаётся через **stdin pipe**, не argv (вариадик
`--allowedTools` иначе бы поглотил позиционный prompt). JSON envelope
на stdout содержит `is_error` / `subtype` / `total_cost_usd` для
dispatcher decoding. Empirical proof: [EVID-097](../../.forgeplan/evidence/EVID-097-phase-b-track-4-a8-real-e2e-closure-measurement.md)
(real `claude` 2.1.126, 5 invocations, $0.98 spent).

`Skill`, `Command`, `ForgeplanCore` dispatchers — без изменений
относительно ADR-010 контракта.

### Async exec через tokio::process

Все subprocess-based dispatchers строят
`tokio::process::Command` с фиксированной конфигурацией:

```text
stdin   = Stdio::null()        // нет path для interactive injection
stdout  = Stdio::piped()       // concurrent drain (избегает 64K-buffer deadlock)
stderr  = Stdio::piped()       // same — capture для journal
env     = clear() + allow-list // нет утечки FORGEPLAN_*
kill_on_drop(true)             // SIGKILL/TerminateProcess при cancel/panic
```

Output читается concurrent через `tokio::join!(stdout, stderr, wait)` —
sequential read-then-wait зависает на stderr-heavy children.

Cap на буфер — **10 MiB** на каждый поток. Превышение → step Failed
с `OutputTooLarge`. Pack authors: если plugin emits >10 MiB stdout,
пишите его в файл (`produces_at`), а не в stdout.

### Timeout policy

| Делегация | Default timeout | Почему |
|---|---:|---|
| `plugin` | **600 s** (10 мин) | Полные C4/autoresearch/DDD прогоны бывают долгими |
| `agent` | **300 s** (5 мин) | Subagent invoke — ограниченный scope |
| `skill` | **180 s** | In-process v1 stub; Wave 5 = real registry |
| `command` | **180 s** | Whitelisted shell — короткие операции |
| `forgeplan_core` | **n/a** | Direct internal call, без subprocess |

Per-step override (FR-8 follow-up — schema_version 1.1):

```yaml
- id: deep-c4-scan
  delegate_to: { type: plugin, name: c4-architecture, target: c4-code }
  timeout_seconds: 1200          # 20 мин для большого монорепо
```

`timeout_seconds: Option<u32>` — backward compatible: старые playbook'и
без поля грузятся OK с дефолтом для типа делегации. Поле landed в
schema 1.1 (additive minor bump).

При timeout runtime вызывает `child.kill().await`, journal записывает
`Failed { reason: timeout, duration_ms }`, остальные шаги
abort'ятся (если `on_error: abort` — default) или продолжаются
(если `on_error: continue`).

### Security model

Хардкорные invariants Phase 6 (см. ADR-010 §Invariants):

- **NEVER**: `Stdio::inherit()` для stdin — закрывает interactive
  prompt injection.
- **NEVER**: `sh -c` или shell expansion. `command:`-делегации принимают
  `Vec<String>` — direct exec через argv-list, никаких глоб'ов или
  переменных оболочки в процессе spawn.
- **NEVER**: env passthrough by default. `env_clear()` + explicit
  allow-list (`PATH`, `HOME`, опционально `FORGEPLAN_WORKSPACE`).
  Никакие `FORGEPLAN_*` ключи (LLM provider, OpenAI keys) НЕ утекают
  в subprocess.
- **NEVER**: `command:`-делегация без флага `--yes` в run-команде. Гейт
  `validate_command_delegate_security` отвергает execution в
  interactive режиме.
- **ALWAYS**: `kill_on_drop(true)` — гарантия очистки zombie
  processes на Ctrl+C, panic, или drop executor'а.
- **ALWAYS**: timeout enforced — нет «бесконечного» step'а.

Pack authors: если ваш plugin требует переменную окружения, добавьте
её в allow-list через `env:` поле шага (a follow-up FR — пока хардкод
allow-list внутри dispatcher).

### Fallback hint behaviour

Если plugin/agent/skill **не установлен**, subprocess не запускается.
Dispatcher эмитит `DispatchError::DelegateMissing` с install-командой
из `fallback_hint`:

```yaml
- id: run-c4-architecture
  delegate_to: { type: plugin, name: c4-architecture, target: c4-code }
  fallback_hint: "claude plugin install c4-architecture"
```

В terminal output:

```text
Error: step `run-c4-architecture` requires plugin `c4-architecture`,
       not installed.
Fix: claude plugin install c4-architecture
```

Контракт `Fix:` маркера — PRD-071 (см.
[agent-protocol.md](../methodology/agent-protocol.md)). Агент Claude
Code распарсит `Fix:` и предложит выполнить install-команду или
прервать playbook.

`fallback_hint` обязательно для каждой `plugin`/`skill` делегации —
load-time error, если поле отсутствует у `plugin:` step. Это
явный invariant ADR-009: pack author не может полагаться, что
target-плагин установлен глобально.

### Скрытое поведение `forgeplan_core`

Делегация `forgeplan_core` — единственная **без subprocess**:

```yaml
- id: ingest-c4
  delegate_to: { type: forgeplan_core, target: ingest }
  mapping: c4-to-forge
  produces_at: "C4-Documentation/"
```

Реальный путь — direct internal call в `forgeplan_core_dispatcher.rs`.
`ForgeplanOp::Ingest` мапится на existing `ingest::engine::run()` API
без дополнительного процесса.

Преимущества: no overhead на fork+exec, shared в-memory state,
journal flushes synchronous. Используется heavy в `greenfield-kickoff.yaml`
(7 шагов через ForgeplanCore + 1 optional Skill).

Ограничение: `forgeplan_core` поддерживает строго 5 операций —
`ingest`, `new`, `validate`, `activate`, `search`. Нужна другая —
делайте через `command:` с явным `forgeplan <subcommand>` или
открывайте PR в `forgeplan-core::playbook::dispatch::forgeplan_core_dispatcher`.

### Resumability + journal

Journal flushed **после каждого `StepEnd`** (per Phase 5 contract).
При kill -9 во время step → restart через `forgeplan playbook run X --yes`
resumes с последнего persisted StepEnd.

`kill_on_drop` гарантирует, что subprocess с убитого forgeplan-процесса
тоже не остаётся zombie — tokio runtime отправляет SIGKILL при Drop
уже после того, как forgeplan-процесс упал. Spike-2 (EVID-090)
verified: после timeout/kill `pgrep` показывал 0 zombies.

Pack authors не должны делать step «ленивым» (например, polling
external API). Вместо этого — `Wait:` hint (PRD-071) с явным TTL
и retry-командой; отдельный шаг получит свежий subprocess.

### Кросс-ссылки секции

- [ADR-010 — Subprocess invocation strategy](../../.forgeplan/adrs/ADR-010-phase-6-subprocess-invocation-via-tokio-process-with-kill-on-drop-and-timeout.md) — формальное решение
- [EVID-090 — Spike-2 measurement](../../.forgeplan/evidence/EVID-090-spike-2-tokio-process-subprocess-invocation-validated-for-phase-6-dispatchers.md) — empirical CL3
- [PRD-072 — Phase 6 PRD](../../.forgeplan/prds/PRD-072-real-subprocess-dispatchers-init-recommendation-wiring-greenfield-playbook.md) — FR-1..FR-10 scope

## Кросс-ссылки

- [SPEC-003 — Playbook YAML schema](../../.forgeplan/specs/SPEC-003-playbook-yaml-schema.md) — формальный контракт
- [SPEC-004 — Mapping YAML schema](../../.forgeplan/specs/SPEC-004-mapping-yaml-schema.md) — формат `mapping`, на который ссылается `step.mapping`
- [PRD-065 — Playbook runtime](../../.forgeplan/prds/PRD-065-playbook-yaml-schema-runtime-executor.md) — контракт runtime'а
- [ADR-009 — Forgeplan as orchestrator](../../.forgeplan/adrs/ADR-009-forgeplan-as-orchestrator-playbook-skill-agent-mapping-pack-marketplace-model.md) — почему playbook'и появились
- [INGEST-MAPPINGS.ru.md](INGEST-MAPPINGS.ru.md) — как писать mapping'и для `step.mapping`
- [marketplace/playbooks/brownfield-code.yaml](../../marketplace/playbooks/brownfield-code.yaml) — рабочий образец для копирования
