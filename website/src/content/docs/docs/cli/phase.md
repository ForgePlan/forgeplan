---
title: forgeplan phase
description: "Read the advisory phase state for an artifact — current phase, workflow type, full transition history. Phase tracking is advisory and never blocks other tools."
---

`forgeplan phase` shows where an artifact stands in the methodology pipeline — Shape, Validate, Adi, Code, Test, Audit, Evidence, or Done — and prints the full history of how it got there (timestamps, reasons). The data lives in `.forgeplan/state/<id>.yaml` and is append-only: every transition is preserved.

The phase is **advisory**, meaning it is a hint for humans and agents, not a lock. No other Forgeplan command refuses to run because the phase looks "wrong". If an artifact has no state file yet (created before phase tracking existed, or with `phase.enabled: false` in config), the command prints `current_phase: unknown` with an empty history — that is normal, not an error.

This is the CLI version of [`forgeplan_phase`](/docs/mcp/forgeplan_phase/) on the MCP side.

## When to use

- Starting work on an artifact that is already in flight — "where did I leave off?"
- Before running an expensive tool, confirm the artifact is past the relevant phase (e.g. do not run `forgeplan score` on something still in `shape`).
- Reviewing an old artifact — read the transition history to understand the path it took.
- Audit or debugging — every transition has a timestamp and optional reason, so you can reconstruct decisions.

## When NOT to use

- As a hard gate to block work — phase is advisory. For structural blocking, use [`forgeplan validate`](/docs/cli/validate/).
- For lifecycle transitions (`draft` → `active` → `superseded`) — that is a separate state machine; see [`forgeplan activate`](/docs/cli/activate/), [`forgeplan supersede`](/docs/cli/supersede/), [`forgeplan deprecate`](/docs/cli/deprecate/).
- On Notes or trivial tactical fixes — phase tracking is not expected for one-off work.

## Usage

```text
forgeplan phase [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID whose phase state to read
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: Inspect an in-flight PRD

```bash
forgeplan phase PRD-057
```

Prints the current phase, workflow type, and the last three transitions in text mode
(full history in `--json`). Typical text output:

```text
PRD-057 — current_phase: code (greenfield)
  advanced_at: 2026-04-26T09:30:00Z
  history (last 3):
    shape    2026-04-25T14:00:00Z
    validate 2026-04-25T15:20:00Z
    code     2026-04-26T09:30:00Z  reason: FRs implemented
```

### Example 2: Full history as JSON

```bash
forgeplan phase PRD-057 --json | jq '.history'
```

Returns every transition the artifact has ever recorded. Useful for audits or for building a timeline view in a downstream tool.

### Example 3: Artifact without a state file

```bash
forgeplan phase PRD-001
```

If the artifact has no state file (created before phase tracking, or with tracking disabled), the output shows `current_phase: unknown` and an empty history. This is intentional — not an error. Start tracking with [`forgeplan phase-advance`](/docs/cli/phase-advance/).

## How it fits the workflow

Phase tracking is an observability layer over the methodology pipeline (Shape → Validate → Code → Evidence → Activate). Read with `phase`, write with [`forgeplan phase-advance`](/docs/cli/phase-advance/). Out-of-order jumps are allowed today (e.g. straight to Done for a typo fix); strict enforcement is planned for a later PRD under EPIC-005.

## See also

- [`forgeplan_phase`](/docs/mcp/forgeplan_phase/) — MCP equivalent
- [`forgeplan phase-advance`](/docs/cli/phase-advance/) — write the next transition
- [`forgeplan validate`](/docs/cli/validate/) — gate around the `validate` phase
- [`forgeplan activate`](/docs/cli/activate/) — the `done` terminal state of the methodology
- [Methodology guide](/docs/methodology/overview/) — Shape → Validate → Code → Evidence → Activate
