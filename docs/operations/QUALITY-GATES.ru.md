[English](QUALITY-GATES.md) · [Русский](QUALITY-GATES.ru.md)

# CI Quality Gates — Инфраструктура проверок качества (v0.28.0+)

Описание всех автоматических качественных ворот (quality gates), которые работают
в Forgeplan CI и в локальных хуках. Документ отвечает на четыре вопроса
для каждого гейта: **что проверяет**, **когда запускается**, **как запустить
локально**, **как исправить частые ошибки**.

> **Связанный документ**: [`docs/methodology/QUALITY-GATES.ru.md`](../methodology/QUALITY-GATES.ru.md)
> описывает методологические ворота (Verification Gate, Adversarial Review,
> R_eff scoring). Этот документ — про CI/CD инфраструктуру, а не про методологию.

---

## Обзор гейтов

| Гейт | Триггер | Блокирует |
|---|---|---|
| `cargo fmt --check` | pre-commit hook + CI | коммит, PR merge |
| `cargo check` / `cargo clippy` | pre-commit hook + CI | коммит, PR merge |
| `cargo test` | CI (PR, push dev) | PR merge |
| `forgeplan health` | CI + pre-commit hook | PR merge |
| `forgeplan validate` | CI | PR merge |
| MCP tool count drift detector | CI (PR, push dev) | PR merge |

Hooks в `.claude/hooks/` — локальная страховка до CI. CI-гейты в
`.github/workflows/forgeplan-health.yml` — окончательный барьер перед merge.

---

## 1. Форматирование: `cargo fmt --check`

**Назначение:** проверяет, что исходный код Rust форматирован согласно `rustfmt`
(нет несохранённых diff'ов после автоформата).

**Когда запускается:**
- `pre-commit-fmt.sh` — pre-commit hook. Запускает `cargo fmt --check`; при
  наличии dirty diff — прерывает `git commit` с объяснением.
- CI — каждый PR к `dev` или `main`.

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
Хук `.claude/hooks/pre-commit-fmt.sh` блокирует коммит автоматически,
но полагаться только на хук нельзя — запускайте `cargo fmt` вручную в
конце каждой сессии работы с кодом.

---

## 2. Статический анализ: `cargo check` и `cargo clippy`

**Назначение:** `cargo check` проверяет компилируемость без сборки бинарей.
`cargo clippy` добавляет lint-правила с `-D warnings` — при любом
предупреждении возвращает exit 1.

**Когда запускается:**
- Локально: после каждого изменения кода (рекомендуется в рабочем процессе).
- CI: каждый PR к `dev` или `main`.

**Полная команда для CI:**
```bash
cargo clippy --workspace --all-targets --features test-helpers -- -D warnings
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

**Назначение:** запускает весь тестовый suite workspace. Провал хотя бы одного
теста — exit 1.

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

Эта последовательность обязательна перед любым коммитом. Порядок важен:
сначала fmt (иначе fmt-check упадёт на следующем шаге), потом check (раньше
test — быстрее), потом test.

**Как исправить:** исправьте упавший тест. Если тест падает из-за изменения
в коде — это сигнал, что реализация сломала ожидаемое поведение.
Не меняйте тест, чтобы заставить его пройти, не разобравшись в причине.

---

## 4. Health gate: `forgeplan health`

**Назначение:** проверяет состояние workspace артефактов — нет ли orphan
артефактов (есть в DB, нет в файлах), blind spots (активные без evidence),
stale артефактов. В CI режиме (`--ci --fail-on`) возвращает exit 1
при превышении порогов.

**Когда запускается:**
- `.claude/hooks/pre-commit-health.sh` — pre-commit hook (локально).
- `.github/workflows/forgeplan-health.yml` step `Health check` — CI.

**CI-команда:**
```bash
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"
```

Пороги: `orphans=10` (терпимо до 10 orphan'ов — lance rebuild может
отставать), `blind_spots=5` (более 5 активных без evidence — методологический
долг, блокирует merge).

**Как запустить локально:**
```bash
forgeplan health                                         # обзор состояния
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"  # полная CI-проверка
```

**Как исправить:**

*Orphan артефакты* (есть в файлах, нет в DB):
```bash
forgeplan scan-import   # переиндексирует markdown → LanceDB
forgeplan health        # проверить что orphans=0
```

*Blind spots* (активные без evidence):
```bash
forgeplan health        # смотрим список blind spots
# Для каждого blind spot:
forgeplan new evidence --for <ID> "Smoke evidence"  # или полноценный EVID
forgeplan activate <EVID-ID>
```

*Stale артефакты* (TTL истёк):
```bash
forgeplan stale                    # посмотреть список
forgeplan renew <ID> --reason "..." --until 2026-08-01
# или если устарели совсем:
forgeplan deprecate <ID> --reason "obsolete"
```

**Файл workflow:** `.github/workflows/forgeplan-health.yml`

---

## 5. Методологические гейты: `forgeplan validate` и `forgeplan score`

**Назначение:**
- `forgeplan validate <ID>` — проверяет артефакт на соответствие схеме
  (MUST-секции, frontmatter, формат). Возвращает exit 1 при ошибках.
- `forgeplan score <ID>` — вычисляет R_eff (weakest-link). Не блокирует
  CI сам по себе, но используется в health check'е.
- `forgeplan blocked` — показывает артефакты, заблокированные из-за
  незакрытых зависимостей.
- `forgeplan order` — топологическая сортировка работы по зависимостям.

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
18 мест в документации с устаревшими числами (28 / 37 / 45 / 47 tools при
реальном count 63). Этот script предотвращает повторение: каждый PR,
который добавляет или удаляет MCP-инструмент, автоматически провалит
CI, пока документация не обновлена.

**Source of truth:** количество async-функций с паттерном `async fn forgeplan_*(`
в `crates/forgeplan-mcp/src/server.rs`.

**Файл скрипта:** `scripts/check-mcp-tool-count.sh`

**Когда запускается:**
- CI step `MCP tool count drift check` в `.github/workflows/forgeplan-health.yml`.
  Запускается **последним** в цепочке health-gate шагов: fmt → build → reindex
  → health → validate → drift-check.
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

**Типичный вывод при успехе:**
```
Actual MCP tool count (src): 63

No drift — all docs are consistent with src (63 tools).
```

**Типичный вывод при drift:**
```
Actual MCP tool count (src): 65

Drift detected (3 lines):
  DRIFT: README.md:42:...63 MCP tools...  (number=63 context="63 MCP tools")
  DRIFT: CLAUDE.md:28:...63 MCP tools...  (number=63 context="63 MCP tools")
  DRIFT: website/src/content/index.mdx:17:...63 MCP tools...

Resolution: update each location to actual count (65) OR add a
comment explaining why the historical number is preserved (e.g. CHANGELOG).
```

**Как исправить drift:**

1. Выяснить фактическое число:
   ```bash
   grep -cE 'async fn forgeplan_' crates/forgeplan-mcp/src/server.rs
   ```

2. Обновить все указанные в drift-выводе файлы:
   ```bash
   # Например, если новый count = 65:
   # Редактируем CLAUDE.md "## Current status" строку:
   # "63 MCP tools" → "65 MCP tools"
   ```

3. Если число в конкретном месте — исторически правильное (например, "было
   37 инструментов до v0.22.0") и менять не нужно, добавьте комментарий:
   ```markdown
   <!-- mcp-count-drift: ignore -->
   было 37 инструментов до v0.22.0, теперь 65
   ```

4. Перезапустить скрипт:
   ```bash
   ./scripts/check-mcp-tool-count.sh   # должен вернуть exit 0
   ```

5. Если изменили CHANGELOG.md — регенерировать website mirror:
   ```bash
   cd website && node scripts/copy-changelog.mjs
   ```

**Правило для авторов инструментов:** при добавлении нового MCP-инструмента
(`async fn forgeplan_<name>`) **обязательно** обновите счётчики в CLAUDE.md
`## Current status` и README.md до создания PR. Иначе CI упадёт на
drift-check step.

---

## 7. Pre-commit hooks — обзор (`.claude/hooks/`)

Хуки — локальная страховка, запускаются до `git commit`. Они не заменяют CI,
но позволяют поймать проблемы раньше.

| Файл хука | Что блокирует | Когда |
|---|---|---|
| `forge-safety-hook.sh` | Деструктивные команды (`rm -rf /`, `cargo publish`, `DROP TABLE`, `git push --force`) | pre-tool-use в Claude Code |
| `pre-commit-fmt.sh` | Коммит, если `cargo fmt --check` показывает drift | git pre-commit |
| `commit-test-check.sh` | Коммит, если в diff есть новая `pub fn` без теста | git pre-commit |
| `pr-todo-check.sh` | Push, если в PR открыты незакрытые P0-задачи | git pre-push |
| `pre-commit-health.sh` | Коммит, если `forgeplan health` показывает критические проблемы | git pre-commit |

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

Полный smoke-тест из CLAUDE.md:
```bash
cargo fmt && cargo fmt -- --check && cargo check && cargo test
forgeplan init -y && forgeplan new prd "Smoke" && forgeplan validate PRD-XXX
forgeplan score PRD-XXX && forgeplan blocked && forgeplan order
forgeplan fpf ingest && forgeplan fpf search "trust"
```

---

## Anatomy CI workflow: `forgeplan-health.yml`

**Файл:** `.github/workflows/forgeplan-health.yml`

**Триггер:** PR к `dev` или `main`, при изменениях в `.forgeplan/**` или
`crates/**`.

**Шаги (в порядке выполнения):**

1. `actions/checkout@v4` — checkout кода
2. Установка системных зависимостей (`protobuf-compiler`)
3. `dtolnay/rust-toolchain@stable` — Rust toolchain
4. `Swatinem/rust-cache@v2` — кэш Cargo
5. `cargo build -p forgeplan` — сборка CLI
6. **Rebuild index** — `forgeplan init -y` + копирование markdown + `scan-import`
   (воссоздаёт LanceDB из tracked markdown файлов)
7. **Health check** — `forgeplan health --ci --fail-on "orphans=10,blind_spots=5"`
8. **Validate artifacts** — `forgeplan validate --ci`
9. **MCP tool count drift check** — `./scripts/check-mcp-tool-count.sh`

Каждый шаг — самостоятельный exit-code барьер. При провале шага 7
шаги 8 и 9 не запускаются (fail-fast поведение GitHub Actions).

---

## Частые сценарии и решения

### "PR заблокирован drift-check'ом после добавления нового MCP tool"

```bash
# 1. Узнать текущий count
grep -cE 'async fn forgeplan_' crates/forgeplan-mcp/src/server.rs

# 2. Найти места, которые нужно обновить
./scripts/check-mcp-tool-count.sh --warn

# 3. Обновить каждое указанное место (CLAUDE.md, README.md, website/, docs/)

# 4. Если изменён CHANGELOG.md — регенерировать website
cd website && node scripts/copy-changelog.mjs

# 5. Проверить
./scripts/check-mcp-tool-count.sh   # exit 0
```

### "forgeplan health падает в CI, но локально всё хорошо"

Причина: локальный LanceDB мог содержать устаревшие данные из предыдущих
init-ов. CI всегда делает clean init + scan-import. Воспроизвести локально:

```bash
forgeplan export --output backup-$(date +%Y%m%d).json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
rm -rf .forgeplan && forgeplan init -y
# Скопировать tracked markdown обратно:
cp -r .forgeplan-backup-*/prds/ .forgeplan/
cp -r .forgeplan-backup-*/rfcs/ .forgeplan/
# ... и т.д.
forgeplan scan-import
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"
```

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
