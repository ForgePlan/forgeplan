---
title: forgeplan discover start
description: "Start a new brownfield discovery session and print the protocol for the AI agent to follow"
---

`forgeplan discover start` creates a new **discovery session** in the workspace and prints the protocol an AI agent should follow to map an existing codebase into Forgeplan artifacts. The session acts as a container for findings the agent appends (via MCP) as it reads the code, git history, tests, and docs.

No scanning happens inside this command itself — it sets up state and instructions; the actual walk is done by an agent that understands the printed protocol.

## When to use

- **Once per brownfield onboarding** — right after `forgeplan init -y` on an existing repo.
- **After a long uninstrumented period** — kick off a refresh session to catch up.
- **When you want an AI teammate to map a new subsystem** you haven't documented yet.

## When NOT to use

- For routine greenfield work — use `forgeplan route` + `forgeplan new`.
- When no agent is going to follow the protocol — the session will sit empty.
- For tiny single-file tasks — overhead isn't worth it; just create a Note.

## Usage

```text
forgeplan discover start <NAME>
```

## Arguments

```text
  <NAME>  Project name for the discovery session
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
# Start a named session in the current workspace
forgeplan discover start "Auth System Onboarding"

# Typical brownfield flow
forgeplan init -y
forgeplan discover start "Legacy API Mapping"
# agent now executes the printed protocol, calling discover_finding via MCP
forgeplan discover show <session-id>
forgeplan discover complete <session-id>
```

## What the printed protocol tells the agent

1. **Tiered reading order** — code > git > tests > docs. Lower tiers only validate higher tiers, never override them.
2. **Finding categories** — decisions, invariants, drift, debt, risks, ownership signals.
3. **How to report** — append each finding via the `discover_finding` MCP tool (MCP-only, by design — keeps humans out of the loop while the survey runs).
4. **When to stop** — heuristics for coverage (e.g. all top-level crates/modules touched, major git epochs sampled).
5. **Handoff** — when done, call `discover complete` to trigger the recommendation pass.

## How it fits

`discover start` is the **entry point** of the brownfield pipeline:

```
discover start  →  agent walks sources  →  discover_finding (MCP) ×N
                     ↓
                  discover show (inspect)
                     ↓
                  discover complete  →  artifact proposals  →  forgeplan new ...
```

It was introduced in PRD-035 Sprint 13.3-13.4 to address the onboarding gap documented in PROB-022.

## See also

- [`forgeplan discover`](/docs/cli/discover/) — parent command and protocol overview
- [`forgeplan discover show`](/docs/cli/discover-show/) — inspect a running session
- [`forgeplan discover list`](/docs/cli/discover-list/) — all sessions
- [`forgeplan discover complete`](/docs/cli/discover-complete/) — finalize + recommendations
- [`forgeplan init`](/docs/cli/init/) — workspace bootstrap step before discover
