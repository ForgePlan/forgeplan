---
title: forgeplan discover list
description: "List all brownfield discovery sessions in the workspace"
---

`forgeplan discover list` prints every discovery session stored in the workspace — active and completed — with ID, status, creation time, and a short coverage summary. It's the index view across sessions.

## When to use

- **To pick up a stalled session** — find the ID to pass to `discover show` or `discover complete`.
- **To audit discovery history** — see how many onboarding / refresh passes the project has had.
- **Before starting a new session** — avoid duplicating an active one.
- **In CI or scripting** — enumerate sessions for automation.

## When NOT to use

- To inspect a single session's findings — use [`discover show`](/docs/cli/discover-show/).
- To start a new session — use [`discover start`](/docs/cli/discover-start/).

## Usage

```text
forgeplan discover list [OPTIONS]
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
# All sessions
forgeplan discover list

# Typical "where was I" recovery flow
forgeplan discover list
forgeplan discover show disc-002
forgeplan discover complete disc-002
```

## What you see

A table (or list) with one row per session, typically showing:

- **Session ID** (`disc-NNN`)
- **Status** — active / completed
- **Created at** — ISO timestamp
- **Findings count** — how many `discover_finding` calls hit this session
- **Coverage** — short tier summary (which of code/git/tests/docs were touched)

## How it fits

`discover list` is the enumeration primitive for the discovery subsystem. Everything else (`show`, `complete`) operates on a specific session ID that you typically pick from this listing.

```
discover list → pick an ID → discover show / complete
```

## See also

- [`forgeplan discover`](/docs/cli/discover/) — parent command
- [`forgeplan discover start`](/docs/cli/discover-start/) — create a new session
- [`forgeplan discover show`](/docs/cli/discover-show/) — inspect a specific session
- [`forgeplan discover complete`](/docs/cli/discover-complete/) — finalize a session
