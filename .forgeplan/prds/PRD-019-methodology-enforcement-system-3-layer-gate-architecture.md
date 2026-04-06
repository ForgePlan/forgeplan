---
depth: tactical
id: PRD-019
kind: prd
links:
- target: PRD-020
  relation: supersedes
status: active
title: Methodology Enforcement System — 3-layer gate architecture
---

## Problem

Forgeplan methodology (Shape→Validate→Code→Evidence→Activate) is documented in CLAUDE.md but NOT enforced. During PROB-012 sprint, AI agent skipped 3 steps:
1. No PRD created before coding (route said Standard+)
2. No Evidence created after implementation
3. Priority downgrade (P0→P1) without formal Note/ADR

Root cause: enforcement relies on CLAUDE.md guidance (soft) — no hard gates prevent skipping phases.

## Goals

- Zero methodology bypasses: every Standard+ task follows Shape→Validate→Code→Evidence→Activate
- Phase-aware enforcement: system knows current phase and blocks out-of-order actions
- Graceful for Tactical: lightweight tasks skip gates without friction
- Self-documenting: each gate decision logged for audit trail

## Non-Goals

- Replacing human judgment — gates can be overridden with justification
- Full CI/CD integration — this is local dev workflow
- Multi-user enforcement — single agent/user scope

## Target Users

- AI agents (Claude Code) working on Forgeplan
- Human developers following Forgeplan methodology

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-012 | motivated_by — integrity sprint exposed methodology gaps |
| EPIC-001 | parent — Forgeplan v1.0 methodology engine |

## Functional Requirements

### Layer 1: Enhanced CLAUDE.md (Guidance)
- [FR-001] Explicit phase state machine diagram in CLAUDE.md
- [FR-002] Per-phase checklist with exact CLI commands
- [FR-003] Decision tree: "if route says X → you MUST do Y before Z"

### Layer 2: Hooks (Enforcement Gates)
- [FR-004] PreToolUse hook: before Edit/Write on crates/ → check forgeplan for active PRD (Standard+)
- [FR-005] PreToolUse hook: before git commit → check forgeplan health blind spots
- [FR-006] PreToolUse hook: before gh pr create → check TODO P0 + Evidence exists + R_eff > 0
- [FR-007] Hook output instructs agent to invoke /fpf-simple for reasoning on blocked actions

### Layer 3: MCP State Machine (Context Awareness)
- [FR-008] New MCP tool: forgeplan_phase — returns current phase (idle/routing/shaping/coding/evidence/pr)
- [FR-009] New MCP tool: forgeplan_guard — validates phase transition (e.g. coding→evidence OK, coding→pr BLOCKED)
- [FR-010] Session state persisted in .forgeplan/session.yaml (current_phase, active_prd, route_depth)
- [FR-011] forgeplan route auto-sets session phase to routing→shaping
- [FR-012] forgeplan validate PASS auto-transitions shaping→coding
- [FR-013] forgeplan new evidence auto-transitions coding→evidence

### Integration
- [FR-014] /forge skill reads session phase and auto-advances through gates
- [FR-015] UserPromptSubmit hook outputs current phase in context reminder


