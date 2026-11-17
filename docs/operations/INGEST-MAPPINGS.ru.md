[English coming soon] · [Русский](INGEST-MAPPINGS.ru.md)

# Ingest Mapping Authoring Guide (Forgeplan 0.38.0)

Гайд для авторов mapping'ов — декларативных YAML-файлов, которые
переводят output внешних плагинов (C4 docs, autoresearch summaries,
git logs, DDD models, SPARC specs) в forge-артефакты
(PRD/ADR/Epic/Note/Spec) с обязательной `## Sources` секцией.

## Что такое Mapping

**Mapping** — это `.yaml`-файл, описывающий **правила трансформации**
от структурированного output одного плагина к набору forge-артефактов.
Движок применяет правила к разобранным источникам и возвращает черновики с `## Sources`.
CLI `forgeplan ingest` может записать их в workspace. MCP `forgeplan_ingest` в 0.38.0
только проверяет входные пути и возвращает план со ссылкой на CLI: пустой `drafts`
в этом ответе не доказывает, что mapping был применён.

Mapping не запускает shell-команды. Поля используют Tera-шаблоны с проверкой
списка разрешённых фильтров. Это не гарантия истинности исходного текста:
наличие `## Sources` обеспечивает ссылку на происхождение, не проверку выводов.

Источник истины — [SPEC-004](../../.forgeplan/specs/SPEC-004-mapping-yaml-schema.md).

## Когда писать mapping

- Вы интегрируете **новый внешний плагин** в forge-граф (своими
  руками или для marketplace).
- Существующий плагин **выпустил новую major version** — нужен
  mapping с обновлённым `compat_spec_version`.
- Вы хотите **специализировать** дефолтный mapping для своего pack'а
  (например, target=`spec` вместо `note` для определённого heading
  pattern).

Когда mapping **не нужен**: одиночный импорт через `forgeplan new`
вручную, ad-hoc парсинг скриптом, генерация артефактов из шаблона.

## Где хранить

| Локация | Назначение |
|---|---|
| `<pack>/mappings/*.yaml` | Канонические mapping'и в составе marketplace pack |
| `marketplace/mappings/*.yaml` | Возможный каталог в проекте; наличие файлов зависит от checkout |
| `.forgeplan/mappings/*.yaml` | Workspace-локальные (не публикуются) |

`forgeplan ingest --mapping <path>` принимает **путь к существующему YAML**,
а не имя из registry. Пакет не становится доступным по короткому имени после установки.

## Структура YAML — top-level поля

Полный референс — SPEC-004 §"Top-level fields". Минимально:

```yaml
schema_version: "1.0"           # формат самой схемы (semver)
name: c4-to-forge               # уникальный kebab-case идентификатор
title: "C4 architecture docs → Forge notes + epic link"
compat_spec_version: "c4-architecture: ^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "**/*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: component-note
    when:
      front_matter:
        kind: component
    target:
      kind: note
    fields:
      title: "{{ front_matter.name | default(value='Imported component') | trim }}"
      summary: "Imported from {{ path }}"
    sources_section:
      include: true
guards:
  max_artifacts: 100
```

### `compat_spec_version` — версионирование upstream

Это **отдельный semver** от `schema_version`. Он фиксирует диапазон
output-форматов **upstream-плагина**, которые этот mapping корректно
переводит:

```yaml
compat_spec_version: "c4-architecture: ^1.0"
```

В 0.38.0 диапазон парсится и доступен для проверки через `version_compat`,
но CLI ingest не определяет версию upstream и не вызывает эту проверку.
Автор должен сам сопоставить формат входа с mapping до записи.

## Источники — `sources`

```yaml
sources:
  - pattern: "C4-Documentation/components/**/*.md"
    type: markdown
    parser: front_matter_plus_sections
  - pattern: ".git/log"
    type: git_log
    parser: log_with_blame
```

В режиме каталога CLI сопоставляет `pattern` с относительным путём от `--source`
и с полным путём. Если `--source` — один файл, при отсутствии совпадения используется
первый SourceSpec. `type` — информационная метка; разбор выбирает `parser`:

| Parser | Что делает |
|---|---|
| `front_matter_plus_sections` | YAML frontmatter + Markdown `## sections` |
| `markdown_only` | Markdown без frontmatter |
| `log_with_blame` | Разбирает сохранённый текст стандартного `git log` по границам `commit`; сам git/blame не запускает |
| `json` | JSON-документ |
| `yaml` | YAML-документ |

Произвольный кастомный парсер — не поддерживается. Если нужен
новый формат — открывайте PR в forgeplan-core с расширением enum
`Parser` (см. `crates/forgeplan-core/src/ingest/types.rs`).

## Селектор `when`

Все указанные подселекторы AND-комбинируются. Несовпадение — rule
**пропускается** (не error).

```yaml
when:
  file_glob: "C4-Documentation/components/*.md"
  front_matter:
    kind: component             # все ключи должны совпасть
  contains_section: "Purpose"
  heading_path: ["Code Elements", "Public Functions", "*"]
```

`heading_path` задаёт путь заголовков без `#`; завершающий `"*"` выбирает
дочерние секции. В 0.38.0 отсутствие совпадений может привести к одному черновику
по всему источнику, а не пропуску. Проверяйте число и содержимое `drafts` в dry-run.
`contains_section` сопоставляет текст заголовка, например `Purpose`, не `## Purpose`.

## Шаблоны `fields` — Tera + whitelist

Поля — строковые Tera-шаблоны. Загрузка проверяет синтаксис и имена фильтров;
не следует обещать, что все конструкции Tera, кроме path lookup, запрещены.
Доступны `front_matter`, `path`, `line_count`; при выборе секции — `heading_text`
и `section` с `body`, `heading_text`, `heading_level`, `line_start`, `line_end`, `sub_sections`.

```yaml
fields:
  title: "{{ heading_text | trim }}"
  summary: "{{ section.body | default(value='Auto-imported') | trim }}"
  details: "{{ section.body | bullet_list }}"
  tags: "c4,component,auto-imported"
```

### Whitelist фильтров (10)

| Фильтр | Назначение |
|---|---|
| `trim` | Убрать whitespace по краям |
| `lower` / `upper` | Регистр |
| `bullet_list` | Преобразовать lines в Markdown bullet list |
| `comma_list` | Объединить через `, ` |
| `slugify` | Превратить в slug (`my-title-here`) |
| `truncate(length=50)` | Обрезать строку до заданной длины |
| `default(value="...")` | Подставить значение при отсутствии данных |
| `replace(from="old", to="new")` | Заменить подстроку |
| `table` | Markdown-таблица из map (added per EVID-088) |

### Tera caveat — `default` syntax

В Tera (в отличие от Jinja) фильтр `default` принимает **named**
аргумент `value`, не позиционный. Пишите:

```yaml
title: "{{ section.purpose | default(value='Code element') }}"
```

**Не пишите** `default('Code element')` или `truncate(50)`: используйте
именованные аргументы Tera (`value=...`, `length=...`).
Проверка разрешённого имени фильтра не гарантирует корректность его аргументов:
ошибки рендеринга видны при применении mapping.

### Любой фильтр вне whitelist → load error

```yaml
fields:
  title: "{{ heading | uppercase }}"   # ❌ uppercase не в списке
```

Loader отвергнет mapping с описанием: «filter `uppercase` not allowed,
expected one of: trim, lower, upper, bullet_list, ...». Это
**security invariant** — нельзя добавить custom filter через mapping
pack для произвольного выполнения кода.

## `sources_section` — invariant ADR-009

```yaml
sources_section:
  include: true                  # ОБЯЗАТЕЛЬНО true
  format: "{path}:{line_start}-{line_end}"
  precision: line                # поле схемы; диапазон задаёт выбранная секция/файл
  source_hash: true              # хеш-маркер всё равно обеспечивается движком
```

`include: false` отвергается при загрузке. Для выбранной секции источник
цитируется с её диапазоном строк, иначе — с диапазоном всего файла.
Это происхождение данных, а не доказательство достоверности.

В 0.38.0 нет `forgeplan doctor --sources` и автоматического перевода артефакта
в `stale` при удалении источника. Ссылки и актуальность нужно проверять отдельно.

## Target kind — куда ингестим

```yaml
target:
  kind: note                     # prd | adr | epic | note | spec | problem
```

Для структурных документов C4/DDD разумно выбирать `note`, а не автоматически
считать их требованиями продукта. PRD/SPEC требуют собственных MUST-секций.
CLI ingest записывает черновики, но не запускает полную валидацию или активацию.
Проверяйте результат отдельным `forgeplan validate <ID>`.
Переход Note → PRD/Spec не является автоматической операцией ingest.

Для product-derived входа (autoresearch summaries, SPARC specs) PRD/Spec подходят,
только если источник действительно содержит нужные требования и секции.
Само имя upstream-плагина не гарантирует полноту результата.

## `links` — auto-graph creation

```yaml
links:
  - target: "{{ front_matter.parent_container }}"
    relation: refines             # informs | based_on | refines | contradicts | supersedes
    if_exists: skip               # skip | warn | error
  - target_artifact_id: "EPIC-007"
    relation: refines
```

`target` — шаблон, `target_artifact_id` — статичный ID. Задавайте одну форму
в каждой записи. Статичный ID подходит для существующего anchor-артефакта.
Не рассчитывайте на автоматическое разрешение названий новых черновиков в ID.
В CLI связи создаются отдельно от тела; `if_exists: error` может вывести ошибку
связи без отката созданного артефакта. После записи проверяйте связи.

## `guards` — safety limits

```yaml
guards:
  max_artifacts: 100              # предел числа черновиков до записи
  require_section: ["Sources"]    # поле схемы; в 0.38.0 не применяется
  forbid_overwrite_active: true   # поле схемы; в 0.38.0 не защищает запись
```

`max_artifacts` ограничивает число **черновиков** до записи; без явного значения
действует лимит движка. Задайте подходящий размер для своего входа, например 50–200.

**Ограничение 0.38.0:** `require_section` и `forbid_overwrite_active` есть в схеме,
но движок передаёт их как неиспользуемый `_guards`, а CLI `update_existing`
не проверяет статус. `forbid_overwrite_active: true` нельзя считать защитой
активных артефактов. До `--update` отдельно проверьте статусы и разрешение на изменение.

## Five `source_kind` enum

`c4-documentation`, `autoresearch`, `git-log`, `ddd-model`, `sparc-spec`.
Каждый mapping декларирует **один**. Расширение enum — PR в
forgeplan-core (`crates/forgeplan-core/src/ingest/types.rs`).

## Six target artifact kinds

`prd`, `adr`, `epic`, `note`, `spec`, `problem`. Не путайте с
**top-level** `target_kind: forge` — последнее обозначает back-end
(пока только Forge), первое — конкретный тип артефакта на rule-level.

## Idempotency

CLI ищет существующий артефакт по **типу и точному заголовку** и сравнивает
маркер тела `<!-- source_hash: ... -->`, а не frontmatter-поле.
Хеш зависит от ID правила и полного исходного текста:

- Тот же заголовок и тот же хеш → пропуск.
- Тот же заголовок, изменённый хеш → без `--update` пропуск, с `--update` замена тела.
- Изменился заголовок или нет маркера → может появиться новый артефакт.
- Удалён источник → автоматического `stale` или удаления нет.

В 0.38.0 движок обеспечивает маркер даже при `sources_section.source_hash: false`.
Повторный запуск с `--update` может заменить ручные дополнения. Dry-run показывает
черновики, но не прогнозирует все решения записи по существующим артефактам.

## Errors — частые ошибки

Полная матрица — SPEC-004 §Errors. Кратко:

| Условие | Severity |
|---|---|
| Отсутствует обязательное top-level поле | ERROR |
| Неизвестный `source_kind` | ERROR (показывает 5 валидных) |
| Rule `target.kind` неизвестен | ERROR (показывает 6 валидных) |
| Шаблон использует non-whitelisted filter | ERROR (показывает 10 разрешённых) |
| `sources_section.include: false` | **ERROR** (ADR-009 invariant) |
| Пустой `sources` или `rules` массив | ERROR |
| Source `pattern` не нашёл файлов | Пустой результат; CLI может завершиться успешно |
| Ошибка рендеринга правила | Запись в `errors`, обработка других сочетаний продолжается |
| Повтор с тем же типом, заголовком и хешем | Пропуск CLI |
| Изменённый хеш без `--update` | Пропуск CLI с причиной |
| `guards.max_artifacts` превышен | ERROR до записи черновиков |

## Запуск

```bash
# Подставьте путь к установленному mapping; имя registry не разрешается.
forgeplan ingest --mapping .forgeplan/mappings/c4-to-forge.yaml --source C4-Documentation/ --dry-run --json

# Запись после проверки drafts/errors и существующих статусов
forgeplan ingest --mapping .forgeplan/mappings/c4-to-forge.yaml --source C4-Documentation/ --json

# Обновление при изменившемся хеше; может заменить дополненное вручную тело
forgeplan ingest --mapping .forgeplan/mappings/c4-to-forge.yaml --source C4-Documentation/ --update --json
```

## Workflow integration: ingest из playbook (Phase 6)

`forgeplan_core` / `ingest` вызывает внутренний `IngestEngine::apply`, но его
запись **не эквивалентна CLI**. Передайте параметры в `input` явно:

```yaml
- id: ingest-c4
  delegate_to: { type: forgeplan_core, target: ingest }
  input:
    mapping_path: .forgeplan/mappings/c4-to-forge.yaml
    source_path: C4-Documentation/
    dry_run: true
```

Поля шага `mapping` и `produces_at` сами не запускают импорт и не подменяют
`input.mapping_path` / `input.source_path`. В 0.38.0 внутренняя запись выдаёт
новые ID, не делает CLI-проверку type+title+hash, игнорирует `update` и не добавляет
объявленные draft links. Не обещайте идемпотентный повтор playbook-ingest.
Для контролируемого обновления используйте отдельную CLI-операцию после просмотра.

MCP `forgeplan_ingest` не является этим внутренним dispatcher: он только проверяет
пути и предлагает CLI, даже при `dry_run: false`.
См. [PLAYBOOK-AUTHORING.ru.md](PLAYBOOK-AUTHORING.ru.md#subprocess-lifecycle-phase-6).

## Кросс-ссылки

- [SPEC-004 — Mapping YAML schema](../../.forgeplan/specs/SPEC-004-mapping-yaml-schema.md) — формальный контракт
- [SPEC-003 — Playbook YAML schema](../../.forgeplan/specs/SPEC-003-playbook-yaml-schema.md) — `step.mapping` ссылается сюда
- [PRD-066 — Ingest engine](../../.forgeplan/prds/PRD-066-ingest-engine-mapping-yaml-format-c4-to-forge-autoresearch-to-forge-git-to-forge-ddd-to-forge-spec-to-forge.md) — runtime контракт
- [ADR-009 — Forgeplan as orchestrator](../../.forgeplan/adrs/ADR-009-forgeplan-as-orchestrator-playbook-skill-agent-mapping-pack-marketplace-model.md) — hallucination-proof invariant
- [EVID-088 — Spike-1 c4-to-forge concept validation](../../.forgeplan/evidence/EVID-088-spike-1-c4-to-forge-mapping-concept-validated-on-scoring-module.md) — measurement-based gaidlines (target=note default, table filter, default(value=...))
- [PLAYBOOK-AUTHORING.ru.md](PLAYBOOK-AUTHORING.ru.md) — как ссылаться на mapping из playbook step
- [Публичный c4-to-forge mapping](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-brownfield-pack/mappings/c4-to-forge.yaml) — исторический формат пакета с `mapping_name`, `source_plugin`, `mappings`. Он **не соответствует** схеме core 0.38.0 (`name`, `title`, `rules` и другие обязательные поля) и не загружается напрямую через CLI. Для core используйте схему и полный минимальный пример этой страницы, а не копируйте этот файл без адаптации.

## Источники релиза 0.38.0

- [CLI ingest: пути и dry-run](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/ingest.rs#L127-L249), [идемпотентность и запись](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/ingest.rs#L500-L590).
- [Движок: лимит и неиспользуемые guards](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/ingest/engine.rs#L193-L309), [контекст шаблона и маркер](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/ingest/engine.rs#L484-L589).
- [MCP только планирует](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-mcp/src/server.rs#L9549-L9640); [внутренний dispatcher](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/playbook/dispatch/forgeplan_core_dispatcher.rs#L266-L352).
