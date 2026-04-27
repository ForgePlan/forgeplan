---
created: 2026-04-20
depth: standard
id: PRD-067
kind: prd
links:
- target: EPIC-007
  relation: refines
- target: ADR-009
  relation: based_on
status: draft
title: Plugin detection + self-describing hints playbook recommendations
updated: 2026-04-20
---

# PRD-067: Plugin detection + self-describing hints playbook recommendations

## Problem

Forgeplan сейчас не знает какие плагины установлены на машине пользователя, следовательно не может рекомендовать applicable playbooks. Self-describing hints (ADR-008) emitted после каждой команды, но они generic — не адаптированы к project signals (empty repo vs legacy с docs vs code-only). Пользователь не получает guided workflow через brownfield или greenfield.

## Goals

1. **Plugin detection** — forge знает какие plugins installed (c4-architecture, autoresearch, ddd-domain-expert, etc.) и их versions
2. **Context-aware hints** — `forgeplan init` на empty repo рекомендует greenfield playbook; на legacy code — brownfield-code; на docs vault — brownfield-docs
3. **Install hints** — missing plugin → exact install command в stderr
4. **ADR-008 extension** — existing self-describing hints расширяются с playbook recommendations

## Non-Goals

- NOT auto-installs plugins — только рекомендует + гарантирует exact command
- NOT changes behavior of base commands (new/validate/activate) — только добавляет hint context
- NOT requires plugins — works with zero plugins (просто меньше рекомендаций)

## Target Users

- **Pack author** — consumes этот runtime/ingest/detection как building block
- **Forgeplan user** — invokes playbooks via `forgeplan playbook run` (доп. к базовому workflow)
- **External plugin author** — публикует mappings для intergration с forge-graph

## Success Criteria / Acceptance

- **AC-1**: `forgeplan plugins list` показывает installed/missing plugins с версиями
- **AC-2**: `forgeplan plugins doctor` — health check + recommendations per installed pack
- **AC-3**: `forgeplan init` на empty repo → stderr hint `recommended: greenfield-kickoff playbook (requires autoresearch plugin)`
- **AC-4**: `forgeplan init` на repo с `.obsidian/` → hint brownfield-docs playbook
- **AC-5**: `forgeplan init` на repo с >100 commits + no docs → hint brownfield-code playbook
- **AC-6**: Missing plugin → stderr emits exact install command (`claude plugin install autoresearch` / `forgeplan skill install brownfield-docs-pack`)
- **AC-7**: Backward compat: no hints emitted when `FORGEPLAN_HINTS=0` or stderr not TTY

## Functional Requirements

- **FR-1** Rust module `forgeplan-core::plugins::{detection,registry,hints}`
- **FR-2** Detection scanner paths: `~/.claude/plugins/cache/`, `.claude/plugins/`, `.agentskills/`, `.cursor/skills/`, etc.
- **FR-3** Plugin registry: known plugins с expected paths + versions (c4-architecture, autoresearch, ddd-domain-expert, specification, brownfield-docs-pack, etc.)
- **FR-4** Project signal detector: empty_repo, legacy_code_no_docs, docs_vault_present, has_package_json, has_cargo_toml, git_commit_count
- **FR-5** Playbook recommendation engine: signals × installed_plugins → applicable playbooks
- **FR-6** CLI: `forgeplan plugins {list|doctor|info <name>}`
- **FR-7** Hint extension ADR-008 pattern — emit `recommended_playbook` and `install_hint` fields в existing hint format

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
| ADR-008 | ADR | informs (self-describing hints extended) |
| EPIC-007 | EPIC | refines |
| PRD-065 | PRD | informs (runtime uses detection for missing plugins) |



