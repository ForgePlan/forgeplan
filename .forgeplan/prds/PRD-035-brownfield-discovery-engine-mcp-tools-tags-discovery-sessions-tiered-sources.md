---
depth: deep
id: PRD-035
kind: prd
links:
- target: PROB-022
  relation: based_on
- target: EPIC-002
  relation: based_on
- target: NOTE-041
  relation: refines
- target: EPIC-003
  relation: refines
status: draft
title: Brownfield Discovery Engine — MCP tools, tags, discovery sessions, tiered sources
---

# PRD-035: Brownfield Discovery Engine

## Progress

```
Phase 1  ░░░░░░░░░░░░░░░░░░░░░░░░  0/8  (  0%)
Phase 2  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/13 (  0%)
```

## Problem

При установке ForgePlan на brownfield/legacy проект нет structured способа создать knowledge base из существующего кода. AI агент идёт в docs/ вместо кода, строит narrative вокруг одного документа, теряет контекст.

Root cause (PROB-022): FPL не имеет discovery protocol, tag system, source tier priority, и session tracking.

Impact: ForgePlan бесполезен на brownfield проектах без ручной работы по созданию артефактов.

## Goals

- [Agent] can call forgeplan_discover_start и получить structured protocol
- [Agent] can call forgeplan_discover_finding для каждой находки с tier, kind, source files
- [Agent] can call forgeplan_discover_complete для завершения session
- [Developer] can run forgeplan discover и получить knowledge base из brownfield проекта
- [Developer] can tag артефакты (forgeplan tag id legacy-doc)
- [System] can assign trust tier (CL3/CL2/CL1) на основе source tier
- [System] can track discovery sessions с progress и artifact inventory

## Non-Goals

- Парсинг кода самим ForgePlan (агент делает это)
- Language-specific AST analysis
- Auto-fix найденных проблем
- Plugin marketplace integration
- DSL/Lua/Rhai scripting (NOTE-039, Phase 3+)

## Target Users

| Persona | Description | Key pain |
|---------|------------|----------|
| Solo developer | Ставит FPL на legacy проект | Нет способа создать knowledge base |
| AI agent (MCP) | Выполняет discovery protocol | Нет structured tools |
| Team lead | Onboards новых членов | Нет автодокументации кода |

## Functional Requirements

### Phase 1: Core Infrastructure (Sprint 13)

- [ ] FR-001: [System] can store tags on artifacts (frontmatter field tags: [])
- [ ] FR-002: [Developer] can add/remove tags: forgeplan tag/untag id key=value
- [ ] FR-003: [Developer] can filter by tags: forgeplan list --tag source=legacy-doc
- [ ] FR-004: [MCP] forgeplan_discover_start(project_name) creates session, returns protocol
- [ ] FR-005: [MCP] forgeplan_discover_finding(session, phase, tier, kind, title, body) creates artifact + links + tags
- [ ] FR-006: [MCP] forgeplan_discover_complete(session) generates summary + health check
- [ ] FR-007: [CLI] forgeplan discover creates session, outputs protocol, tracks progress
- [ ] FR-008: [System] map source tier to CL for R_eff: tier 1=CL3, tier 2=CL2, tier 3=CL1

### Phase 2: Deepening and Multi-Pass (Sprint 14+)

- [ ] FR-009: [CLI] forgeplan discover --deep runs Pass 1 + 2 (per-artifact deepening)
- [ ] FR-010: [System] track discovery pass number per session
- [ ] FR-011: [CLI] forgeplan discover --full runs Pass 1 + 2 + 3 (synthesis + gap analysis)
- [ ] FR-012: [System] detect gaps: module A depends on B but B has no RFC/Spec
- [ ] FR-013: [System] detect contradictions: docs vs code, auto-create Problem

## Success Criteria

| ID | Criterion | Target |
|----|-----------|--------|
| SC-1 | Discovery creates knowledge base | 10+ artifacts from code analysis |
| SC-2 | Code-first compliance | 100% Tier 1 before Tier 3 |
| SC-3 | Tags filter working | forgeplan list --tag returns correct results |
| SC-4 | Trust tier mapping | Tier 1=CL3, Tier 3=CL1 in R_eff |

## Acceptance Criteria

### AC-1: Discovery creates artifacts from code
Given a brownfield project with src/, package.json, and docs/
When agent runs forgeplan discover protocol
Then at least 5 Tier 1 artifacts created from code analysis
And Tier 3 docs artifacts tagged legacy-doc

### AC-2: Tags system works
Given an artifact PRD-001
When developer runs forgeplan tag PRD-001 source=legacy-doc
Then forgeplan list --tag source=legacy-doc includes PRD-001

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tags migration breaks artifacts | Data loss | Additive: tags optional, default empty |
| Too many artifacts from discover | Noise | Protocol limits per phase |
| Large projects overflow context | Incomplete | Sampling strategy in protocol |

## Rollback Plan

Tags: additive field, removal = delete field. Discovery tools: new, dont affect existing. Git revert safe.

## Timeline

| Milestone | Sprint |
|-----------|--------|
| Tags system (FR-001-003) | Sprint 13 |
| Discovery MCP tools (FR-004-006) | Sprint 13 |
| CLI discover command (FR-007) | Sprint 13 |
| Multi-pass deepening (FR-009-013) | Sprint 14 |

## Stakeholders

| Role | Sign-off |
|------|----------|
| Developer (author) | [x] |
| AI agent (consumer) | [ ] via E2E test |

## Dependencies

| Dependency | Status |
|-----------|--------|
| Tags system (FR-001-003) | Not started |
| Marketplace discover agent | V1 ready |
| FPF rule engine (RFC-001 Phase 2) | Done (PR #135) |

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-022 | based_on |
| EPIC-002 | child_of |
| NOTE-041 | refines |
| NOTE-039 | informs (DSL) |
| ADR-003 | constrained_by |
| ADR-006 | constrained_by |


