---
title: forgeplan discover
description: "Brownfield discovery engine - start a protocol-driven session that turns an existing codebase into Forgeplan artifacts"
---

`forgeplan discover` manages brownfield discovery sessions (PRD-035, NOTE-041, PROB-022). `start` saves a session and prints a seven-phase protocol for an AI agent to execute. The agent reads code, history, tests and docs, and reports findings through MCP; the CLI does not inspect the repository on the agent's behalf.

The findings can inform Problems, PRDs, ADRs and Notes. CLI `complete` only marks the session completed and suggests a health check; it does not generate those artifacts or print a proposal list.

## When to use

- **Onboarding Forgeplan into an existing repo** — run `discover start <project-name>` after `forgeplan init -y`.
- **After a long uninstrumented sprint** - catch up on undocumented decisions before they drift out of memory.
- **When joining a new team's project** - let the agent build a structured map of what exists, then use it to plan.
- **Before an audit or refactor** - seed artifacts so the refactor has a baseline to supersede.

## When NOT to use

- For ongoing, greenfield work - prefer `forgeplan route` + `forgeplan new` directly.
- For a single task - discover sessions produce broad surveys, not task-level plans.
- Without an agent or person to execute the protocol — `discover start <project-name>` prints instructions rather than performing the analysis.

## Usage

```text
forgeplan discover <COMMAND>
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Subcommands

```text
  start     Start a new discovery session - prints protocol for AI agent
  list      List all discovery sessions in the workspace
  show      Show status of a discovery session
  complete  Mark a discovery session as completed
  help      Print this message or the help of the given subcommand(s)
```

## Examples

```bash
# Typical brownfield onboarding flow
forgeplan init -y
forgeplan discover start "Example project"
# → agent reads code/git/tests/docs, appends findings via MCP
forgeplan discover show <session-id>
forgeplan discover complete <session-id>
# → session completed; review findings and decide what artifacts are needed
forgeplan discover list
```

## The protocol in one paragraph

`discover start <project-name>` creates a session file and prints a protocol. The agent follows its phases and uses `forgeplan_discover_finding` through MCP; there is no CLI finding subcommand. `discover show` displays findings. CLI `discover complete` saves completion status and prints a health-check hint, not a recommendation-generation pass.

## Tiered source priority (important)

The protocol prioritizes executable sources over documentation for behavioral claims. Code can also contain defects; distinguish observed behavior from intended requirements:

1. **Code** - actual behavior, current invariants.
2. **Git history** - who changed what, when, and why (commit messages, PR refs).
3. **Tests** - declared behavior and edge cases the team cared about.
4. **Docs** - aspirational or stale; use only to cross-check 1-3, never as primary.

Findings from lower-priority tiers that contradict higher tiers are flagged as **drift**, often becoming a Problem card in the recommendations.

## How it fits

Discover closes the gap identified in PROB-022 (brownfield onboarding gap). It plugs into:

- **`forgeplan new`** — create the artifacts you decide are justified by findings.
- **`forgeplan health`** — inspect the result; discovery does not guarantee fewer blind spots or orphans.
- **`forgeplan fpf check`** - once artifacts exist, FPF rules can evaluate them.

## See also

- [`forgeplan discover start`](/docs/cli/discover-start/) - start a session + print protocol
- [`forgeplan discover list`](/docs/cli/discover-list/) - all sessions in the workspace
- [`forgeplan discover show`](/docs/cli/discover-show/) - inspect a session's findings
- [`forgeplan discover complete`](/docs/cli/discover-complete/) - mark the session completed
- [Methodology guide](/docs/methodology/overview/)
- [`forgeplan health`](/docs/cli/health/) - before/after comparison

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan discover --help`.

```text
Start brownfield discovery — creates session, prints protocol for agent

Usage: forgeplan discover <COMMAND>

Commands:
  start     Start a new discovery session — prints protocol for AI agent
  list      List all discovery sessions in the workspace
  show      Show status of a discovery session
  complete  Mark a discovery session as completed
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

- [`forgeplan discover start`](/docs/cli/discover-start/)
- [`forgeplan discover list`](/docs/cli/discover-list/)
- [`forgeplan discover show`](/docs/cli/discover-show/)
- [`forgeplan discover complete`](/docs/cli/discover-complete/)
<!-- forgeplan-runtime:end -->
