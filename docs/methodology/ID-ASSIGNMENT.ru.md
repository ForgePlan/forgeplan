# ID Assignment — правила работы с идентификаторами артефактов

**Status**: Draft — введён вместе с PROB-060 / PRD-076 / SPEC-005 / RFC-009 / ADR-012.
**Audience**: Forgeplan contributors (humans) + AI-agents работающие через MCP.
**Language**: Russian (методология).

> ⚠️ **Phase status (cross-phase audit 2026-05-06)**: этот документ описывает **end-state** контракт. Phase 1.x ships только базовая schema (slug + numbers в frontmatter); CI бот, lookup по обоим форматам, multi-agent pre-allocation — Phase 2+. Sections помечены 🟢 (shipped Phase 1.x), 🟡 (Phase 2+), 🔴 (Phase 4 migration).

> **Уточнение текущего source (2026-10-23, c852f40c):** ниже сохранён
> датированный end-state/Phase 1–2 дизайн, не transcript текущего релиза.
> - `augment_identity_frontmatter` обычно назначает `assigned_number` сразу;
>   явный template `null` сохраняет. Новый артефакт не обязательно имеет `?`.
> - `ci-assign-id` уже существует. Workflow запускается по label `ready-to-merge`
>   или вручную, не по merge; это prototype, его запуск/успех здесь не доказан.
>   CLI пишет frontmatter без rename, `--auto-suffix` пока только предупреждает;
>   `reconcile-ids --report-cross-pr` не выполняет обход cross-PR refs.
> - Одинаковые titles могут дать одинаковые slugs в разных ветках; slug сам
>   по себе не гарантирует глобальную уникальность. «100% race-window» ниже —
>   историческая формулировка риска, не измеренная частота коллизий.
> - Legacy lookup допускает отсутствие slug/assigned_number. На Constellation
>   file-first resolver сопоставляет filename, `id` и `slug`; описание
>   display-id primary key ниже относится к прежнему дизайну индекса.
> - `migrate-legacy-ids` не входит в captured CLI help; не принимайте
>   исторический script/command example за доступную команду.
> - «До 10 — точно» и EVID-B ниже — неподтверждённый здесь исторический claim,
>   не нынешняя гарантия dispatch. Ownership/overlap всё равно проверяются.

---

## TL;DR

Forgeplan использует **двухслойную identity**:

- **Slug** (`prd-auth-system`) — локально создаваемый идентификатор слота. В текущем target creator он обычно сразу получает и display number; слово «canonical» ниже относится к staged design, а не к доказанной глобальной уникальности.
- **Display number** (`PRD-074`) — отображаемый номер. Текущий creator обычно назначает его сразу; nullable `assigned_number` сохраняет поддержку staged CI assignment.
- **`PRD-74?`** — форма для записи с `assigned_number: null`: номер ещё не финализирован.
- Когда номер назначен, используется **`PRD-074`**. Slug остаётся поддерживаемым lookup/alias, но сам по себе не гарантирует уникальность между ветками.

**Главное правило для коммитов**: пока `assigned_number` равен `null`, используй slug в `Refs:` (`Refs: prd-auth-system`). После назначения доступны display ID и slug; всегда сверяй фактический frontmatter.

---

## Зачем это нужно

См. PROB-060 для полного контекста. Историческая формулировка ниже описывает
counter-based allocation (`max + 1`) как **100% race-window** при параллельной
работе на разных ветках или multi-agent dispatch (PRD-057). Это риск
конкурентного выделения номеров, а не измеренная частота: одинаковый title
даёт одинаковый slug, а одинаковый номер возможен при независимом создании
в разных ветках.

Двухслойная модель в staged design разделяет локальный slot и финальный
номер, но сама по себе не доказывает глобальную уникальность slug. Номерные
коллизии требуют явного merge/assignment workflow и проверки фактического
состояния.

---

## Контракт двух полей

### Frontmatter артефакта

```yaml
---
slug: prd-auth-system          # локальный slot; staged design трактует его как immutable
predicted_number: 74           # локальное предсказание номера
assigned_number: null | 74     # current creator обычно пишет 74; null допустим для staged CI
---
```

### Render правило (одной строкой)

```
id_display = assigned_number ? f"PRD-{assigned:03d}" : f"PRD-{predicted}?"
```

| Где используется | Значение |
|---|---|
| `slug` | DB keys, search index, cross-artifact `Related:`, MCP responses field `slug` |
| `id_display` | CLI output, Web header, graph nodes, Slack-friendly format |
| `assigned_number` | Сравнение для backward compat, sort order |
| `predicted_number` | Локальное предсказание номера; используется для `?`-формы, но сам по себе не является lookup-ключом |

### Slug формат (regex)

```
^(prd|rfc|adr|epic|spec|prob|sol|evid|note|ref)-[a-z0-9]+(-[a-z0-9]+)*$
```

- Lowercase + цифры + дефисы
- Длина 3-80 chars (включая prefix)
- Запрещённые: `*-tmp-*`, `*-draft-*`, `*-pending-*`, числовые-only после prefix

Подробности — SPEC-005.

---

## Workflow для разработчика (человек)

### Создание

```bash
git checkout dev && git pull
git checkout -b feat/auth-system
forgeplan new prd "Auth System"
# Иллюстративный staged-null output:
#   Created: .forgeplan/prds/prd-auth-system.md
#   Slug: prd-auth-system
#   Predicted: PRD-74?
#   Next: forgeplan validate prd-auth-system
# Текущий creator обычно сразу назначает assigned_number; сверь реальный output.
```

### Работа и коммиты

В commit messages используй **slug**, не предсказанный номер:

```
✅ ХОРОШО:
feat(auth): add token validation
Refs: prd-auth-system, FR-001..003

❌ ПЛОХО:
feat(auth): add token validation
Refs: PRD-74?, FR-001..003   ← `?`-вариант не должен попадать в коммиты
Refs: PRD-074, FR-001..003   ← номер ещё не assigned, broken pointer

❌ ПЛОХО:
feat(auth): add token validation
Refs: prd-074, FR-001..003   ← lowercase number — не валидный slug, не валидный display
```

### Cross-artifact `Related:` в теле другого артефакта

```yaml
## Related Artifacts
| Artifact | Relation |
|----------|----------|
| prd-auth-system | based_on   ← slot lookup
| PRD-074         | based_on   ← assigned display lookup
```

Обе формы поддерживаются resolver'ом для существующей записи; они не
доказывают глобальную уникальность slot или неизменность номера.

### Pre-commit check

`forgeplan validate <slug>` (или pre-commit hook) проверяет:
- Slug соответствует regex
- Slug уникален в origin/dev (если нет — warning с предложением alt-slug)
- `assigned_number` не выставлен вручную (только CI бот)

### Label-triggered CI prototype → assigned_number

Текущий workflow `assign-id.yml` запускается на PR с label
`ready-to-merge` (или вручную), а не автоматически на merge. Он использует
`concurrency: forgeplan-id-assign`, запускает `forgeplan ci-assign-id` и может
сделать commit/push в PR-ветку. Prototype пишет frontmatter; переименование
файла в нём не выполняется. Успешный запуск и отсутствие коллизий здесь не
доказываются.

Датированный end-state сценарий ниже описывает прежний замысел:

1. Найти кандидатов с `assigned_number: null`.
2. Вычислить `max(assigned_number) + 1` per kind относительно base ref.
3. Выставить `assigned_number` без rename текущего CLI.
4. Сделать auto-commit и вернуть изменение в PR-ветку.

На merge numbering не заменяет assignment автоматически.

### Slug collision

Одинаковые titles могут дать одинаковый slug. Текущий `ci-assign-id`
обнаруживает коллизию, но `--auto-suffix` пока только предупреждает и сам
файл не переименовывает; отдельный maintenance workflow для разрешения
конфликта здесь не установлен. Не обещай suffix или уведомление без
фактического результата.

`forgeplan reconcile-ids --report-cross-pr` сейчас выводит no-op marker:
cross-PR обход не реализован.

---

## Workflow для AI-agent (через MCP)

### Создание артефакта

```python
# MCP call
result = forgeplan_new(kind="prd", title="Auth System")
# result:
# Текущий creator обычно возвращает assigned_number сразу; staged-пример
# с null приведён ниже только для различения форм.
# {
#   "slug": "prd-auth-system",
#   "predicted_number": 74,
#   "assigned_number": null,
#   "id_canonical": "prd-auth-system",
#   "id_display": "PRD-74?",
#   "_next_action": "forgeplan validate prd-auth-system"
# }
```

**Используй фактический `assigned_number`/возвращённый ID.** Пока номер
не назначен, `id_display` с `?` не является lookup-командой, а
`predicted_number` не доказывает финальность номера.

### Поиск артефакта

```python
# Для существующей assigned записи:
forgeplan_get(id="PRD-074")              # assigned display number
forgeplan_get(id="prd-auth-system")      # slot lookup

# Пока assigned_number не назначен, используй slug, а не display с '?':
forgeplan_get(id="prd-auth-system")
forgeplan_get(id="PRD-74?")              # '?' не является lookup-командой
```

### Multi-agent dispatch (`forgeplan_dispatch`) 🟡 Phase 2+

> **Status:** диспетчер пока **не** делает pre-allocation slugs. Phase 1.x: параллельные AI-агенты могут получить одинаковые slugs если переданы похожие task titles. Workaround: задавай unique titles per task. Phase 2.5 ship'нет automatic pre-allocation.

Когда ты — диспетчер раздающий задачи параллельным агентам (target end-state):

```python
# Диспетчер сам pre-allocates уникальные slugs для задач
plan = [
  {"task": "Auth", "slug": "prd-auth-system"},      # pre-allocated
  {"task": "Rate", "slug": "prd-rate-limiter"},     # pre-allocated
  {"task": "Cache", "slug": "prd-caching-layer"},   # pre-allocated
]
```

Подзадачные агенты в staged end-state получают готовый slot. Это устраняет
случайные title-derived collisions внутри одного плана, но не доказывает
глобальную уникальность и не является текущим гарантированным dispatch API.

### ADI / reasoning prompts

Если в prompt видишь `PRD-074` — это assigned display reference; ищи через
`forgeplan_get` и сверь фактический frontmatter.
Если видишь `PRD-74?` — запись с нефинализированным номером; ищи только
по доступному slug, если у тебя есть локальный доступ к workspace.
Если видишь `prd-auth-system` — используй slug lookup, но помни, что
уникальность slot не гарантирована глобально.

### Запрещённые операции

AI-agent **не должен**:
- Самостоятельно выставлять `assigned_number` в frontmatter (write-once by CI bot)
- Переименовывать slug после create (immutable)
- Использовать `predicted_number` в commit refs или cross-artifact `Related:`
- Создавать артефакт обходя `forgeplan new` (вручную писать .md файл с frontmatter)
- Bypass'ить pre-commit warning о slug collision без явной причины

---

## Поиск, lookup, и оба формата

### CLI

```bash
forgeplan get prd-auth-system    # slug lookup, если запись существует
forgeplan get PRD-074            # assigned display lookup
forgeplan search "auth"          # keyword/smart search по доступному workspace
forgeplan search "PRD-074"       # ищет по доступным идентификаторам

forgeplan list --type prd        # иллюстрация end-state-контракта, не текущий transcript:
#   PRD-074  prd-auth-system  active   Auth System
#   PRD-75?  prd-rate-limiter draft   Rate Limiter   ← ? marker для draft
```

Не используй этот блок как доказательство поддержки lowercase numeric
lookup или глобальной уникальности slug; эти свойства здесь не проверялись.

### MCP

`forgeplan_get(id=...)` принимает поддерживаемые текущим resolver slug и
display ID и возвращает найденный артефакт. Для legacy `slug` может быть
`null`/omitted; не называй его единственным гарантированно стабильным полем.

### Web

URL: `/get/<id>` где `<id>` — slug или number.
Header: `<id_display> — <title>` где `id_display` — derived (`PRD-074` или `PRD-74?`).
Graph nodes: label = `id_display`, для draft — pulse/dashed border.

---

## Migration legacy 298 артефактов

> **Update 2026-05-08 (Phase 2.3 audit)**: миграция **demoted с MUST до OPTIONAL
> CLEANUP**. Causa: PR #268 dev-sync CI revealed что 8 legacy PROB-060 artifacts
> (а также все остальные 73 legacy без slug) **работают first-class** через
> resolver fallback paths, MCP DTO `Option<String>` handling, и
> `refs_form_from_body` graceful degradation. Migration теперь — cosmetic
> housekeeping, не required для функциональности.
>
> Если кто-то всё-таки запускает migration script (см. ниже) — это additive
> cleanup, не gating activity. Existing artifacts работают **БЕЗ изменений**.

См. также §«Legacy artifacts compatibility» ниже — там зафиксированы exact
code paths, обеспечивающие backward compat.

### Cosmetic migration (если запущена) — Phase 4 по RFC-009

Для каждого существующего артефакта (PRD-001..PRD-073, RFC-001..RFC-008, ADR-001..ADR-011, и т.д.):

1. Frontmatter получает **дополнительные** поля (existing fields не трогаются):
   - `slug: <generated from existing title>`
   - `predicted_number: <existing number>`
   - `assigned_number: <existing number>`
2. Filename **не переименовывается** (остаётся как `PRD-018-rfc-driven-architecture.md`)
3. Existing refs `Refs: PRD-018` продолжают работать через assigned_number lookup
4. Slug добавляется как алиас (`Refs: prd-rfc-driven-architecture` теперь тоже работает)

Если две legacy артефакта генерируют одинаковый slug (редко) — second получает `<slug>-<number>` суффикс.

**Контракт**: миграция additive-only. Никаких изменений contents, никаких rewrite refs, никаких deletes.

---

## Legacy artifacts compatibility

**TL;DR**: legacy артефакты, созданные до Phase 1.5 (всё, что было в репозитории
к моменту landing'а PROB-060), **не имеют** `slug` поля во frontmatter и **не
требуют** миграции. Они работают как first-class citizens через fallback paths
в resolver, MCP DTOs и hint emission.

### Принципиальное решение

Изначально RFC-009 §Phase 4 предполагал **обязательную** миграцию всех 298+
legacy артефактов (добавление `slug`, `predicted_number`, `assigned_number` во
frontmatter). PR #268 dev-sync CI прогон показал, что 8 PROB-060-related legacy
артефактов работают корректно без миграции, а user feedback («обновлять столько
артефактов это плохо») зафиксировал нежелание делать invasive cleanup пасс по
существующим файлам.

Решение: миграция становится **optional cosmetic cleanup**, не required для
функциональности. Все backward compat paths уже встроены в Phase 1/Phase 2 код.

### Как именно работает fallback

#### 1. Resolver — `crates/forgeplan-core/src/artifact/store.rs`

При вызове `forgeplan get <id>` (или MCP `forgeplan_get`) текущий
file-first resolver сопоставляет filename, `id` и доступный slug:
- slug (`prd-auth-system`) и display ID (`PRD-074`) поддерживаются для
  записей, у которых есть соответствующие поля/имя файла;
- legacy записи без slug остаются first-class: E2E `legacy_compat_e2e.rs`
  проверяет display-ID lookup и omitted `slug` в JSON;
- прежнее утверждение, что `assigned_number` «всегда set» у всех legacy,
  здесь не воспроизводится проверкой.

Иными словами: slug — optional lookup alias; не назначай его единственным
стабильным идентификатором и не описывай прежний display-ID primary key как
текущий универсальный индексный контракт.

#### 2. MCP DTOs — `crates/forgeplan-mcp/src/types.rs`

```rust
pub struct ArtifactDto {
    pub id: String,
    pub kind: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,    // legacy artifact → None → field omitted in JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predicted_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_number: Option<u32>,
    // ...
}
```

Для legacy артефакта `slug = None` — поле не сериализуется в JSON. Agent видит
`id` и работает с ним; отсутствие slug не является ошибкой, но не даёт
дополнительного alias для lookup.

#### 3. `refs_form_from_body` — fallback к display id

`refs_form_from_body` разбирает **frontmatter артефакта**, не commit `Refs:`.
Она вызывает `refs_form`: при отсутствии assigned_number возвращает валидный
slug, если он есть; иначе переданный fallback ID. Ошибка разбора тоже даёт
fallback. Поиск commit refs — отдельная задача, не гарантия этого helper.

#### 4. E2E coverage

`crates/forgeplan-cli/tests/legacy_compat_e2e.rs` (создан Fixer 2.3-A) фиксирует
все три fallback paths real CLI invocation:
- Resolver: `forgeplan get PRD-018` для legacy artifact без slug
- MCP serialization: `forgeplan_get` JSON output не содержит `slug` для legacy
- Refs parsing: commit body `Refs: PRD-018` находит правильный артефакт

### Migration script — статус

Migration script (`scripts/migrate-legacy-ids.sh` / `forgeplan migrate-legacy-ids`):
- **Cosmetic only**: добавляет `slug`, `predicted_number`, `assigned_number` во
  frontmatter existing artifacts.
- **Не required**: системы работают без миграции, см. fallback paths.
- **Additive**: никаких изменений contents, никаких rewrite refs, никаких deletes.
- **Reversible**: rollback — strip added fields из frontmatter (legacy state).

Запускать только если есть конкретная причина (например, желание иметь slug-based
links в новых артефактах ссылающихся на legacy). Default посыл — **don't touch
working legacy**.

### Что НЕ делать

- **Не редактировать legacy frontmatter вручную** через `Edit`/`Write` (red-line #11).
  MCP `forgeplan_update.body` меняет prose body, не является setter для внешних
  identity fields. Добавление slug требует поддержанного identity/migration
  workflow, не вставки frontmatter в body ради обхода API.
- **Не считать missing slug ошибкой** в reasoning prompts — это expected state для
  legacy.
- **Не добавлять slug в legacy при routine operations** (например, при `forgeplan
  link`) — это инвазивная мутация без причины.

### Acceptance criteria для PROB-060 closure

Старая формулировка (RFC-009 pre-Phase 2.3 audit):
> All 73 legacy artifacts have slug + assigned_number in frontmatter

Новая формулировка (Phase 2.3 audit):
> All 73 legacy artifacts continue to work via display id path, verified by E2E
> suite `legacy_compat_e2e.rs`. No migration required.

---

## FAQ

### Q: Почему `?` маркер, а не другой символ?

A: `?` визуально честно сигнализирует «это предсказание, не факт». Альтернативы (`*`, `~`, `#`) либо имеют другие значения в технической нотации, либо менее интуитивны. `?` — стандарт «вопросительности» в большинстве culture.

### Q: Что если я хочу заранее «забить» PRD-100 для будущего?

A: Запрещено. `assigned_number` write-once by CI bot. Reservation API в Out-of-Scope (PRD-076).

### Q: Если slug содержит ошибку (typo) — можно переименовать?

A: Нет (invariant I-1). Если действительно нужно — `supersede` старый артефакт новым (с правильным slug). См. CLAUDE.md `forgeplan supersede`.

### Q: Что если CI prototype упал?

A: Сначала прочитай failed workflow/logs. `forgeplan ci-assign-id` можно
запустить вручную с `--dry-run`, а затем в write-режиме; workflow сейчас
prototype, и его merge-blocking/collision guarantees здесь не доказаны.
Историческая Phase 1.x заметка ниже сохранена как датированный дизайн.

> Исторический Phase 1.x дизайн: команда `forgeplan ci-assign-id` ещё не
> была реализована, поэтому сценарий считался неприменимым до Phase 2.

### Q: Multi-agent dispatch с 20+ агентами — будет ли работать?

A: Текущий core ограничивает `--agents` диапазоном 1–64; это hard bound,
не гарантия отсутствия конфликтов. Он не доказывает работоспособность
конкретного dispatch в 10 или 20 агентов. Историческое «До 10 — точно»
и ссылка на EVID-B здесь не подтверждены и не должны использоваться как
гарантия.

### Q: Как искать предсказанный артефакт коллеги?

A: Если коллега ещё не запушил — никак (он только локально). Если запушил в feature branch — `git fetch <colleague-branch>` + `forgeplan get <slug>`. После merge — стандартный lookup.

### Q: Что с external forks?

A: Исторический migration script был описан как локальный и не требующий
origin. В captured CLI help сейчас нет `migrate-legacy-ids`; доступность
отдельного script проверяй в своём checkout. Fork может иметь собственные
numbers/slugs, поэтому после merge проверяй фактический frontmatter и
resolver, а не полагайся на этот исторический ответ.

### Q: У меня legacy артефакт без slug — это сломанный?

A: **Нет, это first-class citizen.** Phase 2.3 audit (2026-05-08) зафиксировал что
все legacy артефакты работают через display id path без миграции. См. §«Legacy
artifacts compatibility». Resolver, MCP DTOs и refs parser graceful обрабатывают
missing slug через fallback к display number. Migration → optional cosmetic
cleanup, не required.

### Q: Должен ли я добавить slug в legacy artifact при `forgeplan link` или
`forgeplan_update`?

A: Нет. Routine operations не должны мутировать legacy frontmatter. Если хочется
slug — запусти dedicated migration script. Без явной причины — оставь legacy в
текущем состоянии (don't touch working code).

### Q: Migration script когда?

A: Опционально, в любой момент после Phase 2 GA. Не блокирует никакие фичи. Если
команда решит что хочется единый schema (slug у всех артефактов) — запускается
как отдельный housekeeping PR. Не gating activity.

---

## Связанные документы

- **PROB-060** — обоснование проблемы
- **PRD-076** — продуктовые требования
- **SPEC-005** — точный технический контракт (regex, API)
- **RFC-009** — план миграции (5 фаз × 6 недель)
- **ADR-012** — фиксация решения и FPF F-G-R обоснование
- **CLAUDE.md** — секция «Working with artifact IDs»
- **PRD-057** — multi-agent dispatch (consumer этого контракта)
- **PRD-071** — hint protocol (используется в MCP responses)
- **ADR-003** — markdown source of truth (invariant сохранён)

## История изменений

| Версия | Дата | Изменения |
|--------|------|-----------|
| 1.1 | 2026-05-08 | Phase 2.3 audit: legacy compatibility section, migration demoted to optional cosmetic cleanup, FAQ updated |
| 1.0 | 2026-05-06 | Initial — введение двухслойной identity |
