---
title: forgeplan blocked
description: "Report unresolved structural dependencies for one artifact or the whole graph. Structural relations include based_on, refines, and supersedes; informs is not a scheduling dependency. Active, deprecated, and superseded dependencies are treated as resolved."
---

Report unresolved structural dependencies for one artifact or the whole graph. Structural relations include `based_on`, `refines`, and `supersedes`; `informs` is not a scheduling dependency. Active, deprecated, and superseded dependencies are treated as resolved.

## Usage

```text
forgeplan blocked [OPTIONS] [ID]
```

## Arguments

```text
  [ID]  Specific artifact ID to check (optional)
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
forgeplan blocked
forgeplan blocked PRD-046
forgeplan blocked --json | jq '.blocked[] | .id'
forgeplan blocked PRD-046 --json | jq '.blocked, .blocked_by'
```

## Output and automation

Workspace JSON includes `blocked` entries with `id` and `blocked_by`, plus `ready`, `ready_count`, `blocked_count`, `cycles`, `_next_action`, and `hints`. Single-artifact JSON has `id`, `blocked`, `blocked_by`, `_next_action`, and `hints`. Finding blockers does not make this command exit 1. Inspect JSON explicitly when building a dependency gate. For health thresholds use text-mode `health --ci`; in 0.38.0, `health --json --ci` returns before applying CI thresholds (#545).

## Planning

Pair with `order` for execution sequence and `graph` for all relation types. A missing dependency or cycle requires reviewing the actual edge, not blindly activating the blocker.

## See also

- [`forgeplan order`](/docs/cli/order/) - topological sequence
- [`forgeplan graph`](/docs/cli/graph/) - full dependency graph
- [`forgeplan blindspots`](/docs/cli/blindspots/) - decisions without evidence
- [`forgeplan health`](/docs/cli/health/) - project rollup
