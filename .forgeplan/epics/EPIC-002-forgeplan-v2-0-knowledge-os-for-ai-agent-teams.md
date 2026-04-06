---
depth: standard
id: EPIC-002
kind: epic
status: draft
title: Forgeplan v2.0 — Knowledge OS for AI Agent Teams
---

# EPIC-002: Forgeplan v2.0 — Knowledge OS for AI Agent Teams

## Vision

Forgeplan трансформируется из CLI tool (single player, local DB) в Knowledge OS — единый слой между AI агентами и проектом. Агент думает ЧЕРЕЗ Forgeplan, а не просто вызывает его.

## Architecture Layers

```
L3: REASONING      — AI agent reasoning через FPF patterns, ADI cycle
L2: KNOWLEDGE GRAPH — артефакты + связи + evidence + code nodes
L1: STORAGE + MEMORY — LanceDB (vectors) + petgraph + Hindsight bridge
```

## Key Concepts

### Markdown-first (Team Mode)
.forgeplan/*.md = source of truth (git-tracked).
LanceDB = local cache (gitignore, rebuild via `forgeplan sync`).
Enables: git merge, PR review, multi-developer collaboration.

### Code Awareness (Carrier Ref)
Evidence links to code files/commits via carrier_ref.
Enables: "Which PRD led to this code?", drift detection with baselines.

### Reasoning Context
`forgeplan context PRD-001 --json` = single MCP call with:
artifact state, derived status, graph neighbors, code impact, health, suggestions, FPF patterns.

### DerivedStatus (Computed Lifecycle)
STUB → SHAPED → VALIDATED → CODED → EVIDENCED → ACTIVATED
Computed from artifact state, not stored. Shows real progress.

### In-memory Graph
petgraph DiGraph loaded at open_store(). Fast traversal for R_eff recursive, dependency chains.

### Team Roles
author, reviewer, assigned_to fields. `forgeplan assign`, `forgeplan review --approve`.

### Integrations
- Orchestra: bidirectional task sync
- Hindsight: personal memory bridge
- Git CI: `forgeplan pr-check` validates artifacts in PR

## Roadmap

| Version | Focus | Key features |
|---------|-------|-------------|
| v0.11 | Knowledge OS core | Activation gate, DerivedStatus, context command, in-memory graph |
| v0.12 | Code awareness | Carrier ref, watch, diff |
| v0.13 | Team mode | Markdown-first, sync, author/reviewer, PR check |
| v0.14 | Integrations | Orchestra sync, Hindsight bridge |
| v1.0 | Production | Distribution (brew, crates.io), stability, docs |

## Children

| Artifact | Title | Status |
|----------|-------|--------|
| PRD-022 | Activation Gate + DerivedStatus | TBD |
| PRD-023 | Reasoning Context command | TBD |
| PRD-024 | In-memory Graph (petgraph) | TBD |
| PRD-025 | Carrier Ref + Code Awareness | TBD |
| PRD-026 | Markdown-first Team Mode | TBD |
| PRD-027 | Orchestra + Hindsight Integration | TBD |

## Source Analysis

Based on gap analysis of quint-code, BMAD-METHOD, OpenSpec (2026-03-25):
- 75% of source concepts ported
- 15 innovations beyond sources
- 19 gaps identified, 4 critical

