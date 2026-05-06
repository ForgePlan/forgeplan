# ID Assignment — правила работы с идентификаторами артефактов

**Status**: Draft — введён вместе с PROB-060 / PRD-076 / SPEC-005 / RFC-009 / ADR-012.
**Audience**: Forgeplan contributors (humans) + AI-agents работающие через MCP.
**Language**: Russian (методология).

> ⚠️ **Phase status (cross-phase audit 2026-05-06)**: этот документ описывает **end-state** контракт. Phase 1.x ships только базовая schema (slug + numbers в frontmatter); CI бот, lookup по обоим форматам, multi-agent pre-allocation — Phase 2+. Sections помечены 🟢 (shipped Phase 1.x), 🟡 (Phase 2+), 🔴 (Phase 4 migration).

---

## TL;DR

Forgeplan использует **двухслойную identity**:

- **Slug** (`prd-auth-system`) — каноничный идентификатор. Создаётся локально на `forgeplan new`, никогда не меняется. **Используется в commit refs до merge.**
- **Display number** (`PRD-074`) — отображаемый номер. Присваивается CI-ботом **на merge** в `dev`. Атомарно, без коллизий.
- До merge артефакт виден как **`PRD-74?`** — `?` маркер означает «номер предсказан локально, ещё не финализирован».
- После merge `?` уходит, остаётся **`PRD-074`**. Slug продолжает работать как алиас.

**Главное правило для коммитов**: **до merge — пиши slug в `Refs:`** (`Refs: prd-auth-system`). После merge оба формата работают (`Refs: PRD-074` или `Refs: prd-auth-system`).

---

## Зачем это нужно

См. PROB-060 для полного контекста. Кратко: counter-based ID assignment (`max + 1` при `forgeplan new`) даёт **100% race-window** при параллельной работе на разных ветках или с multi-agent dispatch (PRD-057). Два разработчика/агента независимо получают `PRD-074`, на merge — конфликт + ref rot в commit messages.

Двухслойная модель решает это без нарушения local-first: slug гарантированно уникален между ветками (через slug, не число), а атомарное assignment номера на merge сериализуется через GitHub Actions `concurrency` group.

---

## Контракт двух полей

### Frontmatter артефакта

```yaml
---
slug: prd-auth-system          # canonical, IMMUTABLE после create
predicted_number: 74           # local prediction = max + 1 на момент create
assigned_number: null | 74     # null до merge, число после; write-once by CI bot
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
| `predicted_number` | Только для `?` маркера до merge; не используется в lookups |

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
# Output:
#   Created: .forgeplan/prds/prd-auth-system.md
#   Slug: prd-auth-system
#   Predicted: PRD-74?
#   Hint: Use slug `prd-auth-system` in commit Refs: until merged.
#   Next: forgeplan validate prd-auth-system
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
| prd-auth-system | based_on   ← slug pre-merge ✅
| PRD-074         | based_on   ← number post-merge ✅
```

Обе формы валидны. Резолвер маппит в один canonical артефакт.

### Pre-commit check

`forgeplan validate <slug>` (или pre-commit hook) проверяет:
- Slug соответствует regex
- Slug уникален в origin/dev (если нет — warning с предложением alt-slug)
- `assigned_number` не выставлен вручную (только CI бот)

### Merge → CI бот → assigned_number

При merge feat/* → dev:
1. GitHub Actions workflow `assign-id.yml` с `concurrency: forgeplan-id-assign` — серилизует assignment между всеми параллельными PR
2. CI бот сканирует новые артефакты в PR
3. Находит `max(assigned_number) + 1` per kind в origin/dev
4. Выставляет `assigned_number`, переименовывает файл (`prd-auth-system.md` → `prd-074-auth-system.md`)
5. Делает auto-commit `chore: assign PRD-074`
6. Push back в PR ветку
7. PR можно мержить нормально

### Slug collision (редко)

Если двое независимо назвали `prd-auth`, второй мерж получит auto-suffix:
- Alice мержится первой → `prd-auth` остаётся за ней, получает PRD-074
- Bob мержится второй → CI бот видит коллизию → переименовывает в `prd-auth-2.md` → получает PRD-075
- Bob получает PR comment с уведомлением

Cross-PR refs к Bob's оригинальному `prd-auth` (если кто-то успел сослаться) — `forgeplan reconcile-ids --report-cross-pr` детектит и предлагает фикс.

---

## Workflow для AI-agent (через MCP)

### Создание артефакта

```python
# MCP call
result = forgeplan_new(kind="prd", title="Auth System")
# result:
# {
#   "slug": "prd-auth-system",
#   "predicted_number": 74,
#   "assigned_number": null,
#   "id_canonical": "prd-auth-system",
#   "id_display": "PRD-74?",
#   "_next_action": "forgeplan validate prd-auth-system",
#   "hint": "Use slug 'prd-auth-system' in commit Refs: until merged."
# }
```

**Используй `result.slug` в commit refs.** Не используй `id_display` (содержит `?`) и не используй `predicted_number` (ещё не финален).

### Поиск артефакта

```python
# Оба варианта работают post-merge:
forgeplan_get(id="PRD-074")              # by display number
forgeplan_get(id="prd-auth-system")      # by slug

# Pre-merge — только по slug:
forgeplan_get(id="prd-auth-system")      # ✅
forgeplan_get(id="PRD-74?")              # ❌ вопросительный знак не валиден в lookup
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

Подзадачные агенты **не выбирают slug сами** — получают готовый. Это устраняет slug collision by construction.

### ADI / reasoning prompts

Если в prompt видишь `PRD-074` — это post-merge canonical reference, ищи через `forgeplan_get`.
Если видишь `PRD-74?` — это pre-merge draft, ищи только если у тебя локальный access к workspace.
Если видишь `prd-auth-system` — slug, всегда работает в lookup.

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
forgeplan get prd-auth-system    # ✅ slug — всегда работает
forgeplan get PRD-074            # ✅ post-merge — работает
forgeplan get prd-074            # ✅ case-insensitive — работает
forgeplan search "auth"          # ✅ ищет по slug + title
forgeplan search "PRD-074"       # ✅ ищет по assigned_number

forgeplan list --kind prd        # output:
#   PRD-074  prd-auth-system  active   Auth System
#   PRD-75?  prd-rate-limiter draft   Rate Limiter   ← ? marker для draft
```

### MCP

`forgeplan_get(id=...)` принимает любой формат идентификатора и возвращает canonical артефакт. Поле `slug` в response — единственное гарантированно стабильное.

### Web

URL: `/get/<id>` где `<id>` — slug или number.
Header: `<id_display> — <title>` где `id_display` — derived (`PRD-074` или `PRD-74?`).
Graph nodes: label = `id_display`, для draft — pulse/dashed border.

---

## Migration legacy 298 артефактов

При rollout (Phase 4 по RFC-009):

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

## FAQ

### Q: Почему `?` маркер, а не другой символ?

A: `?` визуально честно сигнализирует «это предсказание, не факт». Альтернативы (`*`, `~`, `#`) либо имеют другие значения в технической нотации, либо менее интуитивны. `?` — стандарт «вопросительности» в большинстве culture.

### Q: Что если я хочу заранее «забить» PRD-100 для будущего?

A: Запрещено. `assigned_number` write-once by CI bot. Reservation API в Out-of-Scope (PRD-076).

### Q: Если slug содержит ошибку (typo) — можно переименовать?

A: Нет (invariant I-1). Если действительно нужно — `supersede` старый артефакт новым (с правильным slug). См. CLAUDE.md `forgeplan supersede`.

### Q: Что если CI бот упал? 🟡 Phase 2+

A: Merge блокируется до починки. Это **намеренная** trade-off — лучше задержать merge на час, чем получить ID коллизию в production. Manual fallback: maintainer запускает `forgeplan ci-assign-id --pr <N>` локально и пушит результат.

> Команда `forgeplan ci-assign-id` **не реализована в Phase 1.x** — Phase 2.4. До Phase 2 этот сценарий неприменим (CI бота ещё нет).

### Q: Multi-agent dispatch с 20+ агентами — будет ли работать?

A: До 10 — точно (тестируется в EVID-B). 20+ — теоретически, но slug pre-allocation в диспетчере становится bottleneck. На этот случай в Growth Vision (PRD-076) предусмотрен redesign.

### Q: Как искать предсказанный артефакт коллеги?

A: Если коллега ещё не запушил — никак (он только локально). Если запушил в feature branch — `git fetch <colleague-branch>` + `forgeplan get <slug>`. После merge — стандартный lookup.

### Q: Что с external forks?

A: Migration script работает на любом workspace без обращения к origin. Fork получает свои `assigned_number` независимо от upstream — потенциальный divergence на merge upstream'а в fork.

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
| 1.0 | 2026-05-06 | Initial — введение двухслойной identity |
