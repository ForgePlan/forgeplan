---
title: forgeplan discover
description: "Brownfield discovery engine — start a protocol-driven session that turns an existing codebase into Forgeplan artifacts"
---

`forgeplan discover` is the parent command for Forgeplan's **brownfield discovery engine** (PRD-035, NOTE-041, PROB-022). It orchestrates protocol-driven sessions where an AI agent reads an existing codebase, git history, tests, and docs — in that priority order — and reports findings back through MCP. At the end, Forgeplan proposes concrete artifacts (Problems, PRDs, ADRs, Notes) to create so the project can start benefiting from the methodology without hand-authoring a backlog.

In short: discover takes a project that was built before Forgeplan and produces a first draft of artifacts for it.

## When to use

- **Onboarding Forgeplan into an existing repo** — run `discover start` as one of the first steps after `forgeplan init -y`.
- **After a long uninstrumented sprint** — catch up on undocumented decisions before they drift out of memory.
- **When joining a new team's project** — let the agent build a structured map of what exists, then use it to plan.
- **Before an audit or refactor** — seed artifacts so the refactor has a baseline to supersede.

## When NOT to use

- For ongoing, greenfield work — prefer `forgeplan route` + `forgeplan new` directly.
- For a single task — discover sessions produce broad surveys, not task-level plans.
- Without an AI agent that can follow the MCP protocol — `discover start` prints instructions, but somebody has to execute them.

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
  start     Start a new discovery session — prints protocol for AI agent
  list      List all discovery sessions in the workspace
  show      Show status of a discovery session
  complete  Mark a discovery session as completed
  help      Print this message or the help of the given subcommand(s)
```

## Examples

```bash
# Typical brownfield onboarding flow
forgeplan init -y
forgeplan discover start
# → agent reads code/git/tests/docs, appends findings via MCP
forgeplan discover show <session-id>
forgeplan discover complete <session-id>
# → Forgeplan proposes PROBs / PRDs / Notes to create
forgeplan discover list
```

## The protocol in one paragraph

`discover start` creates a session row and prints a **protocol** for the agent to follow. The agent walks tiered sources — **code > git history > tests > docs** — and for each significant finding calls the `discover_finding` MCP tool (MCP-only; there's no CLI equivalent, to keep the loop agent-driven). `discover show` displays accumulated findings; `discover complete` finalizes the session and emits recommendations for `forgeplan new` commands.

## Tiered source priority (important)

Code is the **ground truth**; everything else can lie. The protocol enforces this order:

1. **Code** — actual behavior, current invariants.
2. **Git history** — who changed what, when, and why (commit messages, PR refs).
3. **Tests** — declared behavior and edge cases the team cared about.
4. **Docs** — aspirational or stale; use only to cross-check 1-3, never as primary.

Findings from lower-priority tiers that contradict higher tiers are flagged as **drift**, often becoming a Problem card in the recommendations.

## How it fits

Discover closes the gap identified in PROB-022 (brownfield onboarding gap). It plugs into:

- **`forgeplan new`** — the recommendations at `discover complete` map 1:1 to artifact creation calls.
- **`forgeplan health`** — post-discover health should show fewer orphans and blind spots.
- **`forgeplan fpf check`** — once artifacts exist, FPF rules can evaluate them.

## See also

- [`forgeplan discover start`](/docs/cli/discover-start/) — start a session + print protocol
- [`forgeplan discover list`](/docs/cli/discover-list/) — all sessions in the workspace
- [`forgeplan discover show`](/docs/cli/discover-show/) — inspect a session's findings
- [`forgeplan discover complete`](/docs/cli/discover-complete/) — finalize + emit recommendations
- [Methodology guide](/docs/methodology/overview/)
- [`forgeplan health`](/docs/cli/health/) — before/after comparison
