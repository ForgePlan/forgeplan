---
title: forgeplan_discover_complete
description: "Finalise a brownfield discovery session. ForgePlan groups findings by phase and tier, runs forgeplan_health on the workspace, and proposes PROBs / PRDs / RFCs synthesised from the findings. Proposed artifacts are printed — not auto-created — so the agent or human can review before committing."
---

Closes an active discovery session started by `forgeplan_discover_start`. ForgePlan walks the findings the agent reported, groups them by phase and tier, runs a project health pass, and synthesises a set of **proposed** follow-up artifacts (PROBs for risks, PRDs for requirements, RFCs for implementation shapes). Crucially, the proposals are **printed only** — they are not auto-created. The agent (or human) reviews them and decides which to promote into the workspace via `forgeplan_new`.

**Category**: Brownfield Discovery

## When an agent calls this

- After the seven protocol phases have been walked and all findings emitted.
- When the user says "wrap up discovery" / "propose next steps" / "finish the scan".
- Before starting any Shape → Validate → Code cycle on the newly-discovered codebase — the proposals become the backlog seed.

## What it does

1. **Collects** every finding artifact tagged with the session (via `discover:<session_id>`).
2. **Groups** them by phase and tier so the summary reflects source-priority ordering.
3. **Runs** `forgeplan health` on the workspace to surface blind spots, orphans, and stale evidence alongside the fresh findings.
4. **Synthesises** proposals by clustering related findings:
   - Multiple tier-1 findings about the same subsystem → proposed **RFC**.
   - Risk / instability / drift findings → proposed **PROB**.
   - User-facing goals inferred from tests + git intent → proposed **PRD**.
5. **Marks** the session as `completed` so it's excluded from further `discover_finding` calls.

## Why proposals are not auto-created

Auto-creating artifacts from discovery would flood the workspace with low-confidence stubs and violate the project rule "never leave PRD stubs." Printing proposals keeps a human (or the agent, with user consent) in the loop: only items that earn buy-in graduate into real artifacts through `forgeplan_new`.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `session_id` | `string` | yes | Session handle from `forgeplan_discover_start`. Must still be active. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DiscoverCompleteParams`_

## Returns

A summary report plus the proposal set:

```json
{
  "session_id": "discover-legacy-billing-service-…",
  "status": "completed",
  "findings_total": 27,
  "findings_by_phase": {
    "detect": 3, "structure": 4, "code": 9,
    "git": 4, "tests": 3, "docs": 4
  },
  "findings_by_tier": { "1": 12, "2": 5, "3": 4, "4": 6 },
  "health": { "blind_spots": 2, "orphans": 1, "stale_evidence": 0 },
  "proposed": [
    { "kind": "rfc",     "title": "Consolidate retry layers in billing engine",
      "rationale": "3 tier-1 findings describe overlapping exponential backoff." },
    { "kind": "problem", "title": "README drifted from src/auth — 4 claims unverified",
      "rationale": "Tier-4 vs tier-1 reconciliation mismatch." },
    { "kind": "prd",     "title": "Formalise idempotency guarantees on checkout",
      "rationale": "Tests imply exactly-once semantics not reflected in code or docs." }
  ],
  "next_steps": [
    "Review each proposal with the user.",
    "For each accepted proposal: forgeplan_new <kind> <title>.",
    "Start Shape → Validate cycle on the first PRD."
  ]
}
```

## Example invocation

```json
{ "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z" }
```

## Typical sequence

```
discover_start → …many discover_finding calls… → discover_complete
                                                    ↓ reviews proposals
                                                 forgeplan_new (one per accepted proposal)
                                                    ↓
                                                 forgeplan_validate / forgeplan_reason …
```

## CLI equivalent

- [`forgeplan discover complete`](/docs/cli/discover-complete/) — same finalisation from the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) — kick off a session
- [`forgeplan_discover_finding`](/docs/mcp/forgeplan_discover_finding/) — report observations
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) — the health snapshot bundled with the summary
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — promote a proposal into a real artifact
