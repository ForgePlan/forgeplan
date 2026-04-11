---
title: forgeplan discover show
description: "Show the status and accumulated findings of a discovery session"
---

`forgeplan discover show <SESSION_ID>` prints the current state of a brownfield discovery session — the protocol stage it's in, how many findings the agent has submitted, and a categorized summary of what's been captured so far.

Use it during a run to watch progress, and before `discover complete` to sanity-check coverage.

## When to use

- **Mid-run** — the agent has been appending findings and you want to see the picture so far.
- **Before `discover complete`** — verify the session actually covered the areas you care about.
- **While debugging** — confirm that agent findings are landing in the right session (useful when multiple sessions run in parallel).
- **When resuming** a stalled session after a break.

## When NOT to use

- For a list of all sessions — use [`discover list`](/docs/cli/discover-list/).
- To start a new one — use [`discover start`](/docs/cli/discover-start/).

## Usage

```text
forgeplan discover show <SESSION_ID> [OPTIONS]
```

## Arguments

```text
  <SESSION_ID>   Discovery session identifier
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
forgeplan discover show disc-001
forgeplan discover show disc-001 | less

# Typical mid-run check
forgeplan discover start
# ...agent runs for a while...
forgeplan discover show disc-001
```

## What you see

Typical output blocks:

- **Session header** — ID, created_at, status (active / completed), owner/agent.
- **Coverage** — which source tiers have been touched (code / git / tests / docs), with counts.
- **Findings by category** — decisions, invariants, drift, debt, risks.
- **Recent findings** — latest N entries with summary text.
- **Next action hint** — either "keep scanning" or "ready to complete".

## How it fits

`discover show` is the **read** side of the discovery loop. It reads directly from the session store (LanceDB), so it always reflects the latest `discover_finding` MCP calls without needing a refresh.

```
discover start → [agent appends via MCP] → discover show → discover complete
```

## See also

- [`forgeplan discover`](/docs/cli/discover/) — parent command
- [`forgeplan discover start`](/docs/cli/discover-start/) — begin a session
- [`forgeplan discover list`](/docs/cli/discover-list/) — all sessions
- [`forgeplan discover complete`](/docs/cli/discover-complete/) — finalize + recommendations
