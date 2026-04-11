---
title: forgeplan_discover_start
description: "Start a brownfield discovery session. Returns a structured protocol (7 phases: detect/structure/code/git/tests/docs/synthesize) that the AI agent follows to map an existing codebase."
---

Starts a new **brownfield discovery session** — a structured protocol the agent follows when it's dropped into an existing codebase with no prior ForgePlan artifacts. Instead of the agent inventing its own exploration order, ForgePlan returns the seven-phase protocol (detect / structure / code / git / tests / docs / synthesize) with explicit source-tier priorities, and the agent then walks through it, calling `forgeplan_discover_finding` for each observation. On `forgeplan_discover_complete`, the collected findings are synthesised into proposed PROBs / PRDs / RFCs (printed, not auto-created).

**Category**: Brownfield Discovery

## When an agent calls this

- First session on an unfamiliar repository — "what is this project and what state is it in?"
- Onboarding a legacy codebase into the Forgeplan methodology without fabricating artifacts from thin air.
- After a large merge or acquisition, when the agent needs to re-map the codebase before proposing new decisions.
- Before a migration PRD — discovery makes the "as-is" state explicit so the migration goals are grounded.

The discover protocol is intentionally **agent-driven**: ForgePlan never reads source files itself. It tells the agent *what* to look at and in *which tier order*, then accepts the findings.

## Source tier priority

The protocol enforces a strict priority order so findings are grounded in authoritative evidence, not documentation drift:

| Tier | Source | Why |
|------|--------|-----|
| **1** | Source code (`src/`, `lib/`, `app/`) | Ground truth — the code is what actually runs. |
| **2** | Git history (`git log`, commits, tags, release notes) | Intent over time — reveals decisions, reverts, and trajectory. |
| **3** | Tests (`tests/`, `spec/`) | Behavioural contracts — pins intended semantics. |
| **4** | Docs (`README`, `docs/`, wikis) | Last because they are frequently stale vs. code. |

Findings from a higher tier override findings from a lower tier when they conflict.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `project_name` | `string` | yes | Human-readable name for the discovery session (e.g. `"legacy-billing-service"`). Used as the session identifier prefix and in the final summary. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DiscoverStartParams`_

## Returns

A session handle plus the full protocol payload:

```json
{
  "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z",
  "project_name": "legacy-billing-service",
  "phases": [
    { "name": "detect",     "goal": "Detect tech stack, build system, languages", "tier_order": [1, 4] },
    { "name": "structure",  "goal": "Map top-level modules and boundaries",        "tier_order": [1] },
    { "name": "code",       "goal": "Read entry points and critical modules",      "tier_order": [1, 3] },
    { "name": "git",        "goal": "Scan commit history for decisions / reverts", "tier_order": [2] },
    { "name": "tests",      "goal": "Identify behavioural contracts and gaps",     "tier_order": [3, 1] },
    { "name": "docs",       "goal": "Reconcile docs vs. code (flag drift)",        "tier_order": [4, 1] },
    { "name": "synthesize", "goal": "Propose PROBs / PRDs / RFCs from findings",   "tier_order": [] }
  ],
  "instructions": "For each phase, walk files in the listed tier order, then call forgeplan_discover_finding for each observation. Close with forgeplan_discover_complete."
}
```

## Example invocation

```json
{ "project_name": "legacy-billing-service" }
```

## Typical sequence

```
forgeplan_discover_start
    ↓ returns session_id + 7-phase protocol
forgeplan_discover_finding (tier=1, phase=detect,    …)
forgeplan_discover_finding (tier=1, phase=structure, …)
forgeplan_discover_finding (tier=1, phase=code,      …)
forgeplan_discover_finding (tier=2, phase=git,       …)
forgeplan_discover_finding (tier=3, phase=tests,     …)
forgeplan_discover_finding (tier=4, phase=docs,      …)
forgeplan_discover_complete (session_id)
    ↓ prints proposed PROBs / PRDs / RFCs
```

The agent is expected to iterate per phase — it can emit many findings per phase before advancing.

## CLI equivalent

- [`forgeplan discover start`](/docs/cli/discover-start/) — same flow from the terminal, for a human operator.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_discover_finding`](/docs/mcp/forgeplan_discover_finding/) — append a single observation
- [`forgeplan_discover_complete`](/docs/mcp/forgeplan_discover_complete/) — finalise + synthesise artifacts
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) — post-discovery project state
