---
depth: tactical
id: PROB-017
kind: problem
links:
- target: PRD-006
  relation: informs
- target: PRD-020
  relation: informs
status: deprecated
title: Router gives single answer — should offer alternatives when context conflicts
---

# PROB-017: Router gives single answer — should offer alternatives when context conflicts

## Signal

`forgeplan route` returns a single depth/pipeline pair. When multiple signals conflict (e.g., "security" pushes Deep but scope is small), the user sees only the max-depth winner with no visibility into alternatives. AI agents cannot evaluate trade-offs or override intelligently because they receive one option, not a ranked set.

Observed during dogfooding: route says "Deep" for a 2-file security fix. User must manually guess that Standard might suffice. No programmatic way to compare.

## Constraints

- Must not break existing `route()` API — alternatives are additive
- Must work at Level 0 (rule-based, offline) — no LLM dependency for alternatives
- Response latency must stay under 50ms for Level 0

## Optimization Targets (1-3 max)

- Decision quality: AI agent picks correct depth on first attempt
- Transparency: user sees WHY a depth was chosen and what else was considered

## Observation Indicators (Anti-Goodhart)

- Do NOT optimize for "number of alternatives shown" — always exactly 2, not more
- Monitor: how often users override the primary recommendation (lower is better)

## Acceptance Criteria

1. `forgeplan route "description"` shows primary result + 2 alternatives with reasoning
2. MCP `forgeplan_route` response includes `_alternatives` array with depth/pipeline/reasoning
3. Each alternative explains why it could be appropriate (e.g., "Standard if scope is limited to 1-2 files")
4. All existing routing tests pass unchanged
5. At least 4 new unit tests for alternatives generation

## Blast Radius

- `crates/forgeplan-core/src/routing/mod.rs` — RoutingResult struct, generate_alternatives()
- `crates/forgeplan-cli/src/commands/route.rs` — CLI display
- `crates/forgeplan-mcp/src/server.rs` — MCP JSON response

## Reversibility

high — additive feature, no breaking changes to existing API

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-006 | informs |
| PRD-020 | informs |


