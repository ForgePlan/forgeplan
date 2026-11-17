---
title: forgeplan phase
description: "Inspect advisory artifact phase and retained transition history. Corrupt YAML triggers a quarantine rename attempt before an unknown-state response."
---

`forgeplan phase` inspects an artifact's advisory methodology phase - Shape, Validate, Adi, Code, Test, Audit, Evidence, or Done. State lives in `.forgeplan/state/<id>.yaml`. JSON includes the retained transition history; text shows only the last three entries. Inspection normally reads state, but corrupt-YAML handling can rename the state file as described below.

The phase is **advisory**, meaning it is a hint for humans and agents, not a lock. No other Forgeplan command refuses to run because the phase looks "wrong". If an artifact has no state file yet (created before phase tracking existed, or with `phase.enabled: false` in config), the command prints `current_phase: unknown` with an empty history - that is normal, not an error.

This is the CLI version of [`forgeplan_phase`](/docs/mcp/forgeplan_phase/) on the MCP side.

## When to use

- Starting work on an artifact that is already in flight - "where did I leave off?"
- Before work, compare the recorded phase with the work actually completed. A phase marker does not prove validation, testing, or scoring ran.
- Reviewing an old artifact - read the transition history to understand the path it took.
- Audit or debugging - every transition has a timestamp and optional reason, so you can reconstruct decisions.

## When NOT to use

- As a hard gate to block work - phase is advisory. For structural blocking, use [`forgeplan validate`](/docs/cli/validate/).
- For lifecycle transitions (`draft` → `active` → `superseded`) - that is a separate state machine; see [`forgeplan activate`](/docs/cli/activate/), [`forgeplan supersede`](/docs/cli/supersede/), [`forgeplan deprecate`](/docs/cli/deprecate/).
- On Notes or trivial tactical fixes - phase tracking is not expected for one-off work.

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
(full history in `--json`). Illustrative text output for three recorded transitions:

```text
PRD-057 phase: code (workflow: greenfield)
  last advanced: 2026-04-26 09:30:00 UTC
  history (showing last 3 of 3):
    2026-04-25 14:00 - -> shape
    2026-04-25 15:20 shape -> validate
    2026-04-26 09:30 validate -> code -- ready for implementation
  suggested next: test (override: `forgeplan phase-advance PRD-057 --to <phase>`)

Next: forgeplan phase-advance PRD-057 --to test
```

### Example 2: full history as JSON

```bash
forgeplan phase PRD-057 --json | jq '.history'
```

Returns all transitions retained in the state file. The writer caps history at 1024 entries, preserving the first entry and the most recent transitions; this is not an unlimited audit log. Export the retained history separately if you need longer-term analysis.

### Example 3: Artifact without a state file

```bash
forgeplan phase PRD-001
```

If the artifact has no state file (created before phase tracking, or with tracking disabled), the output shows `current_phase: unknown` and an empty history. This is intentional - not an error. Start tracking with [`forgeplan phase-advance`](/docs/cli/phase-advance/).

Known-state JSON has `artifact_id`, `workflow_type`, `current_phase`, `advanced_at`, `schema_version`, `history` (`from`, `to`, `at`, `reason`), and `_next_action`. Missing-state JSON instead has `current_phase: "unknown"`, `workflow_type: "greenfield"`, empty `history`, `message`, and `_next_action`, without timestamps/schema version. Missing artifacts can also return this advisory unknown state; it is not an existence check.

## Unknown states, errors, and quarantine

Missing state returns the advisory unknown response. A symlinked state file, a file larger than 1 MiB, or a parsed state with `schema_version` greater than 1 is also treated as unknown, without the corrupt-YAML quarantine rename.

A YAML deserialization failure is different: the reader logs a warning and attempts to rename `<id>.yaml` to `<id>.yaml.corrupt.<UTC-unix-seconds>`. It returns unknown whether the rename succeeds or fails, logging the failure when necessary. Thus this inspection command is not unconditionally read-only, and unknown does not prove a quarantine copy was preserved. Inspect the actual state directory before initializing or advancing new state.

Invalid artifact IDs, missing workspace/store prerequisites, and other propagated read/I/O errors can still fail the command. “Advisory” is not a promise that every failure becomes an unknown response.


## How it fits the workflow

Phase tracking is an observability layer over the methodology pipeline (Shape → Validate → Code → Evidence → Activate). Read with `phase`, write with [`forgeplan phase-advance`](/docs/cli/phase-advance/). A manual marker does not execute the named stage, validate its completion, or replace the independent lifecycle checks of `activate`.

## See also

- [`forgeplan_phase`](/docs/mcp/forgeplan_phase/) - MCP equivalent
- [`forgeplan phase-advance`](/docs/cli/phase-advance/) - write the next transition
- [`forgeplan validate`](/docs/cli/validate/) - gate around the `validate` phase
- [`forgeplan activate`](/docs/cli/activate/) - the `done` terminal state of the methodology
- [Methodology guide](/docs/methodology/overview/) - Shape → Validate → Code → Evidence → Activate
