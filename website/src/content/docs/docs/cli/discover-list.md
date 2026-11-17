---
title: forgeplan discover list
description: "List saved discovery sessions to resume work, inspect prior surveys, or avoid creating a duplicate session."
---

List saved discovery sessions to resume work, inspect prior surveys, or avoid creating a duplicate session.

## Usage

```text
forgeplan discover list
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Example

```bash
forgeplan discover list
# Copy a session identifier from the table:
forgeplan discover show SESSION_ID
```

## Output

The text table shows Session ID, Project, Status, Phase, and Findings count. It does not have a created-at or source-tier coverage column, and this subcommand has no JSON flag. Use `show` for start time, tier counts, and recent findings. Copy real session IDs rather than assuming a disc-NNN sequence.

## See also

- [`forgeplan discover`](/docs/cli/discover/) - parent command
- [`forgeplan discover start`](/docs/cli/discover-start/) - create a new session
- [`forgeplan discover show`](/docs/cli/discover-show/) - inspect a specific session
- [`forgeplan discover complete`](/docs/cli/discover-complete/) - finalize a session
