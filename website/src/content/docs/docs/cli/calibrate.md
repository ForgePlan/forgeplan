---
title: forgeplan calibrate
description: "Suggest a depth level (Tactical/Standard/Deep/Critical) by reading the artifact body."
---

`forgeplan calibrate` re-runs the depth heuristics on an existing artifact and suggests
whether it should stay Tactical, be upgraded to Standard, or escalated to Deep/Critical.
Where `forgeplan route` calibrates **before** you create the artifact, `calibrate` runs
the same signals **after** the content is written — which usually produces a more
accurate result because there is real content to read.

Signals include FR count, presence of irreversibility keywords ("schema migration",
"public API"), cross-module scope, and linked parents. If the suggested depth differs
from the artifact's current `depth` field, that's a signal to either upgrade the depth
or split the artifact.

## When to use

- Mid-sprint: "this PRD is growing — should it still be Standard?"
- After importing artifacts from outside (Brownfield) — see if routing matches reality.
- When `forgeplan route` guessed wrong during creation and you want a second opinion.
- As part of `forgeplan gaps` triage: calibrate flagged artifacts before escalating depth.

## When NOT to use

- Tactical Notes — they don't carry a depth field, calibration is meaningless.
- On every artifact every day — this is a corrective tool, not a daily check.

## Usage

```text
forgeplan calibrate [ID]
```

## Arguments

```text
  [ID]  Artifact ID (checks all if omitted)
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Calibrate one PRD after it grew

```bash
forgeplan calibrate PRD-001
```

Output:

```text
PRD-001 — Auth System
  current depth:     Standard
  suggested depth:   Deep
  drivers:
    + 9 FR (Deep threshold 8+)
    + irreversibility: "schema migration", "public API"
    + cross-module: auth, session, db
  recommendation: upgrade to Deep, add Spec + ADR
```

### Scan every artifact

```bash
forgeplan calibrate
```

Lists only mismatches (artifacts where current ≠ suggested). Empty output = everything
is correctly routed.

## Output interpretation

| Signal            | Contributes to            |
|-------------------|---------------------------|
| FR count ≥ 8      | Deep                      |
| irreversibility   | Deep or Critical          |
| cross-team scope  | Critical                  |
| 1 file, ≤1 day    | Tactical                  |
| 1–3 days, 1 module| Standard                  |

If the recommendation is higher than current: either upgrade the `depth` field and add
the missing pipeline stages (Spec, ADR) or split the artifact into smaller ones.

If the recommendation is lower: consider simplifying — you may be over-engineering.

## How it fits the workflow

```
route → new (depth=X) → code → calibrate → upgrade depth? → add Spec/ADR?
```

`calibrate` is the self-correcting loop. Routing is a guess; calibration is an audit.
Use it when something feels off, or as a scheduled monthly health pass.

## See also

- [`forgeplan route`](/docs/cli/route/) — initial depth guess before creation
- [`forgeplan gaps`](/docs/cli/gaps/) — finds missing pipeline stages after calibration
- [`forgeplan estimate`](/docs/cli/estimate/) — depth shapes the estimation scale
- [Depth Calibration](/docs/methodology/routing/)
- [CLI overview](/docs/cli/)
