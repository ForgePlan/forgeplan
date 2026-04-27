---
created: 2026-04-20
depth: standard
id: PRD-065
kind: prd
links:
- target: EPIC-007
  relation: refines
- target: ADR-009
  relation: based_on
status: draft
title: Playbook YAML schema + runtime executor
updated: 2026-04-20
---

# PRD-065: Playbook YAML schema + runtime executor

## Problem

Без первоклассной playbook abstraction нельзя orchestrate multi-step workflows декларативно. Текущая forgeplan работает только через individual CLI commands: `new`, `validate`, `activate`. Чтобы выполнить brownfield-code flow (7 шагов c делегациями к c4/autoresearch/ddd) нужен скрипт, но без структуры он не переиспользуется между use cases и не интегрируется с lifecycle + scoring + graph.

## Goals

1. **Declarative playbook YAML**: schema с полями name, description, triggered_by, steps (delegate_to, command, input, produces_at, requires, mapping, fallback_hint), published at docs/schemas/playbook.schema.yaml
2. **Runtime executor** parses YAML, validates, resolves delegations, executes sequentially (parallel in v2), captures outputs
3. **CLI**: `forgeplan playbook {list|show <name>|run <name> [--yes] [--step N]|validate <file>}`
4. **Typed delegations**: plugin:X, agent:X, skill:X, command:X, forgeplan_core:X — no arbitrary shell без `command:` explicit opt-in

## Non-Goals

- NOT runs LLM directly — delegations via Task tool / MCP client / plugin subprocess
- NOT supports TOML/JSON playbook format — YAML только
- NOT auto-runs playbook at `forgeplan init` — always explicit user action

## Target Users

- **Pack author** — consumes этот runtime/ingest/detection как building block
- **Forgeplan user** — invokes playbooks via `forgeplan playbook run` (доп. к базовому workflow)
- **External plugin author** — публикует mappings для intergration с forge-graph

## Success Criteria / Acceptance

- **AC-1**: Valid `brownfield-code.yaml` playbook parses + validates OK via `forgeplan playbook validate`
- **AC-2**: `forgeplan playbook run brownfield-code --yes --dry-run` prints 7 steps w/ delegations, no execution
- **AC-3**: Actual run executes steps in order, progress to stderr, final report with counts (success/skipped/failed per step)
- **AC-4**: Missing plugin (e.g. c4-architecture uninstalled) → step fails with exact install command from `fallback_hint`, not crash
- **AC-5**: Playbook schema + validator catch malformed: missing required fields, unknown delegate types, invalid YAML
- **AC-6**: All 1405 existing tests pass — opt-in feature, no base workflow changes

## Functional Requirements

- **FR-1** Rust module `forgeplan-core::playbook::{types,loader,executor,dispatch}` with serde YAML types
- **FR-2** JSON Schema published at `docs/schemas/playbook.schema.yaml` for IDE autocomplete
- **FR-3** CLI subcommand `forgeplan playbook` with list/show/run/validate + flags (--yes, --step, --dry-run)
- **FR-4** Dispatcher with 5 delegate types (plugin via Task tool, agent via Task tool, skill invocation, command via shell, forgeplan_core internal call)
- **FR-5** Step output capture — produces_at path validated, mapped via ingest engine (PRD-066)
- **FR-6** Journal writes to `.forgeplan/journal/playbook-runs.jsonl` — resumable partial failure
- **FR-7** Progress reporting — stderr updates per step, TTY-aware
- **FR-8** Self-describing hints integration (PRD-067): when step fails — emit exact next action + install command

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
| PRD-066 | PRD | informs (runtime invokes ingest engine) |
| PRD-067 | PRD | informs (runtime uses plugin detection) |



