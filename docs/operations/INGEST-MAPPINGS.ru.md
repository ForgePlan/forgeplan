[English coming soon] · [Русский](INGEST-MAPPINGS.ru.md)

# Ingest Mapping Authoring Guide (v0.26.0+)

Гайд для авторов mapping'ов — декларативных YAML-файлов, которые
переводят output внешних плагинов (C4 docs, autoresearch summaries,
git logs, DDD models, SPARC specs) в forge-артефакты
(PRD/ADR/Epic/Note/Spec) с обязательной `## Sources` секцией.

## Что такое Mapping

**Mapping** — это `.yaml`-файл, описывающий **правила трансформации**
от структурированного output одного плагина к набору forge-артефактов.
Сам ingest engine — declarative: он применяет ваши правила к каждому
найденному source-файлу и создаёт артефакты с типизированными
полями + `## Sources` block.

Mapping **никогда не содержит исполняемого кода** — только селекторы,
шаблоны полей с whitelist-фильтрами и invariants. Это security-граница
ADR-009: пользователь, ставящий чужой mapping pack, не получает
arbitrary code execution.

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
| `marketplace/mappings/*.yaml` (этот репозиторий) | Канонические mapping'и Forgeplan |
| `.forgeplan/mappings/*.yaml` | Workspace-локальные (не публикуются) |

`forgeplan ingest --mapping <path>` принимает путь к существующему YAML-файлу,
не короткое registry-имя.

## Структура YAML — top-level поля

Полный референс — SPEC-004 §"Top-level fields". Минимально:

```yaml
schema_version: "1.0"           # формат самой схемы (semver)
name: c4-to-forge               # уникальный kebab-case идентификатор
title: "C4 architecture docs → Forge notes + epic link"
compat_spec_version: "c4-architecture: ^1.0"
source_kind: c4-documentation   # один из 5 (см. ниже)
target_kind: forge              # пока только 'forge'
sources:                        # discovery rules — где искать input
  - pattern: "C4-Documentation/code/**/*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:                          # transformation rules
  - id: ...
    when: { ... }
    target: { kind: note }
    fields: { ... }
    sources_section: { include: true, ... }
```

### `compat_spec_version` — версионирование upstream

Это **отдельный semver** от `schema_version`. Он фиксирует диапазон
output-форматов **upstream-плагина**, которые этот mapping корректно
переводит:

```yaml
compat_spec_version: "c4-architecture: ^1.0"
```

Поле фиксирует заявленную совместимость. В core есть semver-проверка, но текущие
CLI/core-dispatcher не устанавливают версию upstream автоматически и не дают
гарантии такого отказа. Автор должен сверить фактический формат источника;
наличие поля само по себе не доказывает совместимость.

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

`pattern` — glob относительно корня проекта. `type` — informational
тег. `parser` — strict enum (см. ниже), решает, как парсить файл:

| Parser | Что делает |
|---|---|
| `front_matter_plus_sections` | YAML frontmatter + Markdown `## sections` |
| `markdown_only` | Markdown без frontmatter |
| `log_with_blame` | Разбирает переданный текст git log; сам не запускает git log или git blame |
| `json` | JSON-документ |
| `yaml` | YAML-документ |

Произвольный кастомный парсер — не поддерживается. Если нужен
новый формат — открывайте PR в forgeplan-core с расширением enum
`Parser` (см. `crates/forgeplan-core/src/ingest/types.rs`).

## Селектор `when`

`file_glob`, `front_matter` и `contains_section` проверяются вместе.
Несовпадение пропускает правило без ошибки. В `contains_section` указывается
текст заголовка без Markdown-префикса `##`.

```yaml
when:
  file_glob: "C4-Documentation/components/*.md"
  front_matter:
    kind: component             # все ключи должны совпасть
  contains_section: "Purpose"
  heading_path: ["Purpose"]
```

`heading_path` выбирает раздел для шаблона. Например,
`["Code Elements", "Public Functions", "*"]` выбирает непосредственные
подзаголовки `Public Functions` и формирует отдельный draft для каждого.
Промежуточные именованные заголовки должны быть вложены друг в друга.
Если путь не найден, текущий движок возвращается к контексту всего файла;
не используйте `heading_path` как единственный фильтр допуска к импорту.
Проверяйте выбор правила через `ingest --dry-run`. Его JSON содержит
заголовок, тип, источник, hash, правило и связи, но не текст тела.
Верность переноса текста проверяйте через `get` после импорта в песочнице.

Без выбранного раздела доступны `front_matter`, `path` и `line_count`.
Выбор раздела добавляет `heading_text` и `section` с полями `body`,
`heading_text`, `heading_level`, `line_start`, `line_end`, `sub_sections`.
`contains_section` только проверяет наличие раздела, но не выбирает его
для шаблона. Семантических полей `section.purpose` и
`section.responsibilities` нет: для примера выше текст Purpose находится
в `section.body`.

## Шаблоны `fields` — Tera + whitelist

Forge-поля — строковые Tera-шаблоны с проверкой имён фильтров по whitelist.
Текущая проверка обходит Tera AST, но не устанавливает описывавшийся ранее
полный запрет циклов/условий. Не считайте whitelist полноценным sandbox.

```yaml
fields:
  title: "{{ front_matter.name | default(value='Unnamed component') | trim }}"
  summary: "{{ section.body | default(value='Auto-imported') | trim }}"
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
| `truncate(length=50)` | Обрезать до заданной длины |
| `default(value="...")` | Подставить fallback, если значение отсутствует |
| `replace(from="a", to="b")` | Substring replace |
| `table` | Markdown-таблица из map (added per EVID-088) |

### Tera caveat — `default` syntax

В Tera (в отличие от Jinja) фильтр `default` принимает **named**
аргумент `value`, не позиционный. Пишите:

```yaml
title: "{{ heading_text | default(value='Code element') }}"
```

**Не пишите** `default('Code element')` или `truncate(50)`:
используйте именованные аргументы Tera, например `default(value='Code element')`
и `truncate(length=50)`. Старый пример позиционного `truncate` был ошибкой.

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
  precision: line                # line | block | file
  source_hash: true              # для idempotency (PRD-066 AC-3, FR-5)
```

`include: false` отклоняется schema validation. Требование `## Sources` сохраняет
проверяемую ссылку на источник, но не делает вывод автоматически истинным.
Зарегистрированного `forgeplan doctor --sources` в выбранной сборке нет.
`source_hash` помогает повторному ingest определить изменение; применение
изменённого результата требует `--update`. Удаление исходного файла не переводит
артефакт автоматически в `stale`.

## Target kind — куда ингестим

```yaml
target:
  kind: note                     # prd | adr | epic | note | spec | problem
```

**Гайдлайн** (per EVID-088): для **code-derived** артефактов
(C4 docs, DDD models) предпочитайте `note`, не `prd`/`spec`. PRD/SPEC
имеют MUST-секции (Non-Goals, FR, Errors), которых **не существует**
в structural docs — validation gate отвергнет partial-artifact.
`note` skip-ает гейт; пользователь промоутит note → prd/spec
вручную после enrichment.

Для **product-derived** input'а (autoresearch summaries, SPARC
specs) — `prd`/`spec` корректны, потому что upstream даёт нужные
секции.

## `links` — auto-graph creation

```yaml
links:
  - target: "{{ front_matter.parent_container }}"
    relation: refines             # informs | based_on | refines | contradicts | supersedes
    if_exists: skip               # skip | warn | error
  - target_artifact_id: "EPIC-007"
    relation: refines
```

`target` — templated (резолвится из source-данных). `target_artifact_id` —
статичная ссылка. Используйте обе формы: статичная для anchor'а
(«всё C4 импортируется под EPIC-007»), templated для cross-link'ов
между ингестируемыми артефактами.

## `guards` — safety limits

```yaml
guards:
  max_artifacts: 100              # abort если mapping создаст больше
  require_section: ["Sources"]    # все артефакты должны содержать
  forbid_overwrite_active: true   # не апдейтить activated артефакты
```

Без явного `max_artifacts` engine всё равно применяет default 10 000; меньший
предел позволяет ограничить конкретный импорт. Значение 50–200 — пример
настройки, не измеренная универсальная норма.

`forbid_overwrite_active: true` объявлен в schema как защитное намерение.
В текущем CLI `write_draft` не найдено проверки lifecycle status перед
обновлением совпавшего по kind/title файла; полагаться на этот флаг как на
исполненный запрет нельзя. Политика сохранения active-артефактов остаётся
обязательной; не запускайте `--update`, предполагая такую автоматическую защиту.

## Five `source_kind` enum

`c4-documentation`, `autoresearch`, `git-log`, `ddd-model`, `sparc-spec`.
Каждый mapping декларирует **один**. Расширение enum — PR в
forgeplan-core (`crates/forgeplan-core/src/ingest/types.rs`).

## Six target artifact kinds

`prd`, `adr`, `epic`, `note`, `spec`, `problem`. Не путайте с
**top-level** `target_kind: forge` — последнее обозначает back-end
(пока только Forge), первое — конкретный тип артефакта на rule-level.

## Idempotency

При включённом `source_hash` движок добавляет HTML body comment, не поле
frontmatter. Хеш учитывает rule ID и текст источника.

- Совпали kind/title и source hash — skip.
- Источник изменился — по умолчанию skip; запись требует `--update`.
- Источник исчез — автоматического изменения status нет.

Это механизм повторного импорта, не доказательство всех AC PRD-066 и не
проверка существования каждой цитаты.

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
| Source `pattern` не нашёл файлов | WARN |
| Generated artifact field validation fails | ERROR per artifact, mapping продолжает |
| Idempotent re-run, same `source_hash` | INFO (skip) |
| Idempotent CLI re-run, изменённый content | Skip по умолчанию; запись только с `--update` |
| `guards.max_artifacts` превышен | ERROR (abort + partial-state report) |

## Запуск

```bash
# Dry-run — посмотреть, что будет создано
forgeplan ingest --mapping marketplace/mappings/c4-to-forge.yaml --source C4-Documentation/ --dry-run

# Реальный run
forgeplan ingest --mapping marketplace/mappings/c4-to-forge.yaml --source C4-Documentation/

# Принудительный update (вместо warn о diff)
forgeplan ingest --mapping marketplace/mappings/c4-to-forge.yaml --source C4-Documentation/ --update
```

## Workflow integration: ingest из playbook (Phase 6)

С v0.27.0 шаги `delegate_to: { type: forgeplan_core, target: ingest }`
в playbook'ах вызывают ingest engine **прямым internal API call** в
текущем процессе forgeplan, без subprocess или CLI shell-out (как
было в Phase 5).

Это прямой in-process вызов, но dispatcher открывает свежий `LanceStore` handle:
он не гарантирует повторное использование уже открытого соединения или общего
LLM state. Нужен типизированный `input` шага с `mapping_path`, `source_path`
и при необходимости `dry_run`/`update`; одних полей шага `mapping`/`produces_at`
недостаточно для аргументов ForgeplanCoreDispatcher.

Engine собирает drafts; запись выполняет вызывающий путь. CLI ingest пишет.
Full MCP ingest выполняет проверки допустимости путей и пределов размера,
отражает параметры с `wave3_status: "planned"` и возвращает пустые массивы
`drafts`, `skipped`, `errors`. Он не вызывает `IngestEngine` и не составляет
draft-план. Проверяйте layout и контракт конкретного caller: одно имя операции
не означает одинаковую запись или security boundary. Journal playbook
не обеспечивает exactly-once применение или автоматическое возобновление.

Дополнительно internal `run_ingest` принимает, но не использует `update`;
не считайте это тем же overwrite/idempotency-контрактом, что CLI.

См. [ADR-010 §Affected Files](../../.forgeplan/adrs/ADR-010-phase-6-subprocess-invocation-via-tokio-process-with-kill-on-drop-and-timeout.md)
+ [PLAYBOOK-AUTHORING.ru.md §Subprocess lifecycle](PLAYBOOK-AUTHORING.ru.md#subprocess-lifecycle-phase-6).

## Кросс-ссылки

- [SPEC-004 — Mapping YAML schema](../../.forgeplan/specs/SPEC-004-mapping-yaml-schema.md) — формальный контракт
- [SPEC-003 — Playbook YAML schema](../../.forgeplan/specs/SPEC-003-playbook-yaml-schema.md) — `step.mapping` ссылается сюда
- [PRD-066 — Ingest engine](../../.forgeplan/prds/PRD-066-ingest-engine-mapping-yaml-format-c4-to-forge-autoresearch-to-forge-git-to-forge-ddd-to-forge-spec-to-forge.md) — runtime контракт
- [ADR-009 — Forgeplan as orchestrator](../../.forgeplan/adrs/ADR-009-forgeplan-as-orchestrator-playbook-skill-agent-mapping-pack-marketplace-model.md) — hallucination-proof invariant
- [EVID-088 — Spike-1 c4-to-forge concept validation](../../.forgeplan/evidence/EVID-088-spike-1-c4-to-forge-mapping-concept-validated-on-scoring-module.md) — measurement-based gaidlines (target=note default, table filter, default(value=...))
- [PLAYBOOK-AUTHORING.ru.md](PLAYBOOK-AUTHORING.ru.md) — как ссылаться на mapping из playbook step
- [marketplace/mappings/c4-to-forge.yaml](../../marketplace/mappings/c4-to-forge.yaml) — reference-пример с условиями применимости: нужен совместимый source contract для semantic fields; `heading_path` с `*` трактуется буквально. Не готовый исполняемый шаблон для копирования.
