---
created: 2026-04-20
depth: standard
id: PRD-066
kind: prd
links:
- target: EPIC-007
  relation: refines
- target: ADR-009
  relation: based_on
status: draft
title: Ingest engine + mapping YAML format (c4-to-forge autoresearch-to-forge git-to-forge ddd-to-forge spec-to-forge)
updated: 2026-04-20
---

# PRD-066: Ingest engine + mapping YAML format

## Problem

Внешние плагины (c4-architecture, autoresearch, ddd-domain-expert, specification) выдают структурированные outputs (C4-Documentation/, docs/, domain-model.md), но forgeplan не знает как превратить их в forge artifacts (PRD/ADR/Epic/Note) с typed links к sources. Без mapping engine каждая интеграция — bespoke code. С engine — declarative YAML, reusable, testable.

## Goals

1. **Mapping YAML format**: source_kind + target_kind + field rules + links + source_ref schema
2. **5 core mappings CL3-validated**: c4-to-forge, autoresearch-to-forge, git-to-forge, ddd-to-forge, spec-to-forge
3. **Ingest CLI**: `forgeplan ingest --mapping <file> --source <path> [--dry-run]`
4. **Hallucination-proof**: каждый ingested artifact имеет `## Sources` section с `file:line` refs. `forgeplan doctor --sources` проверяет existence.

## Non-Goals

- NOT mutates source files — только читает и создаёт linked forge artifacts в `.forgeplan/`
- NOT embeds arbitrary code в YAML — только declarative rules
- NOT creates duplicates on re-run — idempotent update

## Target Users

- **Pack author** — consumes этот runtime/ingest/detection как building block
- **Forgeplan user** — invokes playbooks via `forgeplan playbook run` (доп. к базовому workflow)
- **External plugin author** — публикует mappings для intergration с forge-graph

## Success Criteria / Acceptance

- **AC-1**: Mapping `c4-to-forge.yaml` applied to fixture `C4-Documentation/` → forge artifacts matching expected schema (N PRD + 1 EPIC)
- **AC-2**: Each ingested artifact has `## Sources` section with accurate file:line refs
- **AC-3**: Re-run `forgeplan ingest` same source → updates existing, no duplicates
- **AC-4**: `forgeplan doctor --sources` on ingested artifacts → all source refs valid; if source deleted → reported as stale
- **AC-5**: Schema violation in mapping YAML → clear validation error, no partial ingest
- **AC-6**: 5 core mappings publish в `marketplace/mappings/` и pass integration tests

## Functional Requirements

- **FR-1** Rust module `forgeplan-core::ingest::{mapping,engine,sources}` with serde YAML types
- **FR-2** Mapping schema published at `docs/schemas/mapping.schema.yaml`
- **FR-3** CLI `forgeplan ingest` с --mapping, --source, --dry-run, --update/--replace flags
- **FR-4** Source-ref format: `{path}:{line}` or `{git_sha}:{path}:{line}` for git sources
- **FR-5** Idempotency via content hash — artifacts get `source_hash` field, compared on re-run
- **FR-6** 5 canonical mappings в marketplace/ с test fixtures
- **FR-7** `forgeplan doctor --sources` validates all ingested artifact source refs

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
| PRD-065 | PRD | informs (playbook runtime invokes ingest) |
| ADR-003 | ADR | informs (markdown source of truth) |



