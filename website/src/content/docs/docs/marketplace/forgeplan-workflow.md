---
title: Forgeplan Workflow Plugin
description: Client workflow templates, methodology skills and the separate bundled /forge skill
---

## What It Does

The **forgeplan-workflow** plugin provides `/forge-cycle`, `/forge-audit`
and the `forgeplan-methodology` skill. The separate bundled `/forge` skill
installed by `forgeplan setup-skill` can guide the same methodology cycle:

```
/forge "Add payment processing"
```

These are agent instructions for Route -> Shape -> Validate -> Code ->
Evidence -> Activate, not a binary command that automatically executes
every stage or installs every client integration.

## Installation

### Via the client's plugin marketplace

Run these slash commands inside the client, not through `npx` or a shell:

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install forgeplan-workflow@ForgePlan-marketplace
```

### Bundled /forge skill via CLI (offline, no network)

If you already have the `forgeplan` binary installed:

```bash
forgeplan setup-skill
```

This writes the embedded skill file to `~/.claude/skills/forge/SKILL.md`. No network access, no marketplace fetching -- the skill definition is compiled into the binary. See [`forgeplan setup-skill`](/docs/cli/setup-skill/).

## Commands

| Command | Description |
|---------|-------------|
| `/forge "task"` | Full cycle: route -> create -> validate -> build -> evidence -> activate |
| `/forge-cycle` | Orchestrator template for the full cycle, with quality gates |
| `/forge-audit` | Multi-expert code audit with methodology integration |

## How /forge Works

When you invoke `/forge "Add rate limiting"`, the skill:

1. **Route** -- calls `forgeplan_route` to determine depth (Tactical / Standard / Deep / Critical)
2. **Shape** -- creates the right artifact (PRD, RFC, etc.) via `forgeplan_new`
3. **Validate** -- checks quality gates via `forgeplan_validate`
4. **Reason** -- requests ADI alternatives (recommended at Standard, required at Deep/Critical; stricter project workflow still applies)
5. **Code** -- builds the solution with tests
6. **Evidence** -- creates evidence pack, links to artifact, checks R_eff
7. **Activate** -- marks artifact as active via `forgeplan_activate`

For Tactical depth, the skill skips artifacts and just executes the task directly.

## /forge-cycle -- Explicit Step-by-Step

When you need more control over each phase:

```
/forge-cycle PRD-001
```

The eight rows below are a compact workflow map, not the current template's
complete numbered steps. The orchestrator performs equivalent operations;
markers such as `/sprint` and `/audit` are not automatically invoked inline:

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

The methodology skill uses **agentic RAG**: a `SKILL.md` router points the
agent to relevant sections rather than requiring the whole specification
at once. The amount read depends on the task and host, not a guaranteed
300-line runtime limit:

- Maps user needs to specific methodology sections
- Provides MCP tool reference (which `forgeplan_*` tool to call)
- Includes depth calibration rules and escalation triggers
- Documents the full artifact lifecycle

### Knowledge base sections included

| Section | Content |
|---------|---------|
| MCP tool table | Selected `forgeplan_*` tools with CLI equivalents; not the full versioned catalog |
| Core workflow | 6-step cycle: health -> route -> new -> validate -> review -> activate |
| Depth calibration | Tactical / Standard / Deep / Critical decision matrix |
| Evidence rules | Structured fields, CL scoring, R_eff calculation |
| Proactive behavior | When to escalate, when to suggest artifacts |
| Lifecycle states | Draft -> active -> superseded/deprecated flow |

## Worked Example

Illustrative workflow, not a recorded MCP response transcript:

```
User: /forge "add rate limiting to the API"

Agent: [calls forgeplan_route("add rate limiting to the API")]
  -> Depth: Standard, Pipeline: PRD -> RFC

Agent: [calls forgeplan_new(kind: "prd", title: "API Rate Limiting")]
  -> Created PRD-042

Agent: [fills all required sections, creates linked RFC, validates and reasons at selected depth]

Agent: [calls forgeplan_validate("PRD-042")]
  -> PASS (0 MUST errors)

Agent: [implements rate limiting middleware + tests]

Agent: [calls forgeplan_new(kind: "evidence", title: "Rate limit tests -- 8 pass")]
  -> Created EVID-089

Agent: [fills evidence body with actual results, structured fields and required code provenance]

Agent: [calls forgeplan_link("EVID-089", "PRD-042", "informs")]
Agent: [calls forgeplan_score("PRD-042"), inspects recursive score and factors]
Agent: [calls forgeplan_activate("PRD-042")]
  -> draft -> active (activation response is not a measured trust guarantee)
```

## See Also

- [Methodology Overview](/docs/methodology/overview/) -- the 10 rules `/forge` enforces
- [Quick Start](/docs/getting-started/quick-start/) -- manual walkthrough of the same cycle
- [`forgeplan setup-skill`](/docs/cli/setup-skill/) -- offline installation
- [Dev Toolkit](/docs/marketplace/dev-toolkit/) -- historical package; current catalog directs users to `fpl-skills`
- [Commands Reference](/docs/marketplace/commands/) -- all slash commands
- [Marketplace Overview](/docs/marketplace/overview/) -- full plugin catalog
