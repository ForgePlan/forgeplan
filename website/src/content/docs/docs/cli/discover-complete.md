---
title: forgeplan discover complete
description: "Mark an existing discovery session completed and save its completion time. CLI 0.38.0 prints finding totals and a hint to run health. It does not run health itself or generate artifact proposals."
---

Mark an existing discovery session completed and save its completion time. CLI 0.38.0 prints finding totals and a hint to run health. It does not run health itself or generate artifact proposals.

## Usage

```text
forgeplan discover complete <SESSION_ID>
```

## Arguments

```text
  <SESSION_ID>  Session ID to complete
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Example

```bash
forgeplan discover list
# Replace SESSION_ID with the actual returned identifier:
forgeplan discover show SESSION_ID
forgeplan discover complete SESSION_ID
forgeplan health
```

## Hand-off from discovery

Review accumulated findings, decide which need a Problem, PRD, ADR, or Note, and create those explicitly with `new`. Completion does not deduplicate findings, choose depth, or create a backlog. Repeating completion on an already completed session returns successfully.

## See also

- [`forgeplan discover`](/docs/cli/discover/) - parent command + pipeline overview
- [`forgeplan discover show`](/docs/cli/discover-show/) - coverage check before completing
- [`forgeplan discover list`](/docs/cli/discover-list/) - session index
- [`forgeplan new`](/docs/cli/new/) - create the proposed artifacts
- [`forgeplan validate`](/docs/cli/validate/) - next step after new
- [Methodology guide](/docs/methodology/overview/)
