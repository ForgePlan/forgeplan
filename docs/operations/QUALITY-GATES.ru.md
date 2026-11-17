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
| `cargo fmt --check` | хук Claude Code + CI | перехваченный коммит; merge, если проверка обязательна |
| `cargo check` / `cargo clippy` | CI (обычная конфигурация + `semantic-search`) | merge, если проверка обязательна |
| Тесты (`nextest` + doc-tests) | CI (PR, push dev) | merge, если проверка обязательна |
| Проверка корректности эмбеддингов | CI с кэшированной моделью | merge, если проверка обязательна |
| `forgeplan health` | CI с фильтром путей; локальный хук предупреждает | merge, если проверка обязательна |
| `forgeplan validate --ci` | CI с фильтром путей | merge, если проверка обязательна |
| Проверка числа MCP-инструментов | `ci.yml` (каждый PR, push dev) | merge, если проверка обязательна |
| Сборка сайта, drift команд, CLI smoke | `ci.yml` | merge, если проверка обязательна |
| `cargo-deny check` | push `dev`/`main`, еженедельно, ручной запуск | security-сигнал после merge; не PR-задание |

Хуки в `.claude/hooks/` перехватывают настроенные вызовы инструментов
Claude Code, а не любые операции shell или Git. CI описан в `ci.yml`,
`forgeplan-health.yml` и `security.yml`; обязательность проверок для
merge задают GitHub rulesets.

`security.yml` выполняет `cargo deny check advisories licenses bans sources`
после push, а не на `pull_request`. Перед тегированием проверь security
результат точного SHA релиза; зелёный PR сам по себе не включает этот скан.

---

## 1. Форматирование: `cargo fmt --check`

**Назначение:** проверяет, что исходный код Rust форматирован согласно `rustfmt`
(нет несохранённых diff'ов после автоформата).

**Когда запускается:**
- `pre-commit-fmt.sh` — хук Claude Code `PreToolUse` для Bash-команд,
  содержащих `git commit`; при ошибке `cargo fmt --check` возвращает exit 2.
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

Форматирование обязательно перед каждым коммитом (CLAUDE.md §Rust coding rules).
Настроенный хук Claude Code блокирует перехваченные коммиты, но не
устанавливает Git pre-commit hook и не охватывает другие клиенты.

---

## 2. Статический анализ: `cargo check` и `cargo clippy`

**Назначение:** `cargo check` проверяет компилируемость без сборки бинарей.
`cargo clippy` добавляет lint-правила с `-D warnings` — при любом
предупреждении возвращает exit 1.

**Когда запускается:**
- Локально: после каждого изменения кода (рекомендуется в рабочем процессе).
- CI: каждый PR к `dev` или `main`.

**Команды CI:** `ci.yml` задаёт `RUSTFLAGS="-Dwarnings"` и выполняет:
```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets
cargo check --workspace --all-targets --features semantic-search
cargo clippy --workspace --all-targets --features semantic-search
```

**Локально:** те же команды с `RUSTFLAGS="-Dwarnings"`.
`test-helpers` — не конфигурация CI; компиляция `semantic-search` не
заменяет отдельную проверку корректности эмбеддингов.

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

**Назначение:** выполняет тесты workspace. В CI тестовые бинарные файлы
запускаются через `nextest`, doc-tests — отдельно; вычисления модели
проверяются в отдельном задании.

**Когда запускается:**
- CI: каждый PR + push к `dev`.
- Локально: обязательно перед каждым коммитом (CLAUDE.md §Rust coding rules п.3).

**Как запустить локально:**
```bash
cargo nextest run --workspace --all-targets                 # тестовые бинарные файлы в CI
cargo test --workspace --doc                               # doc-tests в CI
cargo test --workspace                                    # альтернативный локальный запуск
```

Задание `Embedding correctness oracle` отдельно запускает эталонные
тесты с feature-флагом и кэшированной моделью ~2.1 GB. Защитная переменная
превращает отсутствие модели в ошибку, а не тихий пропуск. Обычный запуск
тестов workspace не проверяет эту конфигурацию.

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

**Назначение:** сообщает о состоянии графа артефактов. Orphans — артефакты
без входящих и исходящих связей; blind spots — активные решения без evidence.
Это не счётчики расхождений файлов с индексом. `--ci --fail-on`
возвращает exit 1 при превышении заданных порогов.

**Когда запускается:**
- `.claude/hooks/pre-commit-health.sh` — рекомендательный хук Claude Code, не блокирующий Git hook.
- `.github/workflows/forgeplan-health.yml` step `Health check` — CI.

**CI-команда:**
```bash
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"
```

Пороги: `orphans=10` допускает до 10 артефактов без связей;
`blind_spots=5` допускает до 5 активных решений без evidence.
Превышение любого порога завершает этот шаг CI с ошибкой.

**Как запустить локально:**
```bash
forgeplan health                                         # обзор состояния
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"  # полная CI-проверка
```

**Ограничение JSON в 0.38.0 (core #545):** `health --json --ci`
завершается в JSON-ветке до проверки порогов `--ci --fail-on`.
Для этого гейта оставь текстовые команды выше: JSON-вывод не даёт
эквивалентного кода завершения. У `health --strict --json` другой,
более строгий контракт (включая ошибку при любом orphan или blind spot);
это не замена с сохранением заданных порогов.

**Как исправить:**

*Orphan-артефакты* (нет связей): изучи `forgeplan get <ID>` и добавь
содержательную связь с решением или evidence. Переиндексация не создаёт
недостающие связи. Если устарел индекс файлов, используй:
```bash
forgeplan reindex       # синхронизация существующих .forgeplan markdown → LanceDB
forgeplan health
```
`scan-import` ищет новые артефакты в произвольном markdown; это не
команда пересборки индекса.

*Blind spots* (активные решения без evidence): собери реальные результаты,
создай `forgeplan new evidence "Verification results"` и заполни пакет
через `forgeplan update`: методы, результаты, `verdict`,
`congruence_level`, `evidence_type`. Свяжи возвращённый ID:
```bash
forgeplan link <EVID-ID> <DECISION-ID> --relation informs
forgeplan validate <EVID-ID> --ci
forgeplan activate <EVID-ID>
forgeplan score <DECISION-ID>
```
Создание или активация пустого шаблона не заменяет доказательства.

*Stale артефакты* (TTL истёк):
```bash
forgeplan stale                    # посмотреть список
forgeplan renew <ID> --reason "Reverified against current sources" --until <YYYY-MM-DD>
# или если устарели совсем:
forgeplan deprecate <ID> --reason "obsolete"
```

**Файл workflow:** `.github/workflows/forgeplan-health.yml`

---

## 5. Методологические гейты: `forgeplan validate` и `forgeplan score`

**Назначение:**
- `forgeplan validate <ID>` — проверяет артефакт на соответствие схеме
  (MUST-секции, frontmatter, формат). Для exit 1 при MUST-ошибках нужен `--ci`.
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
~18 мест в документации с устаревшими счётчиками MCP-инструментов
(несколько разных исторических значений против актуального count на момент
v0.28.0). Этот script предотвращает повторение: каждый PR, который
добавляет или удаляет MCP-инструмент, автоматически провалит CI, пока
документация не обновлена. Конкретные числа здесь намеренно не
закреплены — чтобы этот раздел сам не стал drift-мишенью; live-count
эмитит `scripts/check-mcp-tool-count.sh`.

**Источник счётчика:** зарегистрированные атрибуты `#[tool(...)]` в
`crates/forgeplan-mcp/src/server.rs`. Подсчёт `async fn forgeplan_*`
захватывает и тестовые функции без регистрации, завышая результат.

**Файл скрипта:** `scripts/check-mcp-tool-count.sh`

**Когда запускается:**
- Отдельное задание `Check MCP tool count drift` в `.github/workflows/ci.yml`:
  каждый PR к `dev`/`main` и push в `dev`, без фильтра путей.
  Сначала проверяет собственное регулярное выражение, затем документацию.
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

**Типичный вывод при успехе** (числа меняются от релиза к релизу;
`<ACTUAL>` — число зарегистрированных инструментов, которое выводит скрипт):
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

1. Выяснить фактическое число:
   ```bash
   grep -cE '^[[:space:]]*#\[tool\(' crates/forgeplan-mcp/src/server.rs
   ```

2. Обновить все указанные в drift-выводе места. Замените старое число
   на актуальное (то, что вернул `grep` на шаге 1) — в `CLAUDE.md`,
   `README.md` и т.д., где упомянут MCP tool count.

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

**Правило для авторов инструментов:** при добавлении нового MCP-инструмента
(`async fn forgeplan_<name>`) **обязательно** обновите счётчики в CLAUDE.md
`## Current status` и README.md до создания PR. Иначе CI упадёт на
drift-check step.

---

## 7. Pre-commit hooks — обзор (`.claude/hooks/`)

Хуки перехватывают подходящие вызовы Claude Code, если подключены в
`.claude/settings.json`. Они не выполняются для любой операции Git
и не заменяют CI или серверные rulesets.

| Файл хука | Что блокирует | Когда |
|---|---|---|
| `forge-safety-hook.sh` | Деструктивные команды (`rm -rf /`, `cargo publish`, `DROP TABLE`, `git push --force`) | pre-tool-use в Claude Code |
| `pre-commit-fmt.sh` | Коммит при ошибке `cargo fmt --check` | Claude Code Bash `git commit` |
| `commit-test-check.sh` | Коммит при срабатывании эвристики «новая функция без теста» | Claude Code Bash `git commit` |
| `pr-todo-check.sh` | Создание PR с незакрытыми P0-пунктами TODO | Claude Code Bash `gh pr create` / `gh pr submit` |
| `pre-commit-health.sh` | Предупреждает о blind spots (exit 1, не блокирующий exit 2) | Claude Code Bash `git commit` |
| `pre-pr-evidence-check.sh` | Распознанные ссылки на артефакты без обнаруженной связи с evidence | Claude Code Bash, распознанный `gh pr create` |

**Важно:** хуки расположены в `.claude/hooks/` (для Claude Code integration),
но **не** в стандартном `.git/hooks/`. Они активируются Claude Code средой
как `PreToolUse` хуки. Стандартные git pre-commit хуки работают независимо
(при наличии).

`pre-pr-evidence-check.sh` подключён в `.claude/settings.json`.
Он распознаёт `gh pr create` только первой командой после пробелов
и простых присваиваний переменных окружения. Извлекает числовые ID
артефактов из имени ветки и последних 20 коммитов, пропускает EVID/NOTE
и проверяет наличие связи с evidence (по графу, затем эвристикой секции
связей), а не качество или полноту доказательств. Нет распознанных refs —
проверка пропускается; при отсутствии бинаря Forgeplan тоже лишь предупреждает.

Обходы: `FORGEPLAN_SKIP_EVIDENCE=1` и ветки `docs/*`, `chore/sync-*`,
`chore/dependabot-*`, `release/v*`, `hotfix/*`. Хук также распознаёт
свой токен `--no-evidence-check` в начале аргументов до первой кавычки,
но не удаляет его и не превращает в поддерживаемую опцию `gh`.
Обнаруженное отсутствие evidence блокирует вызов с exit 2; разрешённый
вызов не доказывает проверку всех нужных артефактов или результатов.

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

Для сквозного CLI smoke используй скрипты с временными workspace,
а не создавай незаполненный PRD в рабочем проекте:
```bash
bash scripts/smoke-test.sh --verbose
bash scripts/check-doc-command-drift.sh --bin target/debug/forgeplan
bash scripts/cli-surface-exercise.sh --bin target/debug/forgeplan
```
Сначала собери бинарь. Проверки, которым нужны модель или LLM,
могут получить отметку `EXTERNAL`; это не успешная функциональная проверка.

---

## Anatomy CI workflow: `forgeplan-health.yml`

**Файл:** `.github/workflows/forgeplan-health.yml`

**Триггер:** PR к `dev` или `main`, при изменениях в `.forgeplan/**` или
`crates/**`.

**Шаги (в порядке выполнения):**

1. Checkout (action закреплён по SHA)
2. Установка системных зависимостей (`protobuf-compiler`)
3. Установка stable Rust и настройка кэша Cargo
4. `cargo build -p forgeplan`
5. **Пересборка индекса** — `./target/debug/forgeplan reindex`
6. **Health check** — `./target/debug/forgeplan health --ci --fail-on "orphans=10,blind_spots=5"`
7. **Validate artifacts** — `./target/debug/forgeplan validate --ci`

Каждый шаг проверяет код завершения; после ошибки последующие обычные
шаги пропускаются. Форматирование, тесты и проверка числа MCP-инструментов
находятся в отдельных заданиях `ci.yml`, а не дальше в health-задании.

---

## Частые сценарии и решения

### "PR заблокирован drift-check'ом после добавления нового MCP tool"

```bash
# 1. Узнать текущий count
grep -cE '^[[:space:]]*#\[tool\(' crates/forgeplan-mcp/src/server.rs

# 2. Найти места, которые нужно обновить
./scripts/check-mcp-tool-count.sh --warn

# 3. Обновить каждое указанное место (CLAUDE.md, README.md, website/, docs/)

# 4. Если изменён CHANGELOG.md — регенерировать website
cd website && node scripts/copy-changelog.mjs

# 5. Проверить
./scripts/check-mcp-tool-count.sh   # exit 0
```

### "forgeplan health падает в CI, но локально всё хорошо"

Одна из возможных причин — устаревший локальный индекс. CI пересобирает
tracked markdown через `reindex`, а не `init` плюс `scan-import`.
Сравни точный checkout PR во временном клоне и выполни:

```bash
./target/debug/forgeplan reindex
./target/debug/forgeplan health --ci --fail-on "orphans=10,blind_spots=5"
./target/debug/forgeplan validate --ci
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
