[English coming soon] · [Русский](PLAYBOOK-AUTHORING.ru.md)

# Playbook Authoring Guide (Forgeplan 0.38.0)

Гайд для авторов декларативных YAML-сценариев. Runtime последовательно
делегирует шаги и записывает журнал. CLI и MCP умеют запускать playbook,
но поддержка отдельных типов делегации различается; успешный шаг не доказывает
создание или проверку артефакта.

## Что такое Playbook

**Playbook** — это `.yaml`-файл, описывающий **multi-step workflow**:
триггеры запуска, последовательность шагов с делегациями, mapping для
ингеста результатов, install-хинты на случай отсутствия плагина.

Playbook не исполняется как shell-скрипт, но может содержать шаг `command`
с произвольным argv после отдельного разрешения. `plugin` и `agent` вызывают
Claude CLI; `forgeplan_core` выполняет внутренние операции. `skill` в 0.38.0
только выводит маркер вызова — не загружает навык в агентный контекст.

SPEC-003 описывает проектный контракт формата. Для поведения опубликованного
релиза приоритетны закреплённые исходники 0.38.0 в конце страницы.
Исторические ADR/EVID ниже объясняют решения и измерения того времени,
но не доказывают наличие всех запланированных возможностей сейчас.

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
| `<project>/marketplace/playbooks/*.yaml` | Файлы, отдельно помещённые в checkout проекта; не встроены в бинарь |

CLI ищет последовательно `.forgeplan/playbooks/`, `marketplace/playbooks/`,
`~/.claude/plugins/*/playbooks/` (первое совпадение имени побеждает).
Проверяйте показанный `source_path`; сложенная глубже структура plugin cache
не означает автоматического обнаружения. Для точного выбора передайте путь.

## Структура YAML — top-level поля

Полный референс — SPEC-003 §"Top-level fields". Минимально:

```yaml
schema_version: "1.0"           # SPEC-003 schema version (строка, semver)
name: inspect-notes
title: "Read architecture notes"
description: |
  Поиск архитектурных записей во внутреннем хранилище.
triggered_by:                   # см. ниже — recommendation engine
  has_git: true
  commit_count_min: 50
steps:                          # список шагов (≥1, обязательно)
  - id: inspect-notes
    delegate_to: { type: forgeplan_core, target: search }
    input: { query: architecture }
```

### `triggered_by` — recommendation engine

`forgeplan init` может рекомендовать playbook по сигналам проекта, но не запускает
его и не устанавливает зависимости. Подсказки зависят от TTY и `FORGEPLAN_HINTS`.
`playbook list` перечисляет найденные YAML; не считайте его проверкой готовности.

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

Это декларация требований для автора и читателя. `forgeplan plugins doctor`
проверяет обнаруженные плагины относительно registry, а не доказывает исполнимость
всех шагов этого playbook. Отдельно проверяйте бинарь `claude`, авторизацию,
доступность выбранного agent slug и права на output.

```yaml
requires:
  plugins:
    - name: c4-architecture
      version: ">=1.0"        # semver-диапазон
  skills:
    - name: use-case-miner
      pack: forgeplan-brownfield-pack
```

`fallback_hint` — текст подсказки, не автоматическая установка. Отсутствующий `claude`
может дать `DelegateMissing`; наличие CLI не доказывает наличие нужного агента.

## Структура YAML — `steps`

Каждый step — объект с полями (см. SPEC-003 §"Step object"):

| Поле | Тип | Обязательно | Назначение |
|---|---|:--:|---|
| `id` | string (kebab-case) | да | Уникален в playbook'е |
| `delegate_to` | object | да | Один из 5 типов делегации |
| `input` | object | нет | Step-specific параметры (передаются delegate'у) |
| `produces_at` | path | нет | Ожидаемый output; сам по себе не создаёт файл и не запускает ingest |
| `mapping` | string | нет | Метаданные mapping; загрузчик требует вместе с ним `produces_at`, но автоматического импорта нет |
| `requires` | array of step IDs | нет | DAG ordering — что должно завершиться первым |
| `fallback_hint` | string | нет | Команда установки, если delegate отсутствует |
| `on_error` | enum | нет | `abort` (default) / `continue` |
| `timeout_seconds` | int | нет | Override таймаута subprocess: plugin 600 s, agent 300 s, command 180 s. Не применяется к skill/core |
| `budget_usd` | float | нет | Передаётся в `claude --print --max-budget-usd` для plugin/agent; default `$1.00`. Не является точным бухгалтерским лимитом. Историческое измерение PROB-050 A-25 наблюдало превышение 1.05–1.25× |
| `allowed_tools` | array of tool names | нет | Для plugin/agent передаётся в `--allowedTools`, default `[Read, Glob, Grep]`. Не выдаёт права вместо ограничений хоста |

`requires` использует **ID шагов**, не артефактов и не файлов.
Цикл в `requires` или ссылка на несуществующий ID — load error
(SPEC-003 §Errors).

`schema_version` принимается в диапазоне `>=1.0.0, <2.0.0`; сокращённые
`"1.0"` и `"1.2"` нормализуются. Поля таймаута (1.1), бюджета и инструментов
(1.2) добавлены без смены major. Неизвестные верхнеуровневые поля отвергаются,
неизвестные поля Step игнорируются; не рассчитывайте на диагностику каждой опечатки.

## Пять типов делегации

`delegate_to` — один из пяти вариантов. `command` требует отдельного согласия:
обычный `--yes` сам по себе не открывает выполнение command-шагов.

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

`plugin` в этом runtime означает вызов **агента**, доступного Claude CLI:
`target` (либо `name`, если target пуст) становится `--agent <slug>`.
Это не универсальный API запуска любого плагина. Поставляемые плагины и команды
сверяйте с публичным marketplace, а не с личным набором установок.

### 2. `agent` — Claude Code subagent

```yaml
- id: review-design
  delegate_to:
    type: agent
    name: c4-component
  input:
    target_dir: "src/payments/"
```

Используйте реальное имя агента, которое разрешает `claude --print --agent`.
Runtime проверяет slug по `[A-Za-z][A-Za-z0-9_-]{0,63}`; произвольные
namespaced-строки и пути не принимаются.

### 3. `skill` — agent-skills capability

```yaml
- id: recommend-use-case-miner
  delegate_to:
    type: skill
    name: use-case-miner
    pack: forgeplan-brownfield-pack
  fallback_hint: "Review the installed skill and invoke it in the agent host"
```

**Граница 0.38.0:** `SkillDispatcher` выводит `[skill-invoke] /<pack>/<name>`,
возвращает success и передаёт `produces_at`. Он не проверяет registry, не запускает
subprocess и не создаёт output. Хост может обработать маркер, но core этого
не гарантирует. Команды `forgeplan skill install` нет; установка навыка и его
реальный вызов выполняются средствами выбранного окружения/маркетплейса.

### 4. `command` — произвольный shell (opt-in)

```yaml
- id: inspect-working-tree
  delegate_to:
    type: command
    command: ["git", "status", "--short"]
```

`command` исполняет argv напрямую, без неявного `sh -c` и подстановки переменных.
Это всё равно выполнение программы с правами процесса, не песочница.
Проверяйте шаг вручную до разрешения `--allow-shell`; запрета на явно заданный
интерпретатор оболочки или опасную программу из этого не следует.

### 5. `forgeplan_core` — встроенная операция

```yaml
- id: ingest-c4
  delegate_to:
    type: forgeplan_core
    target: ingest
  input:
    mapping_path: .forgeplan/mappings/c4-to-forge.yaml
    source_path: C4-Documentation/
    dry_run: true
```

Поддерживаются `ingest`, `new`, `validate`, `activate`, `search`, без subprocess.
В `input` обязательны: ingest — `mapping_path`, `source_path`; new — `kind`, `title`;
validate/activate — `id`; search — `query`. Для activate доступны `force`,
для ingest — `dry_run`, `update`, но `update` во внутренней записи 0.38.0 не применяется.
`new`/`ingest` могут писать, `activate` меняет lifecycle: отсутствие subprocess
не означает отсутствие мутаций или необходимость меньшего контроля.

## DAG-ordering через `requires`

```yaml
schema_version: "1.2"
name: inspect-architecture
title: "Read architecture notes, then requirements"
steps:
  - id: find-architecture
    delegate_to: { type: forgeplan_core, target: search }
    input: { query: architecture }
  - id: find-requirements
    delegate_to: { type: forgeplan_core, target: search }
    input: { query: requirements }
    requires: [find-architecture]
```

Loader строит граф из `requires` и валидирует:

- Ссылка на **несуществующий step ID** → load error (SPEC-003 §Errors).
- **Цикл** в графе → load error с показом цепочки.
- Шаги выполняются **последовательно**, в топологическом порядке. DAG шагов
  — не граф задач трекера и не параллельный запуск агентов.

## `fallback_hint` — install-команды

Укажите понятную инструкцию восстановления, соответствующую реальному способу
установки в вашем окружении. Runtime не выполняет `fallback_hint`.
Отказ запуска может означать отсутствие бинаря, неверный slug, ошибку авторизации
или другую ошибку Claude CLI; наличие плагина в registry не исключает эти причины.

## Конвенции naming

- `name` и `id` — **kebab-case** (`brownfield-code`, `run-c4-architecture`).
- Файл playbook'а: `<name>.yaml` (`brownfield-code.yaml`).
- Описывайте назначение, требования и ожидаемый output в комментариях.
- Используйте `schema_version: "1.2"` для примеров с бюджетом и tools.
  Поддерживаемый диапазон — весь major 1, не только `"1.0"`.

## Валидация и запуск

```bash
# Проверка структуры конкретного файла — без выполнения
forgeplan playbook validate .forgeplan/playbooks/inspect-architecture.yaml

# Просмотр и dry-run не запускают delegates
forgeplan playbook show .forgeplan/playbooks/inspect-architecture.yaml
forgeplan playbook run .forgeplan/playbooks/inspect-architecture.yaml --dry-run --json

# Реальный запуск без command-шагов
forgeplan playbook run .forgeplan/playbooks/inspect-architecture.yaml --yes

# Command-шаги требуют обоих разрешений; --allow-shell не заменяет --yes
forgeplan playbook run .forgeplan/playbooks/inspect-architecture.yaml --yes --allow-shell

# Начать с номера в топологическом порядке, не с ID и не «только один шаг»
forgeplan playbook run .forgeplan/playbooks/inspect-architecture.yaml --yes --step 2
```

Без `--yes` CLI отказывает реальному запуску, а не запрашивает интерактивное согласие.
В help 0.38 осталось «Implies --yes» для `--allow-shell`, но реализация проверяет два разрешения независимо. Для command-шагов передавайте оба флага.
`--dry-run` работает без него. В MCP для реального запуска передавайте `yes: true`;
при command-шаге также `allow_shell: true` (или workspace `playbook.allow_shell: true`).
Значение в YAML-конфиге workspace:

```yaml
playbook:
  allow_shell: false
```

`false` по умолчанию. При `true` command-шаги предодобрены для workspace;
отдельного CLI-флага принудительного запрета на один запуск нет.

## Errors — что отвергает loader

Полная матрица — SPEC-003 §Errors. Кратко:

| Условие | Severity |
|---|---|
| Отсутствует обязательное поле (`name` / `title` / `steps`) | ERROR |
| Пустой `steps` массив | ERROR |
| Неизвестный `delegate_to.type` | ERROR (показывает 5 валидных) |
| `requires:` ссылается на несуществующий step ID | ERROR |
| Цикл в `requires:` графе | ERROR (показывает цикл) |
| Delegate недоступен | Ошибка исполнения, не доказательство полной проверки зависимостей при load |
| Неизвестное YAML-поле на top-level | ERROR |
| Неизвестное поле внутри Step | Игнорируется; не полагайтесь на WARN |
| `schema_version` вне `>=1.0.0, <2.0.0` | ERROR |
| `produces_at` есть, но `mapping` нет | WARN (output не ingestится) |
| `mapping` есть, но `produces_at` нет | ERROR |

## Subprocess lifecycle (Phase 6)

Subprocess запускают **plugin, agent и command**. `skill` только печатает
маркер вызова, `forgeplan_core` выполняет внутренний API. Ни success, ни
заявленный `output_path` сами по себе не проверяют существование и качество результата.

История subprocess-решения: [ADR-010](../../.forgeplan/adrs/ADR-010-phase-6-subprocess-invocation-via-tokio-process-with-kill-on-drop-and-timeout.md)
и [EVID-090](../../.forgeplan/evidence/EVID-090-spike-2-tokio-process-subprocess-invocation-validated-for-phase-6-dispatchers.md)
(историческое Spike-2 measurement, CL3).

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

`command` использует direct exec; `skill` не запускает subprocess.
`forgeplan_core` исполняется в процессе Forgeplan.

### Async exec через tokio::process

Все subprocess-based dispatchers строят
`tokio::process::Command` с фиксированной конфигурацией:

```text
stdin   = piped(prompt) для plugin/agent; null для command
stdout  = piped
stderr  = piped
env     = clear() + явный allow-list
kill_on_drop(true)
```

Output читается concurrent через `tokio::join!(stdout, stderr, wait)` —
sequential read-then-wait зависает на stderr-heavy children.

Cap на буфер — **10 MiB** на каждый поток. Превышение → step Failed
с `OutputTooLarge`. Pack authors: если plugin emits >10 MiB stdout,
пишите его в файл (`produces_at`), а не в stdout.

### Timeout policy

| Делегация | Default timeout | Почему |
|---|---:|---|
| `plugin` | **600 s** | Claude CLI subprocess |
| `agent` | **300 s** | Claude CLI subprocess |
| `skill` | **не применяется** | Только маркер, без реального вызова навыка |
| `command` | **180 s** | Прямой запуск argv |
| `forgeplan_core` | **не применяется** | Внутренняя операция |

Per-step override (FR-8 follow-up — schema_version 1.1):

```yaml
- id: deep-c4-scan
  delegate_to: { type: plugin, name: c4-architecture, target: c4-code }
  timeout_seconds: 1200          # 20 мин для большого монорепо
```

`timeout_seconds: Option<u32>` — backward compatible: старые playbook'и
без поля грузятся OK с дефолтом для типа делегации. Поле landed в
schema 1.1 (additive minor bump).

При таймауте subprocess runtime останавливает дочерний процесс и возвращает
ошибку шага. `on_error: abort` останавливает оставшуюся работу; `continue`
позволяет независимым шагам продолжить, но зависимые от неуспешного шага
пропускаются. Это не откат уже выполненных записей.

### Security model

Хардкорные invariants Phase 6 (см. ADR-010 §Invariants):

- Subprocess не наследует интерактивный stdin. Для plugin/agent промпт передаётся через pipe.
- `command` запускается как argv, без неявной оболочки. Проверяйте программу и аргументы.
- Окружение очищается; command получает ограниченный набор `PATH/HOME/USER`.
  Plugin/agent используют отдельный явный allow-list и авторизацию Claude CLI;
  не рассчитывайте на автоматическую передачу API-ключей Forgeplan.
- Command-шагу требуется `--allow-shell` либо workspace opt-in, не только `--yes`.
- `kill_on_drop(true)` и таймаут относятся к управляемому дочернему процессу,
  не доказывают очистку всего дерева потомков. При SIGKILL родителя Drop не выполняется.
- Поля `env:` у Step нет: неизвестное поле может быть проигнорировано, а не передано subprocess.

### Fallback hint behaviour

Отсутствующий исполняемый файл может вернуть `DispatchError::DelegateMissing`
с подсказкой. Не обещайте такую проверку для skill: registry в его dispatcher
не проверяется. `fallback_hint` не обязателен на уровне загрузчика.

```yaml
- id: run-c4-architecture
  delegate_to: { type: plugin, name: c4-architecture, target: c4-code }
  fallback_hint: "claude plugin install c4-architecture"
```

Пример выше описывает внешнюю зависимость, не входящую в поставку core.
Уточните marketplace и точную install-команду у поставщика выбранного плагина.
Не интерпретируйте отсутствие нужного агента как гарантию указанного текста ошибки.

`Fix:` — подсказка по исправлению, не автоматическое одобрение установки.
Конкретный формат зависит от возникшей ошибки. Добавляйте `fallback_hint`
как полезную инструкцию, но не считайте его проверкой доступности delegate.

### Скрытое поведение `forgeplan_core`

Делегация `forgeplan_core` — единственная **без subprocess**:

```yaml
- id: ingest-c4
  delegate_to: { type: forgeplan_core, target: ingest }
  input:
    mapping_path: .forgeplan/mappings/c4-to-forge.yaml
    source_path: C4-Documentation/
    dry_run: true
```

Реальный путь — `IngestEngine::apply` внутри `forgeplan_core_dispatcher.rs`.
`step.mapping` и `step.produces_at` не подставляют параметры `input` и не
запускают импорт автоматически после чужого шага.

Внутренняя запись ingest в 0.38.0 **не эквивалентна CLI**: выдаёт новые ID,
не проверяет type+title+source_hash для повторного импорта, игнорирует `update`
и не добавляет draft links. Повтор может создать дубликаты.
См. [INGEST-MAPPINGS.ru.md](INGEST-MAPPINGS.ru.md#workflow-integration-ingest-из-playbook-phase-6).
MCP `forgeplan_ingest` — ещё одна поверхность: он только планирует вызов CLI.

### Resumability + journal

Журнал `.forgeplan/journal/playbook-runs.jsonl` пополняется границами запуска
и шагов; executor вызывает flush после StepEnd.

Повтор `forgeplan playbook run X --yes` создаёт **новый run_id** и не читает
предыдущий журнал для автоматического пропуска готовых шагов. `--step N`
только пропускает предыдущие номера; их состояние остаётся Skipped,
поэтому зависимые от них шаги тоже могут быть пропущены. Это не полноценный resume.
Читайте `per_step`, `failed`, `skipped`, а не только exit code: CLI возвращает
ошибку при failed > 0, но один лишь skipped может не сделать код выхода ненулевым.

Исторические измерения EVID-090 относятся к таймауту/управляемому kill,
не к гарантии очистки потомков после SIGKILL самого Forgeplan.

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
- [Публичный map-build playbook](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-map-pack/playbooks/map-build.yaml) использует собственную схему (`type: precondition/dispatch/gate/inline`), а не `delegate_to`. Его нельзя передать напрямую в `forgeplan playbook run` 0.38.0: отсутствуют обязательные поля core и есть неизвестные top-level ключи. Это сценарий навыка пакета, не пример YAML для копирования в core.

## Источники релиза 0.38.0

- [CLI: согласие, dry-run, номер шага](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/playbook.rs#L368-L557).
- [Типы и поля](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/playbook/types.rs#L117-L220), [loader](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/playbook/loader.rs#L124-L185).
- [Executor: новый run_id и зависимые пропуски](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/playbook/executor.rs#L184-L325).
- [Skill marker-only](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/playbook/dispatch/skill_dispatcher.rs#L87-L109), [core input](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/playbook/dispatch/forgeplan_core_dispatcher.rs#L131-L200).
