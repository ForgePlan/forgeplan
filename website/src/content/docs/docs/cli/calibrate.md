---
title: forgeplan calibrate
description: "Suggest Tactical, Standard, or Deep from artifact-content heuristics."
---

`forgeplan calibrate` applies content heuristics to an existing artifact. Unlike pre-creation routing, it reads the written body and relation count. The selected implementation can suggest Tactical, Standard, or Deep; the captured help below also mentions Critical, which this heuristic does not produce.

Signals include checkbox count, security/breaking-change wording, relation count, `parent_epic`, section count, and body length. A recommendation is a review signal, not proof that the chosen depth is correct.

Command registration alone does not establish runtime acceptance for every workspace layout.

## When to use

- Mid-sprint: "this PRD is growing - should it still be Standard?"
- After importing artifacts from outside (Brownfield) - see if routing matches reality.
- When `forgeplan route` guessed wrong during creation and you want a second opinion.
- As part of `forgeplan gaps` triage: calibrate flagged artifacts before escalating depth.

## When NOT to use

- Small Tactical Notes usually do not need repeated calibration; the command still reads their stored depth and content.
- On every artifact every day - this is a corrective tool, not a daily check.

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

Illustrative explanation of a recommendation, not captured CLI output:

```text
PRD-001 - Auth System
  current depth:     Standard
  suggested depth:   Deep
  drivers:
    + 11 checkbox items (Deep threshold: more than 10)
    + security section present
    + 6 dependency links (Deep threshold: more than 5)
  recommendation: upgrade to Deep, add Spec + ADR
```

### Scan every artifact

```bash
forgeplan calibrate
```

Without an ID, it prints artifacts needing escalation and a summary. Lower suggestions are not listed in this mode, and a no-escalation summary is not proof that routing is correct.

## Output interpretation

| Signal | Suggested minimum |
|---|---|
| More than 10 checkbox items, 8 `##` sections, 200 lines, or 5 links | Deep |
| Security/authentication/compliance section or breaking-change wording | Deep |
| More than 3 checkbox items, 4 `##` sections, 50 lines, or 2 links | Standard |
| Nonempty `parent_epic` | Standard |
| No matching signal | Tactical |

If the recommendation is higher than current: either upgrade the `depth` field and add
the missing pipeline stages (Spec, ADR) or split the artifact into smaller ones.

If the recommendation is lower: consider simplifying - you may be over-engineering.

## How it fits the workflow

```
route → new (depth=X) → code → calibrate → upgrade depth? → add Spec/ADR?
```

`calibrate` is the self-correcting loop. Routing is a guess; calibration is an audit.
Use it when something feels off, or as a scheduled monthly health pass.

## See also

- [`forgeplan route`](/docs/cli/route/) - initial depth guess before creation
- [`forgeplan gaps`](/docs/cli/gaps/) - finds missing pipeline stages after calibration
- [`forgeplan estimate`](/docs/cli/estimate/) - depth shapes the estimation scale
- [Depth Calibration](/docs/methodology/routing/)
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan calibrate --help`.

```text
Suggest depth level (Tactical/Standard/Deep/Critical) based on artifact content

Usage: forgeplan calibrate [ID]

Arguments:
  [ID]  Artifact ID (checks all if omitted)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
