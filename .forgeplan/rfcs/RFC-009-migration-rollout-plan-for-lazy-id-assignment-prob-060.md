---
depth: standard
id: RFC-009
kind: rfc
links:
- target: PRD-076
  relation: based_on
status: draft
title: Migration rollout plan for lazy ID assignment (PROB-060)
---

---
id: RFC-009
title: "Migration rollout plan for lazy ID assignment (PROB-060)"
status: Draft
author: explosivebit
created: 2026-05-06
updated: 2026-05-06
prd: PRD-076
depth: deep
---

# RFC-009: Migration rollout plan for lazy ID assignment (PROB-060)

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/4   (  0%)  Foundation (EVID, docs, fixture)
Phase 1  ░░░░░░░░░░░░░░░░░░░░░░░░  0/6   (  0%)  Core schema + CLI
Phase 2  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5   (  0%)  CI bot + MCP
Phase 3  ░░░░░░░░░░░░░░░░░░░░░░░░  0/4   (  0%)  ForgePlanWeb + Skills
Phase 4  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5   (  0%)  Migration + activation
─────────────────────────────────────────────────
TOTAL                               0/24  (  0%)
```

---

## Summary

Phased rollout (5 фаз × ~6 недель) перехода с counter-based ID assignment на slug-canonical + lazy display number, с backward compat для 73 legacy артефактов и feature flag для отката.

## Motivation

PROB-060: ID-коллизии при параллельной работе на ветках и multi-agent dispatch. Без решения проблема нарастает при росте команды и числа AI-агентов в `forgeplan_dispatch`.

ADR-012 фиксирует решение (Option A с двухслойной identity, R_eff = 0.665). RFC-009 — конкретный план **как это построить**, фазами, с возможностью откатиться.

Если **не делать**: продолжаем накапливать технический долг — каждый новый параллельный workflow усугубляет проблему. Multi-agent dispatch (PRD-057) уже в production и активно деградирует.

## Goals

1. Доставить решение в 5 фаз без single big-bang migration
2. На каждой фазе иметь работающий feature flag для отката (`id_assignment: legacy / hybrid / new`)
3. Backward compat: все 298 существующих артефактов и `Refs:` к ним продолжают работать
4. Multi-agent dispatch получает zero-collision гарантии by Phase 2
5. ForgePlanWeb рендеринг обновлён до Phase 3 (не блокирует Phases 1-2)
6. Activation gate: все EVID собраны, R_eff > 0.7

## Non-Goals

- Изменение existing legacy IDs (PRD-001..073 etc.)
- Cross-workspace ID coordination
- Реализация Option B (ULID-hybrid) или Option C (pure ULID) в основной ветке (только как rollback fallback)
- Распределённый consensus protocol
- Reservation API («забить номер заранее»)

## Options Considered

(полный FPF F-G-R анализ — см. ADR-012 и evidence в PROB-060 preview table)

### Option A: Phased rollout, 5 phases, 6 weeks (this RFC)

**Description**: Foundation → Core CLI → CI bot + MCP → Web + Skills → Migration. Каждая фаза имеет feature flag, можно откатиться.

**Pros**:
- Минимальный risk: каждая фаза изолирована
- EVID собирается на ходу (EVID-A в Phase 0, EVID-B в Phase 4)
- Web team работает параллельно с core (Phase 3 не блокирует Phase 2)
- Возможность остановиться после Phase 2 если что-то пошло не так — Phases 3-4 cosmetic & migration

**Cons**:
- 6 недель — длинный horizon
- Координация между Phase 1 (core) и Phase 3 (Web) требует sync
- Feature flag добавляет временный complexity

### Option B: Big-bang single release

**Description**: Сделать всё в одной крупной ветке, релизить как breaking change в v0.32.0.

**Pros**: Проще архитектурно — нет промежуточных feature flags

**Cons**:
- Высокий risk: если что-то сломается — затронет всё сразу
- Невозможно собрать EVID на промежуточных стадиях
- Web team вынужден синхронизироваться с core release date
- Откат — только полный revert release

### Option C: Сразу pure ULID (Option C из ADR-012)

**Description**: Пропустить slug-canonical модель, сразу на ULID.

**Pros**: Простейший tooling change

**Cons**: UX collapse, лишает «PRD-074» ментальной модели — отвергнуто на уровне ADR-012. Этот RFC не открывает решение заново.

## Trade-off Analysis

| Критерий | Option A (phased) | Option B (big-bang) | Option C (pure ULID) |
|----------|----------|----------|----------|
| Complexity (during rollout) | Medium (feature flags) | High (single huge PR) | Low |
| Cost (engineering weeks) | 6 weeks distributed | 4-5 weeks concentrated | 2 weeks |
| Scalability | High | High | High |
| Migration risk | Low (per phase rollback) | High (all-or-nothing) | High (UX impact) |
| Developer experience | Low disruption | Medium disruption | Permanent UX degradation |
| Operational burden | Medium (CI bot) | Medium | Low |
| Reversibility | High (per phase) | Low (full revert) | Low (legacy IDs lost) |
| Web team coordination | Independent (Phase 3) | Tightly coupled | Tightly coupled |

## Proposed Direction

**Option A (phased rollout)** — решение из ADR-012 ставит lazy assignment как primary, и phased delivery — лучший способ его завезти с минимизацией risk. Option C остаётся как fallback на Phase 0/1 если EVID-A провалится.

## Risks & Open Questions

### Risks
- **R-1**: GitHub Actions `concurrency: cancel-in-progress: false` не сериализует параллельные merges как документировано ИЛИ multi-agent dispatch deadlock'ится при ≥5 параллельных агентах. **Mitigation**: stress-test 10×concurrent-merge в Phase 0 (EVID-A). **Reversal**: alternative serialization mechanism (push hooks или maintainer-only assignment role).
- **R-2**: ForgePlanWeb team задержит Phase 3. **Mitigation**: Phase 3 отделён от Phase 2 — core может GA без Web; Web догоняет в течение 2 недель.
- **R-3**: Legacy migration найдёт duplicate slugs (два PRD с одинаковым title). **Mitigation**: dry-run в Phase 0 (EVID-C) выявит проблему до Phase 4.
- **R-4**: Open PRs на момент cutoff содержат старую схему. **Mitigation**: grandfather rules + cutoff на момент когда open PRs минимизированы.
- **R-5**: AI-agents через MCP не сразу адаптируются. **Mitigation**: Phase 2 включает explicit `hint:` в responses + Phase 3 обновляет skills.

### Open Questions
- **OQ-1**: Какой именно cutoff date выбрать? — Решается в Phase 4 prep, по состоянию open PRs.
- **OQ-2**: Как именно обрабатывать legacy slug collisions при migration? — Hard-coded suffix `-<existing-number>` или manual review? Решается на основании EVID-C dry-run результатов.
- **OQ-3**: Нужен ли отдельный pre-commit hook в форгеплан или достаточно `forgeplan validate --strict`? — Решается в Phase 1 design review.
- **OQ-4**: Будут ли проблемы у внешних форков (например, у пользователей с приватными forgeplan repos)? — Migration script должен работать на любом workspace без доступа к origin.
- **OQ-5**: Должна ли ForgePlanWeb использовать predicted_number в URL для draft артефактов, или slug? — Phase 3 design decision (предложение: slug в URL, derived display id в headers).

## Implementation Phases

### Phase 0: Foundation (week 1)

Закрыть evidence gaps до коммита на полный путь.

**Status (cross-phase audit 2026-05-06):** Phase 0 was split mid-flight — 0.3 delivered with shape commit (`d375958`), но 0.1/0.2/0.4 не сделаны до старта Phase 1. Это **методологический breach** (CLAUDE.md red-line #7 — нет evidence до code), исправляется через Phase 0b в конце Phase 1.

- [x] **0.3** Документ `docs/methodology/ID-ASSIGNMENT.ru.md` написан и проревьюен — single source of truth для людей и AI-agents. **Done in commit d375958.**
- [ ] **0.1** EVID-A: prototype CI-бота на 50-артефактном fixture + stress-test 10×concurrent-merge. Verify atomicity. Reverse condition check (race conditions detected → switch to alternative serialization mechanism). **Pending — required before Phase 2 starts.**
- [ ] **0.2** EVID-C: migration dry-run на 298 существующих артефактов. Detect potential slug collisions in legacy. **Pending — required before Phase 4 starts.**
- [ ] **0.4** CLAUDE.md обновлён с секцией "Working with artifact IDs": правила refs, slug в коммитах до merge, оба формата после. **Pending.**

**Exit criteria**: EVID-A показывает 0 race conditions on 10×concurrent-merge stress-test; EVID-C показывает 0 unresolved legacy collisions; documents в PR review.

### Phase 1: Core schema + CLI (week 2-3)

Backward-compatible изменение core: новые поля в frontmatter, derived id, оба формата идентификатора в lookups.

- [ ] **1.1** Frontmatter поля `slug`, `predicted_number`, `assigned_number` в `crates/forgeplan-core/src/artifact/frontmatter.rs`. Backward compat: legacy frontmatter без этих полей продолжает работать (slug derived on-the-fly из filename).
- [ ] **1.2** Slug validation в `crates/forgeplan-core/src/artifact/types.rs` (regex, reserved prefixes, length).
- [ ] **1.3** `forgeplan new <kind>` создаёт артефакт со slug + predicted_number, assigned_number = null. Pre-create check vs `git fetch origin/dev` (warn).
- [ ] **1.4** Derived id resolver — `id_display = assigned ?? predicted+"?"`. Используется во всех CLI command outputs.
- [ ] **1.5** `forgeplan get/search/list` принимают оба формата идентификатора. Internal lookup всегда по slug.
- [ ] **1.6** Unit tests: 100% coverage on new logic. Property tests on slug generation invariants.

**Exit criteria**: cargo test green; CLI smoke test passing; feature flag `id_assignment: hybrid` (legacy + new в одном workspace) работает.

### Phase 2: CI bot + MCP (week 4)

CI-бот для атомарного assignment + MCP responses обновлены.

- [ ] **2.1** GitHub Actions workflow `.github/workflows/assign-id.yml` с `concurrency: forgeplan-id-assign, cancel-in-progress: false`.
- [ ] **2.2** `forgeplan ci-assign-id --pr <N>` команда: scans new artifacts in PR, finds next free number per kind, sets `assigned_number`, renames files, rewrites same-PR refs (slug → number alias).
- [ ] **2.3** Slug collision detection + auto-suffix mechanism. PR comment notification.
- [ ] **2.4** `forgeplan reconcile-ids` команда — manual cleanup для post-merge issues.
- [ ] **2.5** MCP `forgeplan_new` returns slug + predicted_number + assigned_number + hint + _next_action (per PRD-071 hint protocol).

**Exit criteria**: stress test 5 simultaneous PRs serialize correctly; EVID-D start collecting (AI-agent compliance).

### Phase 3: ForgePlanWeb + Skills (week 5)

Rendering layer и agent training.

- [ ] **3.1** ForgePlanWeb: derived id rendering в `template/src/widgets/artifact-panel/lib/markdown-export.ts:33` и связанных NodeRef widgets. URL `/api/get/[id]` принимает оба формата.
- [ ] **3.2** ForgePlanWeb: `?` marker styling (dashed border, pulse animation для draft graph nodes).
- [ ] **3.3** Skills `forge-cycle`, `forge-audit`, `forgeplan-methodology` обновлены с примерами good/bad refs (slug-based pre-merge, number-based post-merge).
- [ ] **3.4** Documentation: `docs/operations/GIT-WORKFLOW.ru.md`, `docs/methodology/UNIFIED-WORKFLOW.ru.md`, ADR-003 cross-references обновлены.

**Exit criteria**: visual regression suite ForgePlanWeb green; skills updated and tested via sample agent runs.

### Phase 4: Migration + activation (week 6)

Cutoff date, migration legacy 298 артефактов, EVID closure, activation.

- [ ] **4.1** Cutoff date announce в CHANGELOG. Open PRs grandfather rules: PRs открытые до cutoff merge'атся по старой схеме; new PRs после cutoff — по новой.
- [ ] **4.2** Migration script: legacy 298 artifacts get `slug` + `assigned_number` фrontmatter поля (additive only — никаких contents changes). Run on dev, validate, then push.
- [ ] **4.3** EVID-B: multi-agent benchmark — 10 запусков `forgeplan_dispatch` с 5 агентами. Target: 0 slug collisions при разных titles.
- [ ] **4.4** EVID-D: AI-agent compliance benchmark — 50 reasoning prompts, измерить % коммитов с slug в `Refs:` (target ≥ 95%).
- [ ] **4.5** Activation gate: все EVID собраны (A, B, C, D, E), R_eff > 0.7. ADR-012 переключён в `active`. Feature flag `id_assignment` дефолт меняется на `new`.

**Exit criteria**: smoke test 5 параллельных AI-агентов создают по 3 артефакта, 0 collisions; visual regression Web pass; feature flag `legacy` остаётся доступным как rollback option до v0.34.

---

## Affected Files

### Core (forgeplan repo)
- `crates/forgeplan-core/src/artifact/frontmatter.rs`
- `crates/forgeplan-core/src/artifact/store.rs` (next_id deprecated)
- `crates/forgeplan-core/src/artifact/types.rs`
- `crates/forgeplan-cli/src/commands/new.rs`
- `crates/forgeplan-cli/src/commands/{get,search,list,reconcile}.rs`
- `crates/forgeplan-mcp/src/tools/*.rs`
- `crates/forgeplan-core/src/playbook/dispatch/forgeplan_core_dispatcher.rs` (slug pre-allocation)

### CI / Infra
- `.github/workflows/assign-id.yml` (new)
- `.github/workflows/ci.yml` (add validation gate)

### Documentation
- `docs/methodology/ID-ASSIGNMENT.ru.md` (new)
- `CLAUDE.md` (new section)
- `docs/operations/GIT-WORKFLOW.ru.md`
- `docs/methodology/UNIFIED-WORKFLOW.ru.md`
- `CHANGELOG.md` (cutoff announce)

### ForgePlanWeb (separate repo)
- `template/src/widgets/artifact-panel/lib/markdown-export.ts`
- `template/src/routes/api/get/[id]/+server.ts`
- `template/src/widgets/insights-rail/ui/InsightsRail.svelte`
- `template/src/widgets/dependency-graph/ui/{ForceView,SunburstView}.svelte`
- `template/src/entities/activity/model/types.ts`

### Skills
- `~/.claude/plugins/marketplaces/ForgePlan-marketplace/plugins/forgeplan-workflow/`
- `~/.claude/plugins/marketplaces/ForgePlan-marketplace/plugins/forge-cycle/`

---

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PROB-060 | ProblemCard | based_on |
| PRD-076 | PRD | based_on |
| SPEC-005 | Spec | implements |
| ADR-012 | ADR | decided_by |
| PRD-057 | PRD | informs (multi-agent context) |
| PRD-071 | PRD | informs (hint protocol) |
| ADR-003 | ADR | informs (markdown source of truth) |

---

> **Next step**: После approve RFC → ADR-012 переключить в Proposed → Active с EVID; запустить Phase 0.


