---
title: forgeplan session
description: "Read the current methodology session, or clear it with --reset. Session state is separate from the per-artifact advisory phase shown by phase and phase-advance."
---

Read the current methodology session, or clear it with `--reset`. Session state is separate from the per-artifact advisory phase shown by `phase` and `phase-advance`.

## Usage

```text
forgeplan session [OPTIONS]
```

## Options

```text
      --reset    Reset session to Idle
  -h, --help     Print help
  -V, --version  Print version
```

## Phases and output

Session phases are `idle`, `routing`, `shaping`, `coding`, `evidence`, and `pr`. Output includes the phase, optional artifact, optional depth with enforcement state, a `Since` timestamp when present, and up to five recent transitions. It does not calculate elapsed time. Artifact phases such as `validate`, `adi`, `test`, `audit`, and `done` are not session phases.

## Examples

```bash
forgeplan session
forgeplan session --reset
```

## Reset and boundaries

Reset sets the phase to Idle and clears the active artifact, route depth, and phase-start timestamp. It keeps the session's transition history and does not alter artifacts or their advisory phase history. Inspect active work before resetting. Use `phase ID` for artifact history and `health` for the whole workspace.

## See also

- [`forgeplan route`](/docs/cli/route/) - determine depth and pipeline
- [`forgeplan health`](/docs/cli/health/) - project-level health
- [Unified Workflow](/docs/guides/git-workflow/) - full cycle
