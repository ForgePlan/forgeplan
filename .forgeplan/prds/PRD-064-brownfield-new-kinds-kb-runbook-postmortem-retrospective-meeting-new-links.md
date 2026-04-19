---
depth: standard
id: PRD-064
kind: prd
links:
- target: EPIC-006
  relation: refines
- target: ADR-008
  relation: based_on
status: draft
title: Brownfield — new kinds kb runbook postmortem retrospective meeting + new links
---

# PRD-064: Brownfield — new kinds kb runbook postmortem retrospective meeting + new links

## Problem

Forge имеет 10 типов артефактов, но brownfield-vaults содержат 5 типов, которые не покрыты: KB-статьи (знания, не решения), runbook'и (операционные процедуры), post-mortems (инцидент-анализ), retrospectives (спринт-ретро), meeting notes. Все сейчас мапятся на `note` (с auto-expire 90d — неподходяще для KB) — теряют семантику и validation-rules. Одновременно отсутствуют link-types для новых отношений: `references` (KB↔artifact bi-dir), `responds_to` (runbook→problem), `caused_by` (postmortem→problem), `discusses` (meeting→any).

## Goals

1. 5 новых kinds: `kb`, `runbook`, `postmortem`, `retrospective`, `meeting`.
2. Per-kind validation rules (MUST sections, recommended depth).
3. 4 новых link types с semantics + graph integration.
4. Использовать существующие LanceDB vector search + petgraph traversal для per-kind use cases (semantic KB search, postmortem similarity, runbook-by-symptom).

## Non-Goals

- NOT заменяет существующие kinds (note остаётся для ephemeral, evidence для measurement)
- NOT добавляет `wiki` / `documentation` / `changelog` kinds — только 5 перечисленных
- NOT меняет link cardinality rules существующих

## Target Users

- **Brownfield adopter** — KB из Obsidian vault становятся первоклассными, не второсортными notes
- **SRE/ops** — runbook + postmortem нативно поддерживаются с proper semantics
- **Scrum team** — retrospective + meeting лёгкие бумаги, без heavyweight PRD workflow

## Success Criteria / Acceptance

- **AC-1**: 5 kinds добавлены в enum, templates созданы (`templates/kb/`, `templates/runbook/`, и т.д.).
- **AC-2**: `forgeplan new kb "<title>"` создаёт KB-артефакт с per-kind template + validation.
- **AC-3**: Per-kind MUST sections: kb (Overview, Details), runbook (Symptom, Diagnosis, Remediation), postmortem (Timeline, Root Cause, Learnings), retrospective (What went well, What didn't, Actions), meeting (Agenda, Notes, Decisions).
- **AC-4**: 4 новых link types registered: `references` (bi-dir), `responds_to`, `caused_by`, `discusses`.
- **AC-5**: `forgeplan search "sybil warmup"` с KB entries возвращает relevant KB через vector search.
- **AC-6**: Graph traversal через `forgeplan graph --from POSTMORTEM-001 --follow caused_by` работает.
- **AC-7**: Backward compat: existing types без изменений.
- **AC-8**: E2E brownfield: 27 KB из Obsidian vault мигрируются как `kind: kb` через PRD-059 migrate.

## Functional Requirements

- **FR-1** Enum `ArtifactKind::{Kb, Runbook, Postmortem, Retrospective, Meeting}` в forgeplan-core.
- **FR-2** Templates per kind: `templates/{kb,runbook,postmortem,retrospective,meeting}/template.md` + `README.md`.
- **FR-3** Per-kind validation rules: MUST sections + recommended depth.
- **FR-4** 4 новых link types в enum `LinkRelation::{References, RespondsTo, CausedBy, Discusses}`.
- **FR-5** Link type semantics: references — bi-dir (both sides get link record), responds_to — runbook → problem (directional), caused_by — postmortem → problem (directional), discusses — meeting → any (directional).
- **FR-6** CLI: `forgeplan new kb|runbook|postmortem|retrospective|meeting <title>` работает как для существующих.
- **FR-7** Vector search + graph extensions: LanceDB embeddings per new kind, petgraph traversal covers new links.
- **FR-8** Brownfield integration: PRD-059 migration может map Obsidian `type: kb` или heuristic «KB-like» content → `kind: kb`.
- **FR-9** Meeting auto-expire 180d default (configurable), KB/runbook/postmortem/retrospective — persistent по-умолчанию.

## Implementation Plan

### Phase 1: Kinds + templates
- [ ] **1.1** Enum + frontmatter schema extension
- [ ] **1.2** 5 templates в `templates/`
- [ ] **1.3** Per-kind validation rules

### Phase 2: Link types + graph
- [ ] **2.1** LinkRelation enum extension
- [ ] **2.2** Bi-dir для references
- [ ] **2.3** Graph traversal extensions

### Phase 3: Vector search integration
- [ ] **3.1** Embedding per new kind
- [ ] **3.2** Semantic search AC verification

### Phase 4: Brownfield integration + tests
- [ ] **4.1** status-map + kind-detection heuristics в PRD-059 migrate
- [ ] **4.2** E2E test: 27 KB fixtures мигрируют как kb
- [ ] **4.3** Docs: `docs/methodology/ARTIFACT-MODEL.ru.md` update

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-008 | ADR | based_on |
| EPIC-006 | Epic | refines |
| PRD-059 | PRD | informs (migration maps new kinds) |
| PRD-063 | PRD | informs (state machine applies to new kinds) |




