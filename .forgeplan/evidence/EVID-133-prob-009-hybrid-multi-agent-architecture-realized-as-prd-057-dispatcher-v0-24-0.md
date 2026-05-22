---
depth: standard
id: EVID-133
kind: evidence
last_modified_at: 2026-05-22T22:55:29.967767+00:00
last_modified_by: claude-code/2.1.149
links:
- target: PROB-009
  relation: informs
status: draft
title: PROB-009 Hybrid multi-agent architecture realized as PRD-057 dispatcher (v0.24.0)
---

# EVID-133: PROB-009 Hybrid multi-agent architecture realized as PRD-057 dispatcher

## Structured Fields

verdict: supports
congruence_level: 2
evidence_type: audit

## Summary

PROB-009 в мае 2026 предложил 5 архитектурных подходов к multi-agent orchestration для Forgeplan; явный winner по оценочной матрице — Подход 5 (Hybrid: role-based agents + git worktrees + memory bridge), score 48/60. PROB также определил поэтапный roadmap Phase A → D.

К моменту фиксации этого evidence (2026-05-23) **Phase A + B + C из roadmap реализованы**:
- **Phase A — Core + Roles**: forgeplan_context, petgraph in-memory graph, DerivedStatus, activation gate — все вошли в v0.11..v0.17 ([CHANGELOG](../../CHANGELOG.md)).
- **Phase B — Code Awareness**: carrier ref evidence→file, watch hooks — landed в v0.18..v0.22.
- **Phase C — Multi-Agent**: markdown-first source of truth (ADR-003 enforced), forgeplan dispatch+claim+release (PRD-057 v0.24.0), worktree integration pattern в CLAUDE.md — landed в v0.24.0 sprint.

**PRD-057 commit trail**: 9 commits, R2+R3 audits (30 findings closed), 1391 tests, EVID-077 score=1.00. Documented в memory: project_v0_24_0_prd057_sprint.md.

**Phase D — Memory + Integration** реализован частично: Hindsight bridge активен в дочерних проектах (fpl-hsmem plugin), forgeplan_search semantic уже есть. `forgeplan recall` cross-memory search ещё не реализован (открытый scope).

## Mapping: PROB-009 предложение → реализация

| PROB-009 proposal | Реализовано как | Где |
|---|---|---|
| Role-based agents (Подход 1) | sub-agent profiles (Profile A/B/C/D) + claude-code subagent_type whitelist | `agents-core:*`, `agents-pro:*` plugins |
| Shared knowledge graph + locks (Подход 2) | forgeplan_claim/release с TTL + advisory locks | PRD-057, `mcp__forgeplan__forgeplan_claim` |
| Git-native collaboration (Подход 4) | git worktree pattern для ≥3 parallel workers | CLAUDE.md "Multi-agent worktree pattern", feedback_use_worktrees_per_parallel_worker memory |
| Hybrid (Подход 5) | dispatch → claim → spawn pattern | `forgeplan_dispatch` + Pattern A (Team Lead + Workers) + Pattern B (single-message parallel) в CLAUDE.md |
| Variant C двойная memory | Forgeplan structured + Hindsight unstructured | `.mcp.json` декларирует обе MCP, project/feedback memories pointer-style |

## Outstanding (deferred to future scope, not blocking)

- `forgeplan_recall` (cross-memory semantic merge) — Phase D leftover, не критичный
- Per-role tool whitelists через server-side enforcement — сейчас на стороне subagent_type whitelist в Claude Code, не в forgeplan
- Skill/Agent template formal yaml registry — частично реализован через plugin manifest, не полный PROB-009 формат

## Verdict rationale

`supports` — PROB-009 предложил архитектурное решение; ≥3 из 4 Phase реализованы в production; нет drift между proposal и реализацией; outstanding items orthogonal к core ценности (multi-agent работает).

`congruence_level: 2` — PROB-009 (proposal context) → PRD-057 implementation (different context same domain). Не CL3 потому что между PROB и реализацией прошёл рефакторинг scope; не CL1 потому что концептуальное соответствие явное.

`evidence_type: audit` — это аудит-трассировка proposal→implementation, не measurement (perf) и не test_result.

## Cross-references

- `Refs: PRD-057, EVID-077, ADR-003, project_v0_24_0_prd057_sprint memory`
- CLAUDE.md sections: "Multi-agent (v0.24.0+)", "AgentTeams orchestration patterns", "Multi-agent worktree pattern"

