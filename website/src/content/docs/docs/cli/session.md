---
title: forgeplan session
description: "Show current methodology session state and next step"
---

Show the current methodology session state — which phase of the Forgeplan
cycle you are in (routing, shaping, validating, coding, evidence, activating)
and what the next step should be. This is the "where am I?" command for
agents and humans alike.

## When to use

- Resume work after a break — "what was I doing?"
- AI agents between tool calls — decide the next action
- Protocol enforcement — skip no phases by accident
- Multi-agent setups — the supervisor reads session state to route work

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

Typical output:

```
Session: shaping
  Active artifact:  PRD-046
  Depth:            Standard
  Next step:        forgeplan validate PRD-046
  Last transition:  2026-04-11 14:05 (routing → shaping)
```

Phases map to the Forgeplan cycle:

| Phase       | Meaning                                                  |
|-------------|----------------------------------------------------------|
| `idle`      | No active task                                           |
| `routing`   | Running `forgeplan route` — deciding depth/pipeline      |
| `shaping`   | Writing MUST sections in a new PRD/RFC/ADR               |
| `validating`| Running `forgeplan validate` until PASS                  |
| `reasoning` | ADI hypothesis generation (Standard+ depth)              |
| `coding`    | Implementing the change                                  |
| `evidence`  | Creating EvidencePack and linking                        |
| `activating`| Running `activate` after R_eff > 0                       |

The state is stored in `.forgeplan/session.yaml` and persists across CLI
invocations. Use `--reset` if the state is stuck or stale.

## How it fits

`session` is the phase indicator inside the enforced cycle:

```
route → shape → validate → reason → code → evidence → activate
```

Each successful command transitions the session forward. If you see the same
phase for hours, that is a signal to unstick — either finish the phase or
reset.

## See also

- [`forgeplan route`](/docs/cli/route/) — determine depth and pipeline
- [`forgeplan health`](/docs/cli/health/) — project-level health
- [Unified Workflow](/docs/guides/git-workflow/) — full cycle
