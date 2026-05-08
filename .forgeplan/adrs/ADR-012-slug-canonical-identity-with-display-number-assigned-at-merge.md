---
depth: standard
id: ADR-012
kind: adr
links:
- target: PROB-060
  relation: based_on
- target: RFC-009
  relation: based_on
status: active
title: Slug-canonical identity with display number assigned at merge
---

---
id: ADR-012
title: "Slug-canonical identity with display number assigned at merge"
status: Proposed
depth: deep
valid_until: 2027-05-06
problem_ref: PROB-060
created: 2026-05-06
updated: 2026-05-06
---

# ADR-012: Slug-canonical identity with display number assigned at merge

## Context

PROB-060 зафиксировал: текущая counter-based схема (`forgeplan-core/src/artifact/store.rs:23` — `next_id() = max + 1`) даёт ID-коллизии при параллельной работе на разных ветках и multi-agent dispatch (PRD-057). Pre-commit/pre-merge защиты нет — race-window существует на 100% между ветками/машинами.

Полный FPF-анализ выполнен (см. evaluation в журнале):
- **Option A — Lazy assignment / Rust RFC model** — slug каноничен, номер присваивается CI-ботом на merge. R_eff = 0.665.
- **Option B — ULID-hybrid / Gerrit model** — ULID как primary, derived display number. **Refuted**: display number в реальности не immutable (зависит от merge order), core promise не выдерживается. R_eff = 0.20.
- **Option C — Pure ULID** — drop human-readable numbers. **Weakened**: UX collapse в CLI/Web/Slack, нет precedent для human-collaboration artifact systems с random IDs. R_eff = 0.425.

Trilemma: cannot have all three of {zero-coordination assignment, stable human handle, immutable identity from creation}. Option A выбирает (1)+(2), отказывается от (3) — identity мутирует pre-merge.

Рефинемент Option A — **двухслойная identity** (slug + display number). Slug — canonical, в data plane и refs. Display number — rendering layer для CLI/Web/MCP/Slack, появляется на merge. Pre-merge показывается как `PRD-74?` — predicted_number с маркером `?`.

## Decision

**Selected**: Option A с двухслойной identity (slug-canonical, number-display).

**Why Selected**:
1. Trust Calculus R_eff = 0.665 — самый высокий из всех вариантов
2. Empirical precedent: Rust RFCs (~500+), Kubernetes KEPs (~3000+), npm changesets (~40k+)
3. Сохраняет ментальную модель «PRD-074» через display layer
4. Multi-agent compatible by construction — slugs ортогональны, никакой race-window
5. Forward-only migration — 73 legacy артефактов не трогаются
6. GitHub Actions `concurrency` group обеспечивает атомарное assignment без нарушения local-first

**Двухслойный contract**:
- `slug: prd-auth-system` — canonical identity, immutable, всегда в commit refs и cross-artifact `Related:`
- `predicted_number: 74` — локальное предсказание = `local max(display_number) + 1` на момент create
- `assigned_number: null | 74` — null до merge, выставляется CI-ботом на merge
- Display rule: `id_display = assigned_number ? f"PRD-{assigned:03d}" : f"PRD-{predicted}?"`

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| Counter + remote check (status quo + warning) | Rejected | Не закрывает race-window, только сужает |
| Counter + suffix on collision (PRD-074-a/b) | Rejected | Решает только filesystem-уровень. Refs в commits rot. Семантическая иллюзия родства между несвязанными артефактами |
| **Lazy assignment / Rust RFC model (Option A)** | **Chosen** | R_eff 0.665, empirical precedent, multi-agent friendly, preserves UX через двухслойность |
| ULID-hybrid / Gerrit model (Option B) | Rejected | Display number derived → не immutable. Tooling explosion (139 surfaces). Gerrit precedent имеет central server — local-first analogy ломается |
| Pure ULID (Option C) | Rejected as primary, fallback | UX collapse в CLI/Web/Slack. Нет major precedent для human-collab artifact systems. AI-agent ergonomy degrades |
| Snowflake | Rejected | Требует central machine_id allocation — нарушает local-first. 64-bit меньше уникальности чем UUID при том же UX cost |
| Central ID server (Linear/Phabricator) | Rejected | Нарушает Non-Goal "Local-first, single binary, git for sync" из CLAUDE.md |

## Consequences

### Positive
- Multi-agent dispatch работает с N параллельными агентами без коллизий by construction
- Backward compat: 73 legacy артефакта не трогаются, продолжают работать как раньше
- CI-бот изолирован в merge-hook (~200-300 LOC), не размазывается по 76 CLI commands
- Slug в commit refs стабилен навсегда — нет ref-rot при merge ordering
- Display number в CLI/Web/Slack сохраняется как «PRD-074», ментальная модель не ломается
- Search и MCP принимают оба формата (slug и number) — backward compat для существующих ссылок

### Negative (trade-offs)
- Identity мутирует pre-merge: до merge артефакт `prd-auth-system` (без номера), после — получает `PRD-074`
- Refs в commit messages **до merge** должны использовать slug, не предсказанный номер — это контракт, который команда и агенты должны соблюдать
- Frontmatter усложняется: `slug`, `predicted_number`, `assigned_number` вместо одного `id`
- CI-бот становится критическим: если упал, блокирует merge новых артефактов
- Маркер `?` в `PRD-74?` нужно поддерживать на всех rendering surfaces (CLI, Web, MCP responses)

### Risks
- **R-1 (high impact, low probability)**: GitHub Actions `concurrency: cancel-in-progress: false` не сериализует параллельные merges как документировано ИЛИ multi-agent dispatch deadlock'ится. Mitigation: stress-test 10×concurrent-merge в Phase 0 (EVID-A). Reversal: alternative serialization mechanism (push hooks или maintainer-only assignment role) — не Option C, потому что C проиграл по UX в FPF evaluation.
- **R-2 (high impact, low probability)**: AI-агенты не научатся использовать slug в коммитах до merge — будут писать предсказанный номер, refs rot. Mitigation: явный `hint:` в MCP responses + section в CLAUDE.md + skill update.
- **R-3 (medium impact, medium probability)**: Migration cutoff попадёт на момент когда есть открытые PR со старой схемой. Mitigation: grandfather политика для open PRs at cutoff.

## Invariants

### Phase 1.x current enforcement matrix (cross-phase audit 2026-05-06)

| Invariant | Enforced by code today? | Enforcement mechanism |
|---|:-:|---|
| **I-1** slug immutable after create | ⚠️ partial | `validate_slug` regex enforces format on every create; nothing prevents post-create rename — needs `assert_slug_immutable` helper in projection (Phase 1.5) |
| **I-2** assigned_number write-once | ⚠️ partial | `augment_frontmatter_with_id_fields` preserves explicit `null`; `validate` does NOT yet block PR-diff change of non-null→non-null (Phase 2 CI gate) |
| **I-3** slug refs valid forever | ❌ not yet | requires slug-resolver in `get/search/list/link` — Phase 1.5 |
| **I-4** lookup accepts both formats | ❌ not yet | requires same slug-resolver — Phase 1.5 |
| **I-5** legacy 73 keep their IDs | ✅ trivially | no migration ran yet → legacy untouched. Migration safety verified by EVID-C (Phase 0/4) |
| **I-6** GitHub Actions concurrency serializes | ❌ not yet | `.github/workflows/assign-id.yml` is Phase 2.1 |

Текст ниже описывает **target end-state**; matrix above tracks delivery.

Должны выполняться независимо от реализации:

- **I-1**: `slug` никогда не меняется после create. Любая операция модификации артефакта сохраняет `slug`.
- **I-2**: `assigned_number` — write-once. После присвоения не переписывается (кроме reconciliation на коллизии — должно быть редкое исключение).
- **I-3**: Refs в commit messages, использующие `slug`, остаются валидными forever — slug → канонический lookup по любому состоянию workspace.
- **I-4**: Поиск/get принимает **оба** формата идентификатора (`prd-auth-system` и `PRD-074`) и резолвит в один canonical артефакт.
- **I-5**: Legacy 73 артефакта (PRD-001..073, RFC-001..008, ADR-001..011, EPIC-001..008, и т.д.) сохраняют свои текущие IDs как `assigned_number`; slug генерится из существующего title — никаких изменений в их данных.
- **I-6**: GitHub Actions `concurrency` group `forgeplan-id-assign` сериализует assignment между всеми merge'ами — два PR не получают одинаковый номер ни при каких обстоятельствах.

## Evidence Requirements

Что измерить/доказать для подтверждения решения:

- **EVID-A**: Прототип CI-бота на 50-артефактном fixture + **stress-test 10×concurrent-merge** — 0 race conditions on assigned_number; `concurrency` primitive serializes как документировано; no external state beyond git + GitHub API. **Outcome-based**: meas correctness, не размер.
- **EVID-B**: Multi-agent dispatch benchmark — 10 параллельных запусков `forgeplan_dispatch` с 5 агентами каждый, 0 slug collisions при разных task titles
- **EVID-C**: Migration dry-run — script проходит по всем 298 артефактам и виртуально присваивает им slug + assigned_number, проверяет что нет конфликтов с future numbers
- **EVID-D**: AI-agent compliance — benchmark на 50 reasoning-prompts с MCP, агент использует slug в `Refs:` до merge в ≥ 95% случаев
- **EVID-E**: Web rendering correctness — visual regression suite показывает что pre-merge артефакты отображаются с `?` маркером, post-merge — без

## Valid Until

**Дата**: 2027-05-06 (12 месяцев)

**Обоснование TTL**: ID assignment — фундамент архитектуры. Менять чаще раза в год нецелесообразно (стоимость миграции велика). 12 месяцев — стандартный refresh cycle для архитектурных решений в Forgeplan.

**Refresh Triggers** (когда пере-оценить досрочно):
- Если EVID-A или EVID-B провалятся в ходе реализации (требуется переключение на Option C)
- Если появится major incident с потерей slug uniqueness в production (>3 collisions/month после rollout)
- Если методология поменяется (например, переход на distributed ledger или web-based collaboration model)
- Если число параллельных AI-агентов в `forgeplan_dispatch` превысит 20 (тогда pre-allocation slugs нужно пересмотреть)

## Pre-conditions (чеклист ДО реализации)

- [ ] PRD-076 approved (шёлковая бумага: что именно делаем)
- [ ] SPEC-005 approved (формат frontmatter, regex, derived id)
- [ ] RFC-009 approved (план миграции, фазы, cutoff)
- [ ] EVID-A собран (прототип CI-бота на fixture)
- [ ] EVID-C собран (migration dry-run на 298 артефактах)
- [ ] Документ `docs/methodology/ID-ASSIGNMENT.ru.md` написан и проревьюен
- [ ] CLAUDE.md секция «Working with artifact IDs» добавлена
- [ ] forgeplan skill обновлён с примерами good/bad refs
- [ ] Web team (ForgePlanWeb) уведомлена с концертой задачей по `template/src/widgets/artifact-panel/lib/markdown-export.ts:33` и связанным render points

## Post-conditions (Definition of Done)

- [ ] Все 298 legacy артефактов имеют `slug` и `assigned_number` (миграция выполнена)
- [ ] CI workflow `.github/workflows/assign-id.yml` существует с `concurrency: forgeplan-id-assign`
- [ ] `forgeplan reconcile-ids` команда реализована
- [ ] MCP `forgeplan_new` возвращает `slug`, `predicted_number`, `assigned_number`, `hint`
- [ ] Skills (`forge-cycle`, `forge-audit`, `forgeplan-methodology`) обновлены
- [ ] ForgePlanWeb рендерит `?` маркер для pre-merge артефактов
- [ ] Documentation: ID-ASSIGNMENT.ru.md, CLAUDE.md, GIT-WORKFLOW.ru.md обновлены
- [ ] Smoke test: 5 параллельных AI-агентов создают по 3 артефакта, 0 collisions
- [ ] EVID-D ≥ 95% AI-agent compliance с slug-refs

## Admissibility

Что НЕ допускается в рамках этого решения:

- **NOT**: переименование slug после create (нарушает I-1)
- **NOT**: ручное редактирование `assigned_number` в frontmatter (нарушает I-2; должно быть только через CI-бота)
- **NOT**: использование `predicted_number` в commit refs или cross-artifact `Related:` (только slug или assigned_number)
- **NOT**: создание артефакта с slug, который уже существует в origin/dev — должно блокироваться pre-commit hook
- **NOT**: bypass CI-бота через ручное `git push` с уже выставленным `assigned_number` в frontmatter (CI-бот должен это детектировать и блокировать)
- **NOT**: изменение existing legacy артефактов (PRD-001..073 и т.д.) при миграции — только добавление полей, не модификация id или contents

## Rollback Plan

**Triggers** (когда откатывать — outcome-based):
- EVID-A провалился: stress-test 10×concurrent-merge показывает race conditions ИЛИ `concurrency` primitive не serializes как документировано
- Multi-agent dispatch deadlock'ится при ≥5 параллельных агентах
- В первые 4 недели после rollout: >5 production-incidents с identity coherence (slug collisions, ref rot, duplicate assigned_numbers)
- AI-agent compliance с slug-refs <85% (вместо target ≥95%)

**Steps** (шаги отката):
1. Объявить freeze на создание новых артефактов через MCP/CLI
2. Запустить `forgeplan reconcile-ids --revert-since=<cutoff>` — для всех артефактов созданных после cutoff: переименовать обратно в `<KIND>-NNN-<slug>.md`, выставить `assigned_number` в frontmatter, удалить `predicted_number`
3. Установить feature flag `id_assignment: legacy` в `.forgeplan/config.yaml`
4. Откатить CLI/MCP code до предыдущего release
5. Сообщить агентам и команде о возврате к counter+max+1 модели

**Blast Radius**: revert повлияет на все артефакты созданные после cutoff (ожидаем 5-50 в worst case). Refs в коммитах остаются валидными т.к. slug сохранится как алиас.

## Weakest Link

R_eff = min(F=0.67, G=0.67, R=0.7) × CL2(0.95) = **0.665** (per FPF B.3 — weakest-link aggregation, not averaging).

Слабейшее звено — **R = 0.7**: technique empirically proven (Rust RFC, KEP, changesets), но **не measured в нашей среде с multi-agent + 5 параллельными AI**. EVID-A и EVID-B закроют этот gap.

## Affected Files

| File | Baseline Hash |
|------|---------------|
| crates/forgeplan-core/src/artifact/store.rs | (next_id function — будет deprecated в favor of slug-based) |
| crates/forgeplan-core/src/artifact/frontmatter.rs | (добавить slug, predicted_number, assigned_number поля) |
| crates/forgeplan-core/src/artifact/types.rs | (slug validation, derived id) |
| crates/forgeplan-cli/src/commands/new.rs | (создание с slug + predicted_number) |
| crates/forgeplan-cli/src/commands/reconcile.rs | (новая команда) |
| crates/forgeplan-mcp/src/tools/new.rs | (response с slug + hint) |
| .github/workflows/assign-id.yml | (новый workflow) |
| docs/methodology/ID-ASSIGNMENT.ru.md | (новый документ) |
| CLAUDE.md | (новая секция Working with artifact IDs) |

## AI Guidance

Правила для AI-агентов при работе с этим решением:

- **Создание**: всегда через `forgeplan new <kind> "Title"`. Never manually craft frontmatter с числовым id. Slug генерится автоматически из title.
- **В commit messages до merge**: используй slug в `Refs:` — `Refs: prd-auth-system, FR-001`. Никогда не пиши предсказанный номер (`Refs: PRD-74?` или `Refs: PRD-074` ДО merge — broken).
- **В commit messages после merge**: оба формата работают (`Refs: PRD-074` или `Refs: prd-auth-system`). Предпочитать number для краткости после merge.
- **В cross-artifact `Related:`**: используй slug для pre-merge, либо assigned_number для post-merge.
- **При поиске**: `forgeplan get prd-auth-system` и `forgeplan get PRD-074` оба работают post-merge. Pre-merge только slug работает.
- **При генерации task для другого агента (через `forgeplan_dispatch`)**: диспетчер pre-allocates slug в task — не пытайся сам выбрать slug в подзадаче.
- **Если конфликт**: на pre-commit warning о slug collision — переформулируй title или явно используй `--allow-duplicate` (только если знаешь что делаешь).
- **Не нарушать invariants I-1..I-6** — если задача требует переименования slug или изменения assigned_number вручную, raise это явно с объяснением.

## Implementation Plan

### Phase 0: Foundation (week 1)
- [ ] **0.1** EVID-A: prototype CI-бота на 50-артефактном fixture + stress-test 10×concurrent-merge (atomicity verification)
- [ ] **0.2** EVID-C: migration dry-run на 298 существующих артефактов
- [ ] **0.3** ID-ASSIGNMENT.ru.md написан и проревьюен
- [ ] **0.4** CLAUDE.md обновлён с новой секцией

### Phase 1: Core schema + CLI (week 2-3)
- [ ] **1.1** Frontmatter поля `slug`, `predicted_number`, `assigned_number` в artifact/frontmatter.rs
- [ ] **1.2** Validation: slug regex, uniqueness check vs origin/dev
- [ ] **1.3** `forgeplan new` создаёт артефакт со slug + predicted_number, assigned_number = null
- [ ] **1.4** Derived id resolver — `id_display = assigned ?? predicted+?`
- [ ] **1.5** `forgeplan get/search/list` принимают оба формата
- [ ] **1.6** Unit tests: 100% coverage on new logic

### Phase 2: CI bot + MCP (week 4)
- [ ] **2.1** GitHub Actions workflow `assign-id.yml` с concurrency group
- [ ] **2.2** Atomic next-number assignment + slug suffix on collision
- [ ] **2.3** `forgeplan reconcile-ids` команда
- [ ] **2.4** MCP `forgeplan_new` response с slug + hint + _next_action
- [ ] **2.5** Hint protocol (PRD-071) обновление: `Next:` использует slug pre-merge

### Phase 3: Web + Skills (week 5)
- [ ] **3.1** ForgePlanWeb: derived id rendering в всех виджетах (NodeRef, header, graph)
- [ ] **3.2** ForgePlanWeb: `?` marker styling (dashed border, pulse animation для draft)
- [ ] **3.3** Skills `forge-cycle`, `forge-audit`, `forgeplan-methodology` обновлены
- [ ] **3.4** Documentation: GIT-WORKFLOW.ru.md, UNIFIED-WORKFLOW.ru.md обновлены

### Phase 4: Migration + activation (week 6)
- [ ] **4.1** Migration script: legacy 298 artifacts get slug + assigned_number
- [ ] **4.2** Cutoff date announce, grandfather rules для open PRs
- [ ] **4.3** EVID-B: multi-agent benchmark
- [ ] **4.4** EVID-D: AI-agent compliance benchmark
- [ ] **4.5** Activation gate: все EVID собраны, R_eff > 0.7

## Implementation Log

<!-- Wave entries appended as work progresses -->

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PROB-060 | ProblemCard | based_on |
| PRD-076 | PRD | based_on |
| SPEC-005 | Spec | implements |
| RFC-009 | RFC | based_on |
| PRD-057 | PRD | informs (multi-agent dispatch already in production) |
| PRD-071 | PRD | informs (hint protocol contract) |
| ADR-003 | ADR | informs (markdown source of truth — invariant preserved) |



