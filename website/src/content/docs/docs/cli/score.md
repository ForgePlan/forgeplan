---
title: forgeplan score
description: "Compute R_eff — the weakest-link evidence trust score for a decision."
---

`forgeplan score` computes the **R_eff** (effective reliability) of a decision artifact
based on the EvidencePacks linked to it. R_eff follows the weakest-link rule from
Quint-code: `R_eff = min(evidence_scores)` — **never an average**. One weak piece of
evidence sinks the whole decision, which is the whole point: a PRD backed by one strong
benchmark and one refuted test is still a risky PRD.

Each evidence contribution is shaped by its `congruence_level` (CL0..CL3), `verdict`
(supports / weakens / refuses), and `valid_until` (decay applies when expired). Without
any linked evidence, R_eff = 0.0 and `forgeplan health` will flag the artifact as a
blind spot.

## When to use

- Right after linking a new EvidencePack (`forgeplan link EVID-012 PRD-001 --relation informs`).
- Before `forgeplan activate` — if R_eff is still 0, you are activating a promise without proof.
- In bulk with `--all` after a sprint to refresh cached R_eff on every active decision.
- As a debugging tool: if health flags a blind spot, `score --json` shows which evidence is dragging the min down.

## When NOT to use

- On Notes, Problems, or Epics — they are not decisions and carry no R_eff.
- Before you have any evidence — the answer will always be 0.0.

## Usage

```text
forgeplan score [OPTIONS] [ID]
```

## Arguments

```text
  [ID]  Artifact ID (omit with --all to score everything)
```

## Options

```text
      --all      Score all active decision artifacts and update cached R_eff
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Score a single PRD

```bash
forgeplan score PRD-001
```

Typical output:

```text
PRD-001 — Auth System
  Evidence contributions:
    EVID-012  supports  CL3  valid  → 1.00
    EVID-015  supports  CL2  valid  → 0.90
    EVID-018  weakens   CL1  valid  → 0.40
  R_eff = 0.40   (weakest link: EVID-018)
  DerivedStatus: COMPARED
```

The weakest link (EVID-018) is pulling R_eff down. Either strengthen that evidence,
refute it, or replace it.

### Refresh every active decision at once

```bash
forgeplan score --all
```

Used at sprint end to update cached R_eff across the workspace. `forgeplan health` reads
these cached values.

### JSON for pipelines

```bash
forgeplan score PRD-001 --json | jq '.r_eff, .weakest_link'
```

## Output interpretation

| R_eff range | DerivedStatus  | What it means                               |
|-------------|----------------|---------------------------------------------|
| 0.00        | UNDERFRAMED    | No evidence linked — blind spot             |
| 0.01–0.39   | FRAMED         | Weak evidence or refuting signals dominate  |
| 0.40–0.69   | EXPLORING      | Some support, some doubt                    |
| 0.70–0.89   | COMPARED       | Strong support, at least one caveat         |
| 0.90–1.00   | DECIDED/APPLIED| High-trust, ready to build on               |

**Congruence Level (CL) penalties:** CL3=0.0, CL2=0.1, CL1=0.4, CL0=0.9. Evidence from
the wrong context (CL0) is treated as almost-missing. Expired `valid_until` caps the
contribution at 0.1.

## R_eff confidence intervals (v0.17+, PRD-040)

As of v0.17.0 (PRD-040, Scoring Intelligence) `forgeplan score` reports a
**confidence interval** alongside the point estimate. The interval widens when
evidence is sparse, stale, or concentrated on a single EvidencePack, and
narrows as more independent evidence is linked.

Old format (v0.16 and earlier):

```text
PRD-001 — Auth System
  R_eff = 0.80
```

New format (v0.17+):

```text
PRD-001 — Auth System
  R_eff = 0.80 [0.65 — 0.92]
```

The bracketed range is a lower / upper bound. Read it as: "point estimate 0.80,
but with the evidence we currently have, the true reliability could plausibly
be anywhere from 0.65 to 0.92." Use the interval, not the point, when
deciding whether a decision is safe to ship.

### Why it matters

A point R_eff of 0.80 looks the same whether it is backed by **one** benchmark
at CL3 or **five** benchmarks at CL3. But one-piece evidence is brittle — if
that single EvidencePack turns out to be wrong, your R_eff collapses. The
confidence interval exposes that brittleness:

| Situation                            | R_eff    | Interval              |
|--------------------------------------|----------|-----------------------|
| 1 supports / CL3 evidence            | 0.80     | `[0.40 — 0.92]` wide  |
| 5 supports / CL3, 1 weakens / CL2    | 0.80     | `[0.72 — 0.88]` narrow|
| 1 supports / CL3 with valid_until expiring | 0.80 | `[0.30 — 0.90]` wide (decay) |

A wide interval means "R_eff looks fine but you are one surprise away from
falling off a cliff — add more evidence." A narrow interval means
"R_eff is stable, multiple independent proofs agree."

The interval is computed as a heuristic over evidence count, CL distribution,
verdict mix, and `valid_until` proximity — it is not a formal statistical
confidence interval, and you should not treat it as one.

## How it fits the workflow

```
code → new evidence → link → score → review → activate
                              ↑
                              └── R_eff must be > 0 to pass review gate
```

`score` is cheap, call it often. The cached `R_eff` feeds `forgeplan health`,
`forgeplan decay`, and the project dashboard.

## See also

- [`forgeplan fgr`](/docs/cli/fgr/) — orthogonal F-G-R quality axis
- [`forgeplan decay`](/docs/cli/decay/) — preview R_eff impact when evidence expires
- [`forgeplan review`](/docs/cli/review/) — checks R_eff > 0 as part of the gate
- [Evidence methodology](/docs/methodology/evidence/) — CL, verdict, decay explained
- [CLI overview](/docs/cli/)
