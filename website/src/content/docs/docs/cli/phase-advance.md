---
title: forgeplan phase-advance
description: "Manually advance (or set) the advisory phase marker for an artifact and record an immutable transition entry. Out-of-order jumps allowed."
---

`forgeplan phase-advance` moves an artifact to the next methodology phase (Shape, Validate, Adi, Code, Test, Audit, Evidence, Done) and records the transition in `.forgeplan/state/<id>.yaml` with a timestamp and optional reason. The history is append-only — once written, it cannot be edited or deleted, only appended to.

This is the **advisory** layer: it does not check whether the jump makes sense (you can go straight from Shape to Done if you want), so out-of-order moves are allowed by design — useful for trivial fixes or backfilling old artifacts. Strict ordering enforcement is planned for a later PRD under EPIC-005.

Mirrors [`forgeplan_phase_advance`](/docs/mcp/forgeplan_phase_advance/) on the MCP side.

## When to use

- A tool ran but phase tracking was off, and now you want the artifact to reflect what really happened — advance manually to catch up.
- An artifact moved from `code` to `audit` because a PR review wave just finished — record it.
- An old artifact (created before phase tracking existed) needs its history walked forward to show up correctly in current reports.
- A trivial fix justifies skipping straight to `done` — pass `--reason` so the skip is documented.

## When NOT to use

- As a structural gate (something that blocks other commands) — phase-advance only writes the marker. For real gating, use [`forgeplan validate`](/docs/cli/validate/).
- To rename a phase or rewrite the history — entries are immutable. Add a new entry with a corrective `--reason` instead.
- Without a `--reason` when the jump is not obvious — six months from now during an audit, you will not remember why.

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

Records the transition with a short justification. The reason is preserved forever — future audits can replay exactly why the artifact moved.

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

## How it fits the workflow

Phase tracking sits alongside the methodology pipeline (Shape → Validate → Code → Evidence → Activate). Read the current state with [`forgeplan phase`](/docs/cli/phase/), write the next transition with `phase-advance`. Treat the `--reason` field like a commit message — it is the audit trail for how an artifact moved through the pipeline.

## See also

- [`forgeplan_phase_advance`](/docs/mcp/forgeplan_phase_advance/) — MCP equivalent
- [`forgeplan phase`](/docs/cli/phase/) — read current state + history
- [`forgeplan activate`](/docs/cli/activate/) — the methodology activation gate
- [Methodology guide](/docs/methodology/overview/) — Shape → Validate → Code → Evidence → Activate
