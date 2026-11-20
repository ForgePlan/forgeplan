---
title: forgeplan session
description: "Show current methodology session state and next step"
---

Displays persisted session phase, active artifact, depth, enforcement mode and advisory next steps. The session phase vocabulary is `idle`, `routing`, `shaping`, `coding`, `evidence`, `pr`; validation, reasoning and activation are workflow actions, not additional persisted session states.

## When to use

- Resume work after a break - "what was I doing?"
- AI agents between tool calls - decide the next action
- Protocol enforcement - skip no phases by accident
- Multi-agent setups - the supervisor reads session state to route work

## Not to use when

- You want a task list → use `TODO.md` or your task tracker
- You want project-level health → use [`forgeplan health`](/docs/cli/health/)

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

## Examples

Show the current phase:

```bash
forgeplan session
```

Reset state after aborting a task:

```bash
forgeplan session --reset
```

## Output interpretation

Illustration of a shaping session, not an exact captured display:

```
Session: shaping
  Active artifact:  PRD-046
  Depth:            Standard
  Next step:        forgeplan validate PRD-046
  Last transition:  2026-04-11 14:05 (routing → shaping)
```

Persisted phases:

| Phase | Meaning |
|---|---|
| `idle` | No active session task |
| `routing` | Choose depth and pipeline |
| `shaping` | Shape and validate the decision |
| `coding` | Implement the change |
| `evidence` | Collect/review evidence |
| `pr` | Prepare the pull request |

State lives in `.forgeplan/session.yaml`. `--reset` clears phase and active-session fields; it does not validate artifacts, erase debt or pass any gates.

## How it fits

Session phase and per-artifact lifecycle phase are distinct. Methodology actions such as validate → reason → code → evidence → review → activate remain required where applicable, but not every successful command advances the session. Inspect the active artifact and real work before correcting stale session state; do not reset merely to bypass a gate.

## See also

- [`forgeplan route`](/docs/cli/route/) - determine depth and pipeline
- [`forgeplan health`](/docs/cli/health/) - project-level health
- [Unified Workflow](/docs/guides/git-workflow/) - full cycle

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan session --help`.

```text
Show methodology session state (current phase, active artifact)

Usage: forgeplan session [OPTIONS]

Options:
      --reset    Reset session to Idle
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
