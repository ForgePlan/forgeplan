---
created: 2026-04-20
depth: standard
id: PRD-068
kind: prd
links:
- target: EPIC-007
  relation: refines
- target: ADR-009
  relation: based_on
status: draft
title: forge-history-miner skill — git log to inferred ADR drafts
updated: 2026-04-20
---

# PRD-068: forge-history-miner skill — git log to inferred ADR drafts

## Problem

Существующие плагины (c4, autoresearch, ddd) покрывают structural + behavioral viewpoints brownfield кодбазы. Но **historical viewpoint** — inferred decisions from git history — не покрывается никем. Для легкаси проектов с 1000+ commits это богатая семантика: «когда мы выбрали postgres», «почему deprecated старый auth-middleware», «что меняло session logic чаще всего». Без этого skill brownfield migration теряет историю архитектурных решений.

## Goals

1. **Skill SKILL.md** per agent-skills standard — 'forge-history-miner'
2. **Input**: git log (with diffs), file blame for hot-spots, commit message summarization
3. **Output**: `.forgeplan/analysis/implicit-decisions.md` — list of candidate ADRs with git_sha + rationale + confidence
4. **Integration**: `git-to-forge.yaml` mapping (PRD-066) converts output → forge ADR drafts

## Non-Goals

- NOT rewrites git history — read-only mining
- NOT auto-creates ADRs в forge — produces drafts для review/ingest (explicit step)
- NOT supports non-git VCS (SVN, Mercurial) в v1 — git only

## Target Users

- **Pack author** — consumes этот runtime/ingest/detection как building block
- **Forgeplan user** — invokes playbooks via `forgeplan playbook run` (доп. к базовому workflow)
- **External plugin author** — публикует mappings для intergration с forge-graph

## Success Criteria / Acceptance

- **AC-1**: Skill manifest passes BMAD skill-validator 14 rules (name format, description с Use-when clause)
- **AC-2**: On Forgeplan repo (1000+ commits) produces ≥10 candidate ADRs с git_sha refs
- **AC-3**: Each candidate has structure: title + context (from commit msg) + inferred decision + confidence + source_ref
- **AC-4**: Heuristics filter: commits меняющие interface/signature count as decisions; merge/squash не counted
- **AC-5**: Integration test: skill output → `forgeplan ingest --mapping git-to-forge.yaml` → ADR drafts created в `.forgeplan/adrs/`
- **AC-6**: Hot-spot detection — files с >20 commits by >3 authors flagged as 'likely architectural flashpoints'

## Functional Requirements

- **FR-1** `marketplace/brownfield-code-pack/skills/forge-history-miner/SKILL.md` per agent-skills standard
- **FR-2** LLM prompt strategy: summarize commit → infer decision → score confidence
- **FR-3** Hot-spot scoring: commits_count × authors_count × file_size → importance rank
- **FR-4** Output schema: YAML list of {title, git_sha, context, decision, confidence 0-1, sources: [file:line]}
- **FR-5** Git analysis via shell delegate (git log --format, git blame) — deterministic layer before LLM
- **FR-6** Integration test fixture: small repo с 50+ commits и known decisions — validate extraction correctness

## Implementation Plan

### Phase 1: Foundation
- [ ] **1.1** Core types + schema (Rust + JSON Schema for YAML validation)
- [ ] **1.2** Unit tests — happy path + malformed inputs

### Phase 2: CLI/integration surface
- [ ] **2.1** CLI commands + help text
- [ ] **2.2** Integration tests on fixture

### Phase 3: Documentation + publication
- [ ] **3.1** `docs/` published
- [ ] **3.2** Example pack uses this capability

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-009 | ADR | based_on |
| EPIC-007 | EPIC | refines |
| PRD-066 | PRD | informs (output consumed by ingest engine) |
| PROB-022 | PROB | informs (brownfield onboarding depends on history viewpoint) |



