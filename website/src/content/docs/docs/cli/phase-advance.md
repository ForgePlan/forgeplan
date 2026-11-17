---
title: forgeplan phase-advance
description: "Explicitly set an artifact's advisory phase marker and record a transition in bounded history. Forward jumps and regressions are allowed."
---

`forgeplan phase-advance` explicitly sets an existing artifact's advisory
phase marker and records a transition in `.forgeplan/state/<id>.yaml`.
Forward jumps and regressions are allowed; it does not change lifecycle status.
Repeatedly setting the current phase is a no-op, so a new reason is not recorded.

History is not an immutable permanent ledger: the state file is rewritten
atomically and capped at 1024 entries, keeping the initial entry and newest
transitions. CLI reasons over 4096 bytes are rejected; stored reasons are
truncated to 512 bytes at a UTF-8 boundary. Keep long-lived audit evidence elsewhere.

Mirrors [`forgeplan_phase_advance`](/docs/mcp/forgeplan_phase_advance/) on the MCP side.

## When to use

- A tool ran but phase tracking was off, and now you want the artifact to reflect what really happened - advance manually to catch up.
- An artifact moved from `code` to `audit` because a PR review wave just finished - record it.
- An old artifact (created before phase tracking existed) needs its history walked forward to show up correctly in current reports.
- A trivial fix justifies skipping straight to `done` - pass `--reason` so the skip is documented.

## When NOT to use

- As a structural gate (something that blocks other commands) - phase-advance only writes the marker. For real gating, use [`forgeplan validate`](/docs/cli/validate/).
- To rewrite history through this command — add a corrective transition instead; the local YAML is not tamper-proof storage.
- Without a `--reason` when the jump is not obvious - six months from now during an audit, you will not remember why.

## Usage

```text
forgeplan phase-advance [OPTIONS] --to <TO> <ID>
```

## Arguments

```text
  <ID>  Artifact ID to advance
```

## Options

```text
      --to <TO>          Target phase: shape, validate, adi, code, test, audit, evidence, done [possible values: shape, validate, adi, code, test, audit, evidence, done]
      --reason <REASON>  Optional reason / justification (recorded in history)
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```

## Examples

### Example 1: Advance after tests pass

```bash
forgeplan phase-advance PRD-057 --to test --reason "FR tests green"
```

Records a transition and short reason when the phase actually changes. This records the caller's assertion that tests passed; it does not execute or check tests.

### Example 2: Skip ahead for a trivial fix

```bash
forgeplan phase-advance NOTE-019 --to done --reason "trivial typo fix"
```

Skipping intermediate phases is allowed (advisory layer). Always pair the skip with a clear `--reason` so an auditor reading the history later understands the call.

### Example 3: Backfill an old artifact

```bash
forgeplan phase-advance PRD-001 --to shape
forgeplan phase-advance PRD-001 --to validate
forgeplan phase-advance PRD-001 --to code --reason "backfilled from git history"
```

Walks an artifact created before phase tracking existed through the phases so it appears correctly in current reports. Reason on the final transition explains where the data came from.

`--json` contains `artifact_id`, `current_phase`, `workflow_type`,
`advanced_at`, `history_entries`, `reason`, `suggested_next`, and
`_next_action`. `reason` echoes the supplied argument, not necessarily the
truncated stored value. At `done`, both next-step fields are null.
An unknown artifact fails instead of creating a new artifact. Corrupt state
can be quarantined and reinitialized; do not assume an uninterrupted history.

## How it fits the workflow

Phase tracking sits alongside the methodology pipeline (Shape → Validate → Code → Evidence → Activate). Read the current state with [`forgeplan phase`](/docs/cli/phase/), write the next transition with `phase-advance`. Treat the `--reason` field like a commit message - it is the audit trail for how an artifact moved through the pipeline.

## See also

- [`forgeplan_phase_advance`](/docs/mcp/forgeplan_phase_advance/) - MCP equivalent
- [`forgeplan phase`](/docs/cli/phase/) - read current state + history
- [`forgeplan activate`](/docs/cli/activate/) - the methodology activation gate
- [Methodology guide](/docs/methodology/overview/) - Shape → Validate → Code → Evidence → Activate
