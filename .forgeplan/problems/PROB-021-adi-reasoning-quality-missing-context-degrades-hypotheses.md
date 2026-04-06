---
depth: tactical
id: PROB-021
kind: problem
status: active
title: ADI Reasoning Quality — missing context degrades hypotheses
---

---
id: PROB-021
title: "ADI Reasoning Quality — missing context degrades hypotheses"
status: Draft
created: 2026-04-03
depth: standard
context: "llm"
parent_epic: EPIC-001
---

# PROB-021: ADI Reasoning Quality — missing context degrades hypotheses

## Signal

`forgeplan reason PRD-004` generates 3 hypotheses, but H3 ("git-only chronology") is irrelevant — project already uses LanceDB as primary storage. The LLM lacks architectural context and produces noise.

F-G-R assessment of ADI output:
- F=4 (semi-formal JSON, but confidence unjustified)
- G=2 (partially relevant, 1 of 3 hypotheses is garbage)
- R=1 (no evidence bindings, recommendation untested)

Root cause: `reason.rs` user prompt passes only ID, Kind, Title, Body. Missing: status, depth, relations, project architecture summary.

## Constraints

- Must not break existing `reason` output format (AdiOutput JSON schema)
- Must not increase latency >5 seconds (currently ~18s)
- Prompt override via `.forgeplan/prompts/reason.md` must still work

## Optimization Targets (1-3 max)

- Relevance of hypotheses (G score: 2 → 3+)
- Confidence justification (F score: 4 → 5+)

## Observation Indicators (Anti-Goodhart)

- Total hypotheses count (should stay 3+, not collapse to 1)
- Response time (should stay under 25s)
- JSON parse success rate (should stay 100%)

## Acceptance Criteria

- 0 irrelevant hypotheses in output for PRD-004 and PRD-006
- Each confidence field has 1+ sentence justification
- Relations/status visible in prompt context
- All existing tests pass

## Blast Radius

- `crates/forgeplan-core/src/llm/reason.rs` — prompt changes
- `crates/forgeplan-cli/src/commands/reason.rs` — context building
- MCP tool `forgeplan_reason` — same core function

## Reversibility

high — prompt-only change, no data model or schema changes

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| EPIC-001 | parent |
| PRD-004 | test_subject |

