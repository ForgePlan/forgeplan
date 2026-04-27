---
created: 2026-04-20
depth: standard
id: PRD-069
kind: prd
links:
- target: EPIC-007
  relation: refines
- target: ADR-009
  relation: based_on
status: draft
title: forge-orchestrator agent + forge-ingest skill + forge-scaffolder agent
updated: 2026-04-20
---

# PRD-069: forge-orchestrator agent + forge-ingest skill + forge-scaffolder agent

## Problem

PRD-065 playbook runtime — infrastructure, но чтобы playbooks работали в agent-skills экосистеме (Claude Code / Cursor / etc.) нужны **agent-side** primitives: orchestrator agent который понимает как выполнить playbook шаг через delegates, ingest skill exposed как callable capability, scaffolder agent для greenfield bootstrap. Без этих agent-side artifacts forgeplan runtime работает только из CLI, не из агентской сессии.

## Goals

1. **forge-orchestrator AGENT.md** — role per agent-skills standard, invokes playbook steps через Task tool, aggregates results
2. **forge-ingest SKILL.md** — skill wrapping CLI `forgeplan ingest` для agent-side invocation
3. **forge-scaffolder AGENT.md** — greenfield kickoff agent (interacts with user, runs greenfield-kickoff playbook)
4. Published в brownfield-code-pack + greenfield-pack

## Non-Goals

- NOT reimplements playbook runtime — delegates to `forgeplan playbook run`
- NOT LLM-based mapping logic — uses CLI `forgeplan ingest` which has declarative YAML
- NOT auto-activates — все results remain `draft` until user explicit activate

## Target Users

- **Pack author** — consumes этот runtime/ingest/detection как building block
- **Forgeplan user** — invokes playbooks via `forgeplan playbook run` (доп. к базовому workflow)
- **External plugin author** — публикует mappings для intergration с forge-graph

## Success Criteria / Acceptance

- **AC-1**: All 3 agents/skills pass BMAD skill-validator 14 rules
- **AC-2**: forge-orchestrator successfully runs `brownfield-code` playbook on fixture — 7 steps complete, artifacts ingested
- **AC-3**: forge-scaffolder on empty repo → ADR-001 + EPIC-001 + 5 PRD stubs drafted with correct frontmatter
- **AC-4**: forge-ingest skill callable from other agents — input: mapping + source → output: forge artifacts drafted
- **AC-5**: Works в Claude Code + Cursor + Windsurf harness (3 of 7 minimum CL3 verified)
- **AC-6**: All produce `draft` artifacts — no auto-activation; user has review gate

## Functional Requirements

- **FR-1** `marketplace/{brownfield-code,greenfield}-pack/agents/forge-orchestrator/AGENT.md` и aналогично scaffolder
- **FR-2** forge-ingest SKILL.md с SKILL-06 compliant description (Use-when clause)
- **FR-3** Agent strategy: parse playbook YAML → Task tool invocations with specialized agents per step
- **FR-4** forge-scaffolder integration with `/autoresearch:plan` pattern for vision capture (4-question wizard)
- **FR-5** All 3 artifacts distributed через forgeplan-skill-installer (PRD-062 in EPIC-006 scope — now re-framed)

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
| PRD-065 | PRD | informs (agent uses playbook runtime) |
| PRD-066 | PRD | informs (uses ingest engine) |
| PRD-067 | PRD | informs (uses plugin detection) |



