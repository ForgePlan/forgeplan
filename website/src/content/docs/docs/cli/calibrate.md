---
title: forgeplan calibrate
description: "Suggest Tactical, Standard, or Deep depth from stored artifact content and outgoing relation count, without changing the artifact."
---

`forgeplan calibrate` suggests a depth for an existing artifact from its stored
body and outgoing relation count. Unlike `route`, it uses deterministic content
heuristics, not a provider call. It reports advice; it does not change `depth`.

The released heuristic suggests Tactical, Standard, or Deep; it does not produce
Critical. Its signals are checkbox count, security and breaking-change text,
relation count, `parent_epic`, second-level heading count, and body line count.

## When to use

- Mid-sprint: "this PRD is growing - should it still be Standard?"
- After importing artifacts from outside (Brownfield) - see if routing matches reality.
- When `forgeplan route` guessed wrong during creation and you want a second opinion.
- As part of `forgeplan gaps` triage: calibrate flagged artifacts before escalating depth.

## When NOT to use

- To prove a depth is correct: this is a heuristic, not a semantic or implementation audit.
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

Illustrative excerpt for a Standard artifact containing a security section:

```text
PRD-001 "Auth System"
──────────────────────────────────────────────────
  Current:   "standard"
  Suggested: Deep ⬆ ESCALATION
  Signals:
    Deep → security_section (present)
```

### Scan every artifact

```bash
forgeplan calibrate
```

Without an ID, prints only upward escalations plus a summary. When none are found,
it prints `All artifacts are at appropriate depth levels.` This does not prove
that every artifact is correctly routed or identify possible downgrades.

## Output interpretation

| Signal | Minimum suggested depth |
|---|---|
| 4–10 checkbox lines; more than 10 | Standard; Deep |
| Security/compliance/authentication section or `security considerations` | Deep |
| `breaking change`, `## migration`, or `backwards compatibility` | Deep |
| 3–5 outgoing relations; more than 5 | Standard; Deep |
| Non-empty `parent_epic` | Standard |
| 5–8 `## ` headings; more than 8 | Standard; Deep |
| 51–200 body lines; more than 200 | Standard; Deep |
| No signal | Tactical |

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
