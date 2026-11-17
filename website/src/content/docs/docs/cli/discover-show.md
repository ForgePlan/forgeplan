---
title: forgeplan discover show
description: "Inspect an existing discovery session before resuming or completing it. The command reads saved session state, not a LanceDB table or a live feed from another agent."
---

Inspect an existing discovery session before resuming or completing it. The command reads saved session state, not a LanceDB table or a live feed from another agent.

## Usage

```text
forgeplan discover show <SESSION_ID>
```

## Arguments

```text
  <SESSION_ID>  Session ID (e.g. disc-20260407-abc)
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Example

```bash
forgeplan discover list
# Replace SESSION_ID with a listed identifier:
forgeplan discover show SESSION_ID
```

## Output

Shows session ID, project, status, start time, current phase, finding totals by phase and source tier, and up to ten recent findings. It does not report an owner field or infer that coverage is sufficient. Review whether the actual findings cover your intended scope before completing the session.

## See also

- [`forgeplan discover`](/docs/cli/discover/) - parent command
- [`forgeplan discover start`](/docs/cli/discover-start/) - begin a session
- [`forgeplan discover list`](/docs/cli/discover-list/) - all sessions
- [`forgeplan discover complete`](/docs/cli/discover-complete/) - save completion and print totals
