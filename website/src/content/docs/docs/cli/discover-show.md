---
title: forgeplan discover show
description: "Show the status and accumulated findings of a discovery session"
---

`forgeplan discover show <SESSION_ID>` prints the current state of a brownfield discovery session - the protocol stage it's in, how many findings the agent has submitted, and a categorized summary of what's been captured so far.

Use it during a run to watch progress, and before `discover complete` to sanity-check coverage.

## When to use

- **Mid-run** - the agent has been appending findings and you want to see the picture so far.
- **Before `discover complete`** - verify the session actually covered the areas you care about.
- **While debugging** - confirm that agent findings are landing in the right session (useful when multiple sessions run in parallel).
- **When resuming** a stalled session after a break.

## When NOT to use

- For a list of all sessions - use [`discover list`](/docs/cli/discover-list/).
- To start a new one - use [`discover start`](/docs/cli/discover-start/).

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
forgeplan discover start "Project discovery"
# ...agent runs for a while...
forgeplan discover show disc-001
```

## What you see

The report shows:

- **Session header** - ID, project name, status, current phase, start/completion time.
- **Findings by phase and tier** - counts from recorded findings.
- **Recent findings** - the last 10, including phase, tier, artifact reference and title.
- **Total findings** and a next-action hint. The hint is not a coverage certificate.

## How it fits

`discover show` reads `.forgeplan/discover/<SESSION_ID>.json`, not LanceDB.
It displays recorded session state; it does not independently verify survey coverage.

```
discover start → [agent appends via MCP] → discover show → discover complete
```

## See also

- [`forgeplan discover`](/docs/cli/discover/) - parent command
- [`forgeplan discover start`](/docs/cli/discover-start/) - begin a session
- [`forgeplan discover list`](/docs/cli/discover-list/) - all sessions
- [`forgeplan discover complete`](/docs/cli/discover-complete/) - finalize + recommendations

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan discover show --help`.

```text
Show status of a discovery session

Usage: forgeplan discover show <SESSION_ID>

Arguments:
  <SESSION_ID>  Session ID (e.g. disc-20260407-abc)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
