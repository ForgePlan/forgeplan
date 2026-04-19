---
created: 2026-04-19
depth: tactical
id: PROB-040
kind: problem
links:
- target: ADR-008
  relation: informs
- target: EPIC-006
  relation: informs
- target: PROB-022
  relation: informs
status: draft
title: Brownfield Shape audit findings 2026-04-19 — 6 CRITICAL + 12 HIGH for next iteration
updated: 2026-04-19
---

# PROB-040: Brownfield Shape audit findings — next-iteration backlog

## Problem Statement

4-agent adversarial audit (architect-reviewer, ddd-domain-expert, code-analyzer, production-validator) для Shape-phase артефактов ADR-008 + EPIC-006 + PRD-059..064 + EVID-079 на ветке `feat/prd-059-brownfield-pipeline` (2026-04-19) выявил существенные architectural + methodology gaps, которые не удалось закрыть в первой shape-итерации и требуют отдельной работы до Code-phase и до активации PRDs.

Production-validator дал 0 Red Line violations (shape methodology clean) — это значит commits валидны. Но content-quality findings от трёх других agents критичны: их игнорирование ведёт к implementation с broken invariants (Status enum drift), hostile UX (scan-import removal), и silent data loss (atomic supersede без механизма).

## Signal

Reference: audit completed 2026-04-19 на commits bc811dd..e3f0382. 4 agent outputs сохранены в session log. First iteration fixes (Phase A/B) закрыли tractable findings (dead refs PRD-A..F, typos, frontmatter drift, _hints rename, scan-import removal softening v0.27→v1.0, meeting expiry config-driven, double-supersede PROB-022, EVID-079 linked к PRD-059..064, heuristic kind_hint rename, scope boundary PRD-059 vs PRD-062, numeric Goal budgets EPIC-006) — остались 6 CRITICAL + 12 HIGH + 15 MEDIUM + 8 LOW untractable через batch edit.

## Root Cause

Shape-iteration 1 создавалась без предварительной проверки runtime баз (Status enum в `crates/forgeplan-core/src/artifact/types.rs`, LanceDB transaction semantics, существующий `LinkType` enum), что привело к assumptions не совпадающим с кодом. Плюс depth=critical claim в ADR-008 body без соответствующих Spec/RFC артефактов per CLAUDE.md routing matrix.

## Findings — Next-iteration backlog

### CRITICAL (6) — blockers для activate + Code-phase

**C1 — Status enum reality drift** (architect). PRD-063 assumes `Stale` variant + `Draft→Active→{Superseded|Deprecated|Stale}` machine. Реальный enum в `crates/forgeplan-core/src/artifact/types.rs:131`: `Draft|Active|Superseded|Deprecated|RefreshDue` (нет `Stale`). CLAUDE.md + ADR docs ссылаются на `stale` как если бы он существовал. Pre-work: reconcile — rename `RefreshDue → Stale` или добавить alias. Blocks PRD-063.

**C2 — Skill files outside `.forgeplan/` violates ADR-003 spirit** (architect). PRD-062 `forgeplan-skill-installer` пишет в `.claude/skills/`, `.cursor/skills/`, etc. — это derived artifacts outside `.forgeplan/`. ADR-003 invariant scoped только на `.forgeplan/`. Нет policy: как обновлять при user-edited installed skill, round-trip, eject-flow. Hash-tracking (FR-7) не эквивалент markdown-primary модели. Fix: ADR-008 amendment с derived-skill-file policy + `forgeplan skill eject` command в PRD-062.

**C3 — "Atomic" bidirectional supersede без механизма** (architect + DDD 7.2). PRD-063 AC-5 требует atomic rollback across 2 aggregates (ADR-005 + ADR-012, projection + DB). LanceDB без multi-row tx, markdown writes sequential. Текущая формулировка — aspiration. Fix: переписать на journaled replay — intent record в `.forgeplan/journal/`, apply side A, apply side B, mark complete, replay on startup. Альтернатива: scope down до "best effort + `forgeplan doctor --fix-links`".

**C4 — Depth=critical без Spec/RFC artifacts** (code-analyzer). ADR-008 body claims "depth=critical, pipeline PRD→Spec→RFC→ADR", но frontmatter depth=deep и нет Spec/RFC для data contracts (migration-plan.schema.json, agent-manifest.schema.json, new crate forgeplan-skill-installer). Fix-option 1: создать SPEC-migration-plan, SPEC-agent-manifest, RFC-skill-installer-architecture. Fix-option 2: формально downgrade body claim на `deep` + justification что contracts embedded в PRDs.

**C5 — MigrationPlan aggregate ownership undefined** (DDD 3.1/3.2). Три writer'а (PRD-059 discover, PRD-061 classify skill, PRD-062 init wizard) без единого invariant-owner. PlanEntry entity state machine не названа. Anti-pattern: shared mutable state across contexts. Fix: MigrationPlan aggregate в Migration context (PRD-059 owns); другие contexts через commands + domain events (EntryClassified, EntryDecided); PRD-061 skills НЕ пишут plan.json напрямую — через `forgeplan plan update` CLI с invariant checks.

**C6 — status_map as leaky translator, не proper ACL** (DDD 8.1). Mapping (MADR/ADR-tools/log4brains/Obsidian) размазан по ADR-008 prose + PRD-059 + PRD-063 FR-7. Нет single module, per-dialect detection, versioning. Fix: BrownfieldStatusTranslator ACL module в Migration context, input (source_vocabulary, source_status), output (forge_status, confidence, warning). Per-dialect tests. Applies аналогично к link-mapping PRD-064.

### HIGH (12)

**H1 — Classification context homeless** (DDD 1.1). EPIC claims context, но work split: PRD-059 heuristic (kind_hint) + PRD-061 LLM. Two different confidence semantics. Fix: publish ClassificationResult value object; both PRDs conform.

**H2 — PRD-062 conflates Discovery + Skill Distribution** (DDD 1.2). BrownfieldScanner (Discovery) + HarnessAdapter (Distribution) — разные ubiquitous language, invariants, dependencies. Fix: split PRD-062 на PRD-062a (brownfield-detect) + PRD-062b (multi-harness-installer), либо new PRD added.

**H3 — Dialogue context in-name-only** (DDD 1.3). EPIC lists но нет aggregate. Fix: либо downgrade на capability of Classification, либо add DialogueSession aggregate.

**H4 — "skill" terminology overloaded** (DDD 2.1). 3 значения (SkillPackage/InstalledSkill/SkillReference) используются взаимозаменяемо. Fix: disambiguate в glossary + consistent usage.

**H5 — EVID-079 CL2 too weak для deep decision** (architect H3). Research-only evidence для решения с blast radius 1500+ LOC new crate + 7 adapters. Fix: spike EVID-080 — реально установить SKILL.md в Claude Code + Cursor, верифицировать loading in both. CL3 measurement required before `forgeplan activate` chains для PRDs.

**H6 — Context map absent** (DDD 4.1). Contexts listed без integration patterns (shared kernel / customer-supplier / published language). Fix: add "Context Map" section в EPIC-006 + integration-pattern declarations.

**H7 — Per-kind invariants under-specified** (DDD 6.1). PRD-064 MUST sections есть, но invariants нет (postmortem MUST have caused_by; runbook MUST have responds_to). Fix: "Invariants" subsection per kind + enforce в validator.

**H8 — Domain events implicit** (DDD 7.1). "migrate applied", "projection written", "skill installed" cross context boundaries но не first-class. Fix: enumerate events в ADR-008 Domain Events subsection или companion RFC.

**H9 — Completed/Archived orthogonal axes conflated** (DDD 5.2). completed mixes "work done" (method) + "housekeeping" (visibility). Two axes в one enum. Fix: либо split (work_state + visibility), либо justify single-enum в ADR-008 amendment.

**H10 — AC not testable** (code-analyzer #5). PRD-059 AC-4 (rollback — нет fault injection), PRD-062 AC-3 ("green" undefined), PRD-063 AC-2 (R_eff freeze без numeric check), PRD-064 AC-5 (semantic "relevant"). Fix: specify measurable criteria per AC.

**H11 — Orphan FRs** (code-analyzer #6). FRs без AC coverage: PRD-060 FR-9, PRD-061 FR-5, PRD-062 FR-9, PRD-063 FR-5, PRD-064 FR-7, PRD-059 FR-8. Fix: add AC per orphan FR или mark invariant explicitly.

**H12 — 44-file Obsidian fixture не закоммичен** (architect M5). Все E2E AC ссылаются на `tests/fixtures/obsidian-vault-44` который не существует. Fix: commit anonymized fixture как DoR blocker before Code-phase PRD-059 начинается.

### MEDIUM (15) — follow-up, не блокирует activate

context injection size cap для rules_per_kind; migrate idempotency под schema evolution; adapter-drift CI matrix; Epic sizing (0/?); AGENT.md validation в PRD-061; `.forgeplan/migration/` dir в ADR-003 storage; PRD-063 Goal 5 без FR/AC; PRD-064 FR-3 half-spec; EPIC Dependencies missing EVID-079 (fixed Phase B); Implementation Plan duplication ADR vs PRD; PROB-022 supersede vs deprecate semantics; migration-plan directory layout; KB identity rule across re-runs; meeting kind justification.

### LOW (8) — housekeeping

Refresh Triggers missing agentskills.io v1.0; Progress placeholders 0/?; FR-2 inline schema vs schema file; PRD-064 meeting informs ADR-005; EVID draft→active post-code; `_hints` → `x-forgeplan-hints` (fixed Phase B); ADR-008 rollback step 3 wrapper broken (fixed Phase B); skill pack compat matrix; meeting 180d arbitrary (fixed Phase B — config-driven).

## Proposed Solution (next-iteration work order)

### Iteration 2 — close CRITICAL (pre-code blockers)

1. **C1 reconcile** (~2h): Status enum alignment PR — rename `RefreshDue → Stale` или add alias. Updates CLAUDE.md + ADR references. Tests pass.
2. **C4 depth decision** (~1h): formal downgrade ADR-008 body claim to `deep` + justification, ИЛИ create SPEC-XXX + RFC-XXX стабы для 3 data contracts.
3. **C2 ADR-008 amendment** (~2h): append derived-skill-file policy section. Add `forgeplan skill eject` command в PRD-062 FR list.
4. **C3 PRD-063 rewrite** (~3h): replace "atomic" language с journaled-replay mechanism. Describe `.forgeplan/journal/` write path.
5. **C5 MigrationPlan aggregate doc** (~2h): PRD-059 ownership section + PRD-061 restrict к `plan update` CLI commands.
6. **C6 ACL module spec** (~2h): PRD-059 + PRD-063 add BrownfieldStatusTranslator module design.

### Iteration 3 — close HIGH (before activate PRD)

- H1 Publish ClassificationResult value object (PRD-059 + PRD-061 aligned)
- H2 Split PRD-062 → two PRDs (либо keep 062 as orchestrator)
- H5 Spike EVID-080 — real cross-harness install test (Claude Code + Cursor)
- H10 AC rewrite — measurable criteria
- H11 Orphan FRs mapped to AC
- H12 Fixture 44-file commit
- H3/H4/H6/H7/H8/H9 — smaller edits по каждой

### Iteration 4 — close MEDIUM+LOW during Code phase

Остальные findings адресуются либо в code, либо через follow-up PRs.

## Acceptance Criteria (for closing PROB-040)

- [ ] Iter-2 work items C1..C6 completed как separate commits или PRs
- [ ] Iter-3 HIGH items H1..H12 адресованы: fix in PRD body или downgrade в Non-Goals или split в новый артефакт
- [ ] Audit re-run (same 4 agents) — 0 CRITICAL, ≤3 HIGH findings
- [ ] EVID-080 created (CL3 measurement) и linked к ADR-008
- [ ] PRD-059..064 ready to activate (R_eff > 0 после code evidence)
- [ ] EPIC-006 Success Criteria measurable (все 10 с numeric targets или verifiable fixture)

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-008 | ADR | informs (decision under review) |
| EPIC-006 | Epic | informs (scope under review) |
| PRD-059..064 | PRD | informs |
| EVID-079 | Evidence | informs (shape iter 1 support, needs complement by EVID-080 CL3) |
| PROB-022 | Problem | informs (original brownfield onboarding problem this chain addresses) |




