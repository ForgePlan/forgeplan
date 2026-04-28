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

`forgeplan ingest --mapping <name|path>` принимает либо имя из
registry, либо путь к файлу.

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

При **breaking change** в upstream плагине ingest engine выдаёт hint
«mapping `c4-to-forge` несовместим с c4-architecture v2 — нужен
`c4-to-forge-2.0.yaml`», а не молча корраптит ваш форге-граф.

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
| `log_with_blame` | `git log` + `git blame` для inferred ADR |
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
  contains_section: "## Purpose"
  heading_path: ["Code Elements", "Public Functions", "*"]
```

`heading_path` — путь по уровням `#`/`##`/`###` заголовков. `"*"`
в конце означает «любой заголовок этого уровня — fan out» (создаст
N артефактов из N подзаголовков).

## Шаблоны `fields` — Tera + whitelist

Forge-поля задаются **строковыми шаблонами** в стиле Tera. Доступно
**только** path lookup + whitelist filters — никаких циклов, условий,
include'ов и custom functions:

```yaml
fields:
  title: "{{ heading_text | trim }}"
  summary: "{{ section.purpose | default(value='Auto-imported') | trim }}"
  details: "{{ section.responsibilities | bullet_list }}"
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
| `truncate(n)` | Обрезать до N символов |
| `default(value="...")` | Подставить fallback, если значение отсутствует |
| `replace(from, to)` | Substring replace |
| `table` | Markdown-таблица из map (added per EVID-088) |

### Tera caveat — `default` syntax

В Tera (в отличие от Jinja) фильтр `default` принимает **named**
аргумент `value`, не позиционный. Пишите:

```yaml
title: "{{ section.purpose | default(value='Code element') }}"
```

**Не пишите** `default('Code element')` — синтаксис позиционных
аргументов в Tera работает только для встроенных filter'ов вроде
`truncate(50)`, у `default` он отвергается. См. EVID-088 (W2-B
deviation note).

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

`include: false` — **schema validation fails**: ADR-009
hallucination-proof invariant. Каждый ингестированный артефакт обязан
указать источник с точностью до line range. Это даёт:

- **Аудит**: `forgeplan doctor --sources` сверяет, существует ли
  каждый цитируемый файл.
- **Idempotency**: `source_hash` позволяет переходу источника
  triggerнуть update артефакта без дубликатов.
- **Stale detection**: удалили source — артефакт получает status
  `stale`, не молча живёт с устаревшим контентом.

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

`max_artifacts` — критичен. Без лимита mapping на 10K-LOC C4-вывод
может создать сотни Note'ов и засрать workspace. Ставьте
realistic cap (50–200).

`forbid_overwrite_active: true` — дефолт; ingest никогда не мутирует
already-active артефакт (это нарушило бы immutability per CLAUDE.md).
Update идёт только в draft.

## Five `source_kind` enum

`c4-documentation`, `autoresearch`, `git-log`, `ddd-model`, `sparc-spec`.
Каждый mapping декларирует **один**. Расширение enum — PR в
forgeplan-core (`crates/forgeplan-core/src/ingest/types.rs`).

## Six target artifact kinds

`prd`, `adr`, `epic`, `note`, `spec`, `problem`. Не путайте с
**top-level** `target_kind: forge` — последнее обозначает back-end
(пока только Forge), первое — конкретный тип артефакта на rule-level.

## Idempotency

`source_hash: true` (default) → каждый ингестированный артефакт
получает frontmatter-поле `source_hash`. На повторный run:

- Source не изменился, артефакт существует → **skip** (no-op).
- Source изменился, артефакт существует → **update** + log diff.
- Source удалён → **stale** (не delete, чтобы не потерять manual
  enrichment).

Это покрывает PRD-066 AC-3 («не создаёт дубликатов») и AC-4
(`forgeplan doctor --sources` валидирует).

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
| Idempotent re-run, изменённый content | WARN (update + log diff) |
| `guards.max_artifacts` превышен | ERROR (abort + partial-state report) |

## Запуск

```bash
# Dry-run — посмотреть, что будет создано
forgeplan ingest --mapping c4-to-forge --source C4-Documentation/ --dry-run

# Реальный run
forgeplan ingest --mapping c4-to-forge --source C4-Documentation/

# Принудительный update (вместо warn о diff)
forgeplan ingest --mapping c4-to-forge --source C4-Documentation/ --update
```

## Workflow integration: ingest из playbook (Phase 6)

С v0.27.0 шаги `delegate_to: { type: forgeplan_core, target: ingest }`
в playbook'ах вызывают ingest engine **прямым internal API call** в
текущем процессе forgeplan, без subprocess или CLI shell-out (как
было в Phase 5).

Это значит:

- **No process overhead**: ingest шаг наследует уже загруженные
  workspace state, LanceDB connection и LLM provider config —
  не нужно re-init.
- **Shared journal**: журналируемые события идут в тот же
  `.forgeplan/journal/playbook-runs.jsonl` без race с CLI.
- **Same security boundary**: whitelist filters (10) + `## Sources`
  invariant + `compat_spec_version` остаются обязательными — direct
  call использует тот же `ingest::engine::run()`, что и CLI.

Pack authors могут полагаться, что ingest-шаг внутри playbook'а
ведёт себя идентично standalone `forgeplan ingest --mapping <name>`,
включая идемпотентность через `source_hash` и dry-run preview.

См. [ADR-010 §Affected Files](../../.forgeplan/adrs/ADR-010-phase-6-subprocess-invocation-via-tokio-process-with-kill-on-drop-and-timeout.md)
+ [PLAYBOOK-AUTHORING.ru.md §Subprocess lifecycle](PLAYBOOK-AUTHORING.ru.md#subprocess-lifecycle-phase-6).

## Кросс-ссылки

- [SPEC-004 — Mapping YAML schema](../../.forgeplan/specs/SPEC-004-mapping-yaml-schema.md) — формальный контракт
- [SPEC-003 — Playbook YAML schema](../../.forgeplan/specs/SPEC-003-playbook-yaml-schema.md) — `step.mapping` ссылается сюда
- [PRD-066 — Ingest engine](../../.forgeplan/prds/PRD-066-ingest-engine-mapping-yaml-format-c4-to-forge-autoresearch-to-forge-git-to-forge-ddd-to-forge-spec-to-forge.md) — runtime контракт
- [ADR-009 — Forgeplan as orchestrator](../../.forgeplan/adrs/ADR-009-forgeplan-as-orchestrator-playbook-skill-agent-mapping-pack-marketplace-model.md) — hallucination-proof invariant
- [EVID-088 — Spike-1 c4-to-forge concept validation](../../.forgeplan/evidence/EVID-088-spike-1-c4-to-forge-mapping-concept-validated-on-scoring-module.md) — measurement-based gaidlines (target=note default, table filter, default(value=...))
- [PLAYBOOK-AUTHORING.ru.md](PLAYBOOK-AUTHORING.ru.md) — как ссылаться на mapping из playbook step
- [marketplace/mappings/c4-to-forge.yaml](../../marketplace/mappings/c4-to-forge.yaml) — рабочий образец для копирования
