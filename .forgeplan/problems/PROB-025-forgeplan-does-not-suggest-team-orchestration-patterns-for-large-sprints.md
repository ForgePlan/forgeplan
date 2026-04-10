---
depth: tactical
id: PROB-025
kind: problem
links:
- target: EPIC-003
  relation: informs
- target: NOTE-043
  relation: based_on
status: deprecated
title: Forgeplan does not suggest team orchestration patterns for large sprints
---

---
id: PROB-025
title: "Forgeplan does not suggest team orchestration patterns for large sprints"
status: Draft
created: 2026-04-07
depth: standard
context: "tooling-integration"
parent_epic: EPIC-003
---

# PROB-025: Forgeplan does not suggest team orchestration patterns for large sprints

## Signal

Real incident (2026-04-07, Sprint 13.1):

1. User asked to run Sprint 13.1 via `/sprint` workflow
2. `/sprint` skill spawned team-lead via Agent (general-purpose)
3. Team-lead reported BLOCKER: "I do not have an Agent tool in my available function set"
4. Manual workaround was needed: main thread spawned all teammates, team-lead only coordinated
5. **Forgeplan itself had no awareness** that this pattern even exists. No CLI hint, no MCP tool, no documentation, no template.
6. Marketplace agents (in `agents/`) also have no knowledge of this pattern.

The pattern (documented in NOTE-043) WORKS — Sprint 13.1 finished successfully with 11 agents, 4 waves, 22 minutes. But it was discovered ad-hoc, not suggested by tooling.

## Constraints

- MUST not break existing `/sprint` and `/team-up` skills (additive)
- MUST work with current Agent Teams system (no upstream changes)
- MUST be discoverable by AI agents (CLI, MCP, marketplace)
- SHOULD reduce manual orchestration overhead in main thread
- SHOULD scale to 10+ teammate sprints

## Optimization Targets

1. **Discoverability** — when user runs `forgeplan route` for a multi-day task with 5+ FRs, system suggests sprint pattern
2. **Marketplace integration** — agents/<name>/agent.md files include orchestration hint
3. **Documentation** — `/sprint` skill, `/team-up` skill, CLAUDE.md all reference NOTE-043 pattern

## Observation Indicators (Anti-Goodhart)

- DO NOT optimize: "always suggest sprint pattern" — small tasks shouldn't have overhead
- MONITOR: how often users override the suggestion (signal of false positive)
- MONITOR: time to first commit in sprint vs solo execution

## Acceptance Criteria

1. **AC-1: Forgeplan route suggests pattern**
   - Given `forgeplan route "implement 5 FRs across 6 modules in 2 days"`
   - Then output includes "Sprint pattern recommended (estimated 5+ agents, 3+ waves)"
   - And suggests: `/sprint <task>` with hybrid main-spawn pattern reference

2. **AC-2: Marketplace agents know the pattern**
   - Given any marketplace agent in `agents/<name>/agent.md`
   - Then it has section "Team Coordination" referencing NOTE-043 pattern OR relevant skill

3. **AC-3: Documentation updated**
   - CLAUDE.md has "Team Orchestration" section
   - `/sprint` skill explicitly documents hybrid pattern
   - `/team-up` skill explicitly documents hybrid pattern

## Blast Radius

- `crates/forgeplan-core/src/routing/` — add team_orchestration heuristic
- `crates/forgeplan-cli/src/commands/route.rs` — display suggestion
- `crates/forgeplan-mcp/src/server.rs` — extend `forgeplan_route` MCP response
- `~/.claude/skills/sprint/` — update SKILL.md
- `~/.claude/skills/team-up/` — update SKILL.md
- `CLAUDE.md` — add Team Orchestration section
- `agents/*/agent.md` — marketplace updates
- Templates — sprint plan template includes pattern reference

## Reversibility

**High** — feature is additive (suggestion text, doc additions). No data migration. Rollback = revert PRs.

## Proposed Solution

PRD-044 (TBD) implementing:
1. Routing heuristic for "needs sprint pattern"
2. CLI/MCP suggestion in route output
3. Documentation updates
4. Marketplace agent template additions

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| EPIC-003 | informs (Sprint 13 quality of life) |
| NOTE-043 | based_on (the pattern itself) |
| PRD-043 | based_on (real case where pattern was discovered) |




