[English](QUALITY-GATES.md) · [Русский](QUALITY-GATES.ru.md)

# CI Quality Gates — Инфраструктура проверок качества (v0.28.0+)

Описание всех автоматических качественных ворот (quality gates), которые работают
в Forgeplan CI и в локальных хуках. Документ отвечает на четыре вопроса
для каждого гейта: **что проверяет**, **когда запускается**, **как запустить
локально**, **как исправить частые ошибки**.

Scope: здесь описаны обязательные проверки и source-defined triggers, не
текущий зелёный CI receipt и не live-аудит branch rules. Merge блокируют только
применимые required checks/rules; hooks требуют подключения в host.
По `AGENTS.md` дорогие проверки запускает один допущенный tester/build owner
под общей lease после завершения writers, а не каждый параллельный автор.
Команды ниже — справочник, не разрешение запускать их в любой задаче.

> **Связанный документ**: [`docs/methodology/QUALITY-GATES.ru.md`](../methodology/QUALITY-GATES.ru.md)
> описывает методологические ворота (Verification Gate, Adversarial Review,
> R_eff scoring). Этот документ — про CI/CD инфраструктуру, а не про методологию.

---

## Обзор гейтов

| Гейт | Триггер | Блокирует |
|---|---|---|
| `cargo fmt --check` | настроенный Claude PreToolUse hook + CI | совпавшая попытка commit; required CI |
| `cargo check` / `cargo clippy` | CI; локально verification owner | required CI |
| nextest + doc-tests | CI (PR, push dev) | required CI |
| `forgeplan health` | CI; advisory local hook | CI-пороги, не универсальный local commit block |
| `forgeplan validate` | CI | required CI |
| MCP tool count drift detector | CI (PR, push dev) | required CI |

Hooks в `.claude/hooks/` — локальная страховка до CI. Health/validation
выполняются в `.github/workflows/forgeplan-health.yml`, а tool-count drift —
отдельно в `.github/workflows/ci.yml`, в том числе для docs-only PR.

---

## 1. Форматирование: `cargo fmt --check`

**Назначение:** проверяет, что исходный код Rust форматирован согласно `rustfmt`
(нет несохранённых diff'ов после автоформата).

**Когда запускается:**
- Настроенный Claude Code `PreToolUse` handler `pre-commit-fmt.sh` проверяет строки `Bash` с commit-командой и может отклонить tool call.
- CI — PR к `dev`/`main` и push к `dev`.

**Файл хука:** `.claude/hooks/pre-commit-fmt.sh`

**Как запустить локально:**
```bash
cargo fmt                    # auto-fix (применяет изменения)
cargo fmt -- --check         # dry-run — показывает diff без изменений; exit 1 при drift
```

**Как исправить:**
```bash
# Просто запустить форматтер
cargo fmt
# Убедиться что всё чисто
cargo fmt -- --check         # должен выдать exit 0, пустой stdout
```

Форматирование — ОБЯЗАТЕЛЬНО перед каждым коммитом (CLAUDE.md §Rust coding rules п.3).
Это не стандартный Git hook: он не перехватывает commits всех клиентов.
Требование форматирования сохраняется; проверки выполняет допущенный
verification owner под общей build/test lease.

---

## 2. Статический анализ: `cargo check` и `cargo clippy`

**Назначение:** `cargo check` проверяет компилируемость без сборки бинарей.
`cargo clippy` добавляет lint-правила с `-D warnings`; warning приводит
к nonzero exit, не обязательно 1.

**Когда запускается:**
- Локально: назначенный verification owner согласно scope задачи и lease.
- CI: PR к `dev`/`main` и push к `dev`.

**Полная команда для CI:**
```bash
RUSTFLAGS=-Dwarnings cargo clippy --workspace --all-targets
RUSTFLAGS=-Dwarnings cargo clippy --workspace --all-targets --features semantic-search
```

**Как запустить локально:**
```bash
cargo check --workspace                                    # быстрая проверка
cargo clippy --workspace --all-targets -- -D warnings      # полный lint
```

**Как исправить:**

Большинство clippy-предупреждений исправляются автоматически:
```bash
cargo clippy --fix --workspace --allow-dirty --allow-staged
```

Если предупреждение нельзя исправить автоматически, и оно false-positive —
добавьте точечное подавление в код с объяснением:
```rust
#[allow(clippy::too_many_arguments)]  // Нет способа разбить без потери читаемости
pub fn complex_init(/* ... */) { /* ... */ }
```

Не подавляйте `clippy::all` или `warnings` глобально — это маскирует
реальные проблемы. Rust 1.95 усилил часть lint'ов (CLAUDE.md §Rust coding rules п.3).

---

## 3. Тестирование: `cargo test`

**Назначение:** запускает выбранные test targets workspace. Cargo сообщает
о провале кодом 101; это не script-specific exit 1.

Текущий CI выполняет `cargo nextest run --workspace --all-targets` и отдельно
`cargo test --workspace --doc`; semantic-search oracle — отдельный job.
Локальные команды ниже — альтернативы, не точная копия CI job.

**Когда запускается:**
- CI: каждый PR + push к `dev`.
- Локально: обязательно перед каждым коммитом (CLAUDE.md §Rust coding rules п.3).

**Как запустить локально:**
```bash
cargo test --workspace                                     # базовый запуск
cargo test --workspace --features test-helpers             # с test-helper escape-hatches
cargo test --workspace --features test-helpers -- --nocapture  # verbose
```

**Smoke-тест из CLAUDE.md:**
```bash
cargo fmt && cargo fmt -- --check && cargo check && cargo test
```

Применимые pre-commit проверки остаются обязательными; их один раз запускает
designated owner под общей lease, не каждый writer. Порядок — fmt, fmt-check,
compilation, tests. В отчёте нужны фактические команды и outcomes, не PASS
невыполненной последовательности.

**Как исправить:** исправьте упавший тест. Если тест падает из-за изменения
в коде — это сигнал, что реализация сломала ожидаемое поведение.
Не меняйте тест, чтобы заставить его пройти, не разобравшись в причине.

---

## 4. Health gate: `forgeplan health`

**Назначение:** graph-orphans — артефакты без входящих и исходящих связей;
blind spots — подходящие decision-артефакты без linked evidence; stale/risk
signals. В CI `--ci --fail-on` даёт nonzero при превышении заданных порогов.

**Когда запускается:**
- Настроенный `.claude/hooks/pre-commit-health.sh` — advisory PreToolUse, не стандартный Git pre-commit gate.
- `.github/workflows/forgeplan-health.yml` step `Health check` — CI.

**CI-команда:**
```bash
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"
```

Пороги — `orphans=10` и `blind_spots=5`. Orphan означает разрыв графа,
не несовпадение файлов с DB; отставание индекса — отдельный диагноз.
Блокировка merge зависит от required-check configuration.

**Как запустить локально:**
```bash
forgeplan health                                         # обзор состояния
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"  # полная CI-проверка
```

**Как исправить:**

*Graph-orphans*: выясните, нужна ли реальная типизированная связь, добавьте
только обоснованный link и снова изучите health. `scan-import` импортирует
внешние Markdown, а не чинит отсутствующие graph edges.

*Blind spots*: соберите реальные evidence, создайте EvidencePack через
`forgeplan new evidence "Verification of <ID>"`, заполните наблюдения и scoring
fields поддерживаемым authoring-путём, затем свяжите полученный EVID ID с
решением через `forgeplan link <EVID-ID> <ID>`. У `new` нет `--for`.
Не создавайте и не активируйте пустое evidence ради зелёного dashboard.

*Expiry*: `forgeplan stale` перечисляет overdue records, не меняя их status.
`forgeplan renew <ID> --reason "..." --until <YYYY-MM-DD>` требует допустимого
перехода сохранённого status и обоснованного reassessment. Продление даты —
не новое измерение; expired запись с сохранённым active не допускает
active→active renewal. Устаревшее решение закрывают обоснованным lifecycle-путём.

**Файл workflow:** `.github/workflows/forgeplan-health.yml`

---

## 5. Методологические гейты: `forgeplan validate` и `forgeplan score`

**Назначение:**
- `forgeplan validate <ID>` — показывает реализованные schema checks (секции,
  наличие metadata, placeholders, depth-specific rules); `--ci` даёт exit 1 при MUST failures.
- `forgeplan score <ID>` — вычисляет R_eff (weakest-link). Не блокирует
  CI сам по себе, но используется в health check'е.
- `forgeplan blocked` — показывает артефакты, заблокированные из-за
  незакрытых зависимостей.
- `forgeplan order` — топологическая сортировка работы по зависимостям.

CLI `validate` работает на legacy и migrated Constellation layout; full MCP
`forgeplan_validate` также успешно проверен на migrated артефактах.
Границы проверки см. в [оговорках пилота](CONSTELLATION-PILOT.ru.md):
это не гарантия поведения всех зарегистрированных tools.
Ноль MUST findings не гарантирует активацию: есть length/evidence/stub gates.
Не все рекомендации схем автоматически проверяются.

**Когда запускается:**
- `forgeplan validate --ci` — в CI step `Validate artifacts`
  (`.github/workflows/forgeplan-health.yml`).
- Методологический smoke-тест из CLAUDE.md (после каждого спринта):
  ```bash
  forgeplan validate PRD-XXX && forgeplan score PRD-XXX
  forgeplan blocked && forgeplan order
  ```

**CI-команда:**
```bash
forgeplan validate --ci   # валидирует все артефакты; exit 1 при любой MUST-ошибке
```

**Как запустить локально:**
```bash
forgeplan validate PRD-001          # конкретный артефакт
forgeplan validate --ci             # все артефакты разом
forgeplan score PRD-001             # R_eff score
forgeplan blocked                   # что заблокировано
forgeplan order                     # в каком порядке работать
```

**Как исправить:**

Validate выводит список MUST-секций, которых не хватает, или поля
с неверным форматом. Исправьте указанные секции. Validator принимает
алиасы (CLAUDE.md §Validator aliases):
- `## Problem` = `## Motivation` = `## Problem Statement`
- `## Goals` = `## Success Criteria`
- и др.

Если validate падает из-за правильно написанного артефакта, возможно
использован нераспознанный алиас — добавьте стандартный заголовок или
проверьте список алиасов в CLAUDE.md.

**Файл workflow:** `.github/workflows/forgeplan-health.yml`

---

## 6. Drift detector: `scripts/check-mcp-tool-count.sh`

**Назначение:** сравнивает **фактическое число MCP-инструментов** в исходном
коде (`crates/forgeplan-mcp/src/server.rs`) с **числами в документации**
(README, CLAUDE.md, website, docs). Если документация расходится с кодом —
exit 1 (CI failure).

**Предыстория (PROB-050):** в audit v0.28.0 внешний OpenAI-агент обнаружил
~18 мест в документации с устаревшими счётчиками MCP-инструментов
(несколько разных исторических значений против актуального count на момент
v0.28.0). Скрипт проверяет распознанные числовые claims в заданных путях,
не всю документацию или все profile modules. Нераспознанные формулировки
и исключённые строки могут остаться вне проверки. Зелёный job не доказывает
полное покрытие; изучайте реальный scope и output detector.

**Source of truth:** detector считает registration attributes `#[tool(...)]`
в `crates/forgeplan-mcp/src/server.rs`, не считая undecorated test functions.
Отдельный модуль read-профиля в этот count не входит.

**Файл скрипта:** `scripts/check-mcp-tool-count.sh`

**Когда запускается:**
- CI job `check-mcp-tool-count-drift` в `.github/workflows/ci.yml`.
  Он отделён от path-filtered health workflow, чтобы работать на docs-only PR.
- **Не** запускается как pre-commit hook (слишком медленно для каждого коммита);
  рекомендован для запуска как pre-push check или локально перед PR.

**Что проверяет:**

Скрипт ищет в следующих путях:
- `CLAUDE.md`
- `README.md`
- `TODO.md`
- `website/src/` (все `.md`, `.tsx`, `.astro`, `.mdx`)
- `docs/` (все `.md`)

Паттерн: строки вида `<N> MCP tools`, `<N> инструментов`, `<N> tools`
(только числа ≥ 10, чтобы не ловить "3 tools cover..." и подобное).

Исключения из проверки:
- `CHANGELOG` файлы и строки — исторические числа сохраняются намеренно.
- Строки с комментарием `# mcp-count-drift: ignore`.
- TODO.md строки с паттерном `Previous: v0.*`.

**Как запустить локально:**
```bash
# Строгий режим (как в CI)
./scripts/check-mcp-tool-count.sh

# Warn-only (не прерывает работу, только показывает drift)
./scripts/check-mcp-tool-count.sh --warn

# Помощь
./scripts/check-mcp-tool-count.sh --help
```

**Иллюстративный вывод при успехе** (числа зависят от релиза —
`<ACTUAL>` — registration count, который сообщает detector):
```
Actual MCP tool count (src): <ACTUAL>

No drift — all docs are consistent with src (<ACTUAL>).
```

**Типичный вывод при drift** (`<ACTUAL>` ≠ `<STALE>`):
```
Actual MCP tool count (src): <ACTUAL>

Drift detected (3 lines):
  DRIFT: README.md:42:...<STALE> MCP …    (number=<STALE> context="<STALE> MCP …")
  DRIFT: CLAUDE.md:28:...<STALE> MCP …    (number=<STALE> context="<STALE> MCP …")
  DRIFT: website/src/content/index.mdx:17:...<STALE> MCP …

Resolution: update each location to actual count (<ACTUAL>) OR add a
comment explaining why the historical number is preserved (e.g. CHANGELOG).
```

**Как исправить drift:**

1. Получите count и список строк самим detector:
   ```bash
   ./scripts/check-mcp-tool-count.sh --warn
   ```

2. Обновите текущие утверждения для соответствующей сборки/профиля.
   Не переписывайте исторические release facts и не смешивайте full/read count.

3. Если число в конкретном месте — исторически правильное (например,
   release-notes-строка ссылается на счётчик предыдущей версии) и менять
   нельзя, **добавьте drift-ignore комментарий в ту же самую строку** где
   находится число: `... <!-- mcp-count-drift: ignore -->` (маркер ОБЯЗАН
   быть на одной строке с числом — `grep -v` line-anchored).

4. Перезапустить скрипт:
   ```bash
   ./scripts/check-mcp-tool-count.sh   # должен вернуть exit 0
   ```

5. Если изменили CHANGELOG.md — регенерировать website mirror:
   ```bash
   cd website && node scripts/copy-changelog.mjs
   ```

**Правило для авторов tools:** при изменении MCP registrations обновляйте
актуальные build/profile claims до PR. Функция с именем `forgeplan_*`
без `#[tool(...)]` сама по себе не зарегистрированный tool.

Command-drift detector сканирует `CLAUDE.md` и shipped forge skill, не README
и не все `docs/**`. Он проверяет существование команд/флагов, не positional
arguments и не runtime success. Зелёный detector не подтверждает все примеры.
Регенерация CLI/MCP reference — явный шаг обслуживания, не замена упражнению
примеров; сохраняйте ручные объяснения и различайте профили.

---

## 7. Pre-commit hooks — обзор (`.claude/hooks/`)

Это настроенные Claude Code `PreToolUse` handlers, не автоматически
установленные `.git/hooks`; они дополняют CI только для совпавших tool calls.

| Файл хука | Что блокирует | Когда |
|---|---|---|
| `forge-safety-hook.sh` | Буквальные blacklist matches, не shell parser/sandbox | настроенный PreToolUse, только `Bash` |
| `pre-commit-fmt.sh` | Совпавший commit tool call при fmt drift | настроенный PreToolUse, `Bash` |
| `commit-test-check.sh` | Добавлены public functions, но ни одного `#[test]`; малое отношение только warn, соответствие function↔test не доказано | настроенный PreToolUse, commit command |
| `pr-todo-check.sh` | `gh pr create`/`submit` с открытыми P0, не push | настроенный PreToolUse, `Bash` |
| `pre-commit-health.sh` | Advisory warnings, exit 1; без tool/workspace пропуск | настроенный PreToolUse, commit command |

**Важно:** хуки расположены в `.claude/hooks/` (для Claude Code integration),
но **не** в стандартном `.git/hooks/`. Они активируются Claude Code средой
как `PreToolUse` хуки. Стандартные git pre-commit хуки работают независимо
(при наличии).

**Подробный гайд:** [`docs/operations/AGENT-HOOKS.ru.md`](AGENT-HOOKS.ru.md)

---

## Порядок гейтов (контракт для разработчика)

Правильный порядок перед коммитом/PR — критичен. Неправильный порядок
маскирует ошибки и затрудняет их изоляцию:

```
Перед каждым коммитом:
  1. cargo fmt                              # fix
  2. cargo fmt -- --check                   # verify
  3. cargo check --workspace                # 0 warnings
  4. cargo test --workspace                 # 0 failures
  5. forgeplan health                       # нет критических blind spots

Перед PR (дополнительно):
  6. cargo clippy --workspace --all-targets -- -D warnings   # strict lint
  7. forgeplan validate --ci                # все артефакты pass
  8. ./scripts/check-mcp-tool-count.sh     # нет drift
```

Для methodology smoke создайте отдельный sandbox вне существующих workspace,
создайте PRD и заполните нужные секции **до** валидации. Приложите реальные
evidence вместо примерного test title; незаполненный шаблон не должен
объявляться успешным. Выберите совместимую CLI/MCP-операцию для layout.
FPF ingest/search — отдельная проверка с собственным prerequisite корпуса.

---

## Anatomy CI workflow: `forgeplan-health.yml`

**Файл:** `.github/workflows/forgeplan-health.yml`

**Триггер:** PR к `dev` или `main`, при изменениях в `.forgeplan/**` или
`crates/**`.

**Шаги (в порядке выполнения):**

1. Pinned `actions/checkout` — checkout кода
2. Установка системных зависимостей (`protobuf-compiler`)
3. `dtolnay/rust-toolchain@stable` — Rust toolchain
4. `Swatinem/rust-cache@v2` — кэш Cargo
5. `cargo build -p forgeplan` — сборка CLI
6. **Rebuild index** — `forgeplan reindex` из tracked canonical Markdown;
   без удаления workspace, частичного copy-back и round-trip через `scan-import`
7. **Health check** — `forgeplan health --ci --fail-on "orphans=10,blind_spots=5"`
8. **Validate artifacts** — `forgeplan validate --ci`

Каждый шаг — отдельный exit-code барьер. При провале health check валидация
не выполняется. MCP count drift — отдельная job в `ci.yml`.

---

## Частые сценарии и решения

### "PR заблокирован drift-check'ом после добавления нового MCP tool"

```bash
# 1. Прочитать registration count из detector
./scripts/check-mcp-tool-count.sh --warn

# 2. Найти места, которые нужно обновить
./scripts/check-mcp-tool-count.sh --warn

# 3. Обновить каждое указанное место (CLAUDE.md, README.md, website/, docs/)

# 4. Если изменён CHANGELOG.md — регенерировать website
cd website && node scripts/copy-changelog.mjs

# 5. Проверить
./scripts/check-mcp-tool-count.sh   # exit 0
```

### "forgeplan health падает в CI, но локально всё хорошо"

Отставший производный индекс — одна из возможных причин, не установленный
диагноз. CI восстанавливает индекс из tracked Markdown через `reindex`.
Сравнивайте состояния, сохраняя canonical данные:

1. Остановите конкурирующих writers; сохраните **весь** `.forgeplan/` и MCP
   config вне workspace. Одного экспорта недостаточно.
2. Проверьте обнаруженный путь и локальный config. Parent discovery и отсутствие
   config могут объяснять отличие; `init -y` не является reset.
3. Из корня workspace выполните `forgeplan reindex` подходящим бинарником.
   На migrated layout он передаёт работу Constellation; доступен и явный
   `forgeplan index`. Для семантического покрытия выполните `forgeplan embed`.
4. Проверьте ожидаемые артефакты и диагностику, затем применимый health gate.
   Перед CLI `validate` проверьте совместимость layout.

Не удаляйте `.forgeplan/`: там canonical документы, config и локальное состояние,
не только disposable индекс. `init --force` сохраняет тела, но активирует default
config; custom edits (включая `embedding`) остаются только в backup до ручного
переноса. См. [переносимый recovery/rollback runbook](CONSTELLATION-PILOT.ru.md).

### "cargo fmt --check падает в CI, но локально fmt чистый"

Проверить версию rustfmt:
```bash
rustfmt --version   # должна совпадать с CI (stable)
cargo +stable fmt -- --check
```

Если версии разные — обновите toolchain:
```bash
rustup update stable
```

---

## Кросс-ссылки

- [`scripts/check-mcp-tool-count.sh`](../../scripts/check-mcp-tool-count.sh) — drift detector (исходный код с inline комментариями)
- [`.github/workflows/forgeplan-health.yml`](../../.github/workflows/forgeplan-health.yml) — полный CI workflow
- [`docs/operations/AGENT-HOOKS.ru.md`](AGENT-HOOKS.ru.md) — подробный гайд по pre-commit hooks
- [`docs/methodology/QUALITY-GATES.ru.md`](../methodology/QUALITY-GATES.ru.md) — методологические гейты (Verification Gate, R_eff, Adversarial Review)
- [`docs/methodology/release-workflow.md`](../methodology/release-workflow.md) — pre-conditions checklist релиза (использует эти гейты)
- [`CLAUDE.md`](../../CLAUDE.md) §Hooks enforcement — краткий reference table хуков
