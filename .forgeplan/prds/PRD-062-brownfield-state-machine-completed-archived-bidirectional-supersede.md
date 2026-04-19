---
created: 2026-04-19
depth: standard
id: PRD-062
kind: prd
links:
- target: EPIC-006
  relation: refines
- target: ADR-008
  relation: based_on
status: draft
title: Brownfield — state machine completed archived + bidirectional supersede
updated: 2026-04-19
---

# PRD-062: Brownfield — state machine completed archived + bidirectional supersede

## Problem

Forge lifecycle сейчас: `draft → active → {superseded|deprecated|stale}`. Для brownfield-legacy («эту PRD сделали 2 года назад») нет адекватного terminal state — `active` не говорит «done», `superseded` требует by-target, `deprecated` = «don't use» (неверно). Добавить отсутствующие `completed`/`archived` как терминальные. Одновременно — `forgeplan supersede --by` не bidirectional: новый артефакт получает linked но статус старого не меняется на `superseded` автоматически. При brownfield миграции wikilinks типа `[[ADR-005]]` (superseded by ADR-012) должны корректно создать двусторонние отношения.

## Goals

1. Добавить `completed` и `archived` states как terminal post-active.
2. `forgeplan complete <id>` переводит `active → completed` (freeze R_eff, no decay).
3. `forgeplan archive <id>` переводит `completed → archived` (ещё дальше terminal).
4. Bidirectional `supersede`/`deprecate`: atomically обновляют обе стороны, сохраняют links history.
5. Migration path существующих `active` → `completed` руками (опциональная операция).

## Non-Goals

- NOT автоматический promote `active → completed` по критериям (ручной trigger)
- NOT revert `completed → active` без explicit `forgeplan reopen` (уже существует)
- NOT меняет semantics существующих `superseded`/`deprecated`/`stale` — только расширяет

## Target Users

- **Brownfield adopter** — impotrted done-work → sets `completed` не `active`
- **Existing user** — ongoing work stays `active`, completed work gets proper state
- **Auditor** — clearly distinguishes «done but live» (completed) от «done and historical» (archived)

## Success Criteria / Acceptance

- **AC-1**: Состояние `completed` добавлено в enum, validator принимает в status frontmatter.
- **AC-2**: `forgeplan complete ADR-007` → status меняется с `active` на `completed`, R_eff замораживается (не уменьшается по TTL).
- **AC-3**: `forgeplan archive ADR-007` → status меняется с `completed` на `archived`. Terminal.
- **AC-4**: `forgeplan supersede ADR-005 --by ADR-012` atomically: ADR-005 → superseded, ADR-012 получает link `supersedes: ADR-005`. Both projection + DB updated in one transaction.
- **AC-5**: Rollback on failure: если ADR-012 update fails, ADR-005 не остаётся superseded.
- **AC-6**: State transitions matrix documented в `docs/methodology/LIFECYCLE.ru.md`.
- **AC-7**: Backward compat: существующие `active` artifacts без изменений, tests PASS.
- **AC-8**: Brownfield migration (PRD-058 migrate --apply) может ставить `completed`/`archived` напрямую из source `status: done`/`status: archived`.

## Functional Requirements

- **FR-1** Enum `Status::Completed` + `Status::Archived` в forgeplan-core.
- **FR-2** State machine transitions: `active → completed → archived`. `completed` terminal для decay (freeze R_eff). `archived` terminal.
- **FR-3** Commands `forgeplan complete <id>` + `forgeplan archive <id>`.
- **FR-4** Bidirectional supersede: `lifecycle::supersede()` обновляет обе стороны atomically (transactional).
- **FR-5** Bidirectional deprecate: если A deprecated и B имеет `based_on: A` link — warning при deprecate + optional cleanup.
- **FR-6** R_eff freeze для completed/archived: не применяется evidence TTL decay.
- **FR-7** Status-map extension (PRD-058 brownfield migration): `status: done` → `completed`, `status: archived` → `archived`.
- **FR-8** Validator принимает completed/archived как valid statuses.

## Implementation Plan

### Phase 1: Core
- [ ] **1.1** Enum extension + transitions matrix
- [ ] **1.2** `forgeplan complete` / `archive` commands
- [ ] **1.3** R_eff freeze logic

### Phase 2: Bidirectional
- [ ] **2.1** Transactional supersede (both sides atomic)
- [ ] **2.2** Deprecate with symmetric warnings

### Phase 3: Brownfield integration
- [ ] **3.1** status-map extension (done/archived vocabularies)
- [ ] **3.2** Docs update LIFECYCLE.ru.md

### Phase 4: Tests
- [ ] **4.1** State transition tests (all valid paths)
- [ ] **4.2** Atomic rollback test при failure
- [ ] **4.3** Backward compat test

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-008 | ADR | based_on |
| EPIC-006 | Epic | refines |
| PRD-058 | PRD | informs (migration ставит completed/archived напрямую) |



