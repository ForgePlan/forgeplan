---
title: forgeplan discover list
description: "List all brownfield discovery sessions in the workspace"
---

`forgeplan discover list` prints stored discovery sessions with Session ID, Project, Status, Phase, and Findings count. It reads session files without opening the artifact index.

## When to use

- **To pick up a stalled session** - find the ID to pass to `discover show` or `discover complete`.
- **To audit discovery history** - see how many onboarding / refresh passes the project has had.
- **Before starting a new session** - avoid duplicating an active one.
- **In CI or scripting** - enumerate sessions for automation.

## When NOT to use

- To inspect a single session's findings - use [`discover show`](/docs/cli/discover-show/).
- To start a new session - use [`discover start`](/docs/cli/discover-start/).

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

The columns are:

- **Session ID** - copy the actual returned ID; `disc-002` above is illustrative.
- **Project** - project name, possibly truncated.
- **Status** - stored session status.
- **Phase** - current discovery phase, not a coverage summary.
- **Findings** - number of stored findings.

## How it fits

`discover list` is the enumeration primitive for the discovery subsystem. Everything else (`show`, `complete`) operates on a specific session ID that you typically pick from this listing.

```
discover list → pick an ID → discover show / complete
```

## See also

- [`forgeplan discover`](/docs/cli/discover/) - parent command
- [`forgeplan discover start`](/docs/cli/discover-start/) - create a new session
- [`forgeplan discover show`](/docs/cli/discover-show/) - inspect a specific session
- [`forgeplan discover complete`](/docs/cli/discover-complete/) - finalize a session

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan discover list --help`.

```text
List all discovery sessions in the workspace

Usage: forgeplan discover list

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
