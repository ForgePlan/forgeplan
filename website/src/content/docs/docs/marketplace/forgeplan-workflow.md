---
title: Forgeplan Workflow Plugin
description: /forge command — full methodology cycle for Claude Code
---

## What It Does

The **forgeplan-workflow** plugin adds the `/forge` command to Claude Code and compatible AI agents. It runs the full Forgeplan methodology cycle in a single conversational command:

```
/forge "Add payment processing"
```

This triggers: Route -> Shape -> Validate -> Code -> Evidence -> Activate.

## Installation

### Via marketplace (npx)

```bash
npx skills add ForgePlan/marketplace --skill forge
```

### Via built-in CLI (offline, no network)

If you already have the `forgeplan` binary installed:

```bash
forgeplan setup-skill
```

This writes the embedded skill file to `~/.claude/skills/forge/SKILL.md`. No network access, no marketplace fetching -- the skill definition is compiled into the binary. See [`forgeplan setup-skill`](/docs/cli/setup-skill/).

## Commands

| Command | Description |
|---------|-------------|
| `/forge "task"` | Full cycle: route -> create -> validate -> build -> evidence -> activate |
| `/forge-cycle` | Explicit step-by-step forge cycle with 8 phases |
| `/forge-audit` | Multi-expert code audit with methodology integration |

## How /forge Works

When you invoke `/forge "Add rate limiting"`, the skill:

1. **Route** -- calls `forgeplan_route` to determine depth (Tactical / Standard / Deep / Critical)
2. **Shape** -- creates the right artifact (PRD, RFC, etc.) via `forgeplan_new`
3. **Validate** -- checks quality gates via `forgeplan_validate`
4. **Reason** -- runs ADI reasoning if Standard+ depth (3+ hypotheses)
5. **Code** -- builds the solution with tests
6. **Evidence** -- creates evidence pack, links to artifact, checks R_eff
7. **Activate** -- marks artifact as active via `forgeplan_activate`

For Tactical depth, the skill skips artifacts and just executes the task directly.

## /forge-cycle -- Explicit Step-by-Step

When you need more control over each phase:

```
/forge-cycle PRD-001
```

Runs 8 explicit phases:

| Phase | Action |
|-------|--------|
| 0. OBSERVE | `forgeplan health` -- understand project state |
| 1. ROUTE | Determine depth and pipeline |
| 2. SPRINT | Plan implementation waves |
| 3. BUILD | Implement the solution |
| 4. AUDIT | Adversarial multi-expert review |
| 5. FIXES | Fix HIGH/CRITICAL findings |
| 6. EVIDENCE | Create evidence, link, score |
| 7. COMMIT | Git commit + PR |

## /forge-audit -- Methodology-Aware Audit

Combines code auditing with Forgeplan's quality framework. Reports findings against both code quality and methodology compliance (missing evidence, unlinked artifacts, R_eff gaps).

## Agentic RAG Architecture

The skill uses **agentic RAG** -- it loads only the relevant portion of the methodology knowledge base for each step (~300 lines), not the entire specification. The `SKILL.md` file acts as a router:

- Maps user needs to specific methodology sections
- Provides MCP tool reference (which `forgeplan_*` tool to call)
- Includes depth calibration rules and escalation triggers
- Documents the full artifact lifecycle

### Knowledge base sections included

| Section | Content |
|---------|---------|
| MCP tool table | All `forgeplan_*` tools with CLI equivalents |
| Core workflow | 6-step cycle: health -> route -> new -> validate -> review -> activate |
| Depth calibration | Tactical / Standard / Deep / Critical decision matrix |
| Evidence rules | Structured fields, CL scoring, R_eff calculation |
| Proactive behavior | When to escalate, when to suggest artifacts |
| Lifecycle states | Draft -> active -> superseded/deprecated flow |

## Worked Example

```
User: /forge "add rate limiting to the API"

Agent: [calls forgeplan_route("add rate limiting to the API")]
  -> Depth: Standard, Pipeline: PRD -> RFC

Agent: [calls forgeplan_new(kind: "prd", title: "API Rate Limiting")]
  -> Created PRD-042

Agent: [fills Problem, Goals, Non-Goals, FR sections]

Agent: [calls forgeplan_validate("PRD-042")]
  -> PASS (0 MUST errors)

Agent: [implements rate limiting middleware + tests]

Agent: [calls forgeplan_new(kind: "evidence", title: "Rate limit tests -- 8 pass")]
  -> Created EVID-089

Agent: [calls forgeplan_link("EVID-089", "PRD-042", "informs")]
Agent: [calls forgeplan_activate("PRD-042")]
  -> draft -> active, R_eff = 1.00
```

## See Also

- [Methodology Overview](/docs/methodology/overview/) -- the 10 rules `/forge` enforces
- [Quick Start](/docs/getting-started/quick-start/) -- manual walkthrough of the same cycle
- [`forgeplan setup-skill`](/docs/cli/setup-skill/) -- offline installation
- [Dev Toolkit](/docs/marketplace/dev-toolkit/) -- complementary `/audit`, `/sprint` commands
- [Commands Reference](/docs/marketplace/commands/) -- all slash commands
- [Marketplace Overview](/docs/marketplace/overview/) -- full plugin catalog
