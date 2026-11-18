---
title: forgeplan score
description: "Compute R_eff - the weakest-link evidence trust score for a decision."
---

`forgeplan score` computes local **R_eff** using a weakest-link minimum, not an average. Non-terminal linked EvidencePacks contribute; superseded/deprecated packs are excluded. The selected scorer also traverses dependency relations recursively, so `min(own evidence)` alone is not the complete command result.

Verdicts are `supports`, `weakens`, and `refutes`, distinct from `review_verdict`. Congruence and evidence type affect contributions. The current expired-evidence branch returns 0.1, even for a refuting pack; it is not a gradual pre-expiry decay or a universal cap. An ordinary decision without evidence/dependency support scores zero, but an Evidence leaf has intrinsic scoring. A blind spot means missing linked evidence, not simply any zero score.

## When to use

- Right after linking a new EvidencePack (`forgeplan link EVID-012 PRD-001 --relation informs`).
- Before `forgeplan activate` - if R_eff is still 0, you are activating a promise without proof.
- Use `--all` for active PRD/RFC/ADR/Epic/Spec/Problem/Solution records; Note is excluded from the bulk decision-kind set.
- Use JSON to distinguish missing evidence from linked refuting evidence and inspect the weakest link.

## When NOT to use

- A scalar score is not implementation acceptance, lifecycle status, or upstream FPF conformance.
- Explicit IDs can be scored beyond the bulk kind set; Problem and Epic are not universally excluded.

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
      --all      Score active decision kinds (cache writes only on the legacy path)
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Score a single PRD

```bash
forgeplan score PRD-001
```

Illustrative flat contributions for test/measurement packs without dependencies, not a captured runtime transcript:

```text
PRD-001 - Auth System
  Evidence contributions:
    EVID-012  supports  CL3  valid  → 1.00
    EVID-015  supports  CL2  valid  → 0.90
    EVID-018  weakens   CL1  valid  → 0.10
  R_eff = 0.10   (weakest link: EVID-018)
  Status: AT RISK
```

Investigate the weak evidence rather than removing inconvenient results to increase a metric. A genuinely superseding pack can displace an obsolete pack via `supersede --by`; this is a lifecycle decision backed by real replacement evidence, not a guaranteed instant score increase. Recompute and inspect the actual result; refuting a weak pack does not strengthen it.

### Refresh every active decision at once

```bash
forgeplan score --all
```

Legacy scoring updates cached values. On migrated workspaces the selected score path computes fresh results from the canonical snapshot without writing retired score caches. Do not assume every health field consumes the same recursive point estimate.

### JSON for pipelines

```bash
forgeplan score PRD-001 --json | jq '.r_eff, .weakest_link'
```

## Output interpretation

Human text uses report labels `Adequate` (R_eff ≥ 0.5), `Needs Review` (≥ 0.3), and `AT RISK` (below 0.3). These are not scalar mappings to lifecycle/DerivedStatus states or permission to ship.

Congruence penalties are CL3=0, CL2=0.1, CL1=0.4, CL0=0.9. Base verdict values are supports=1, weakens=0.5, refutes=0. Thus a live weakens/CL1 pack contributes 0.1 before any type modifier. Recursive scoring additionally penalizes benchmark by 0.1 and audit by 0.2; measurement/test has no type penalty. The displayed flat evidence breakdown and CI point omit that type penalty and need not equal the recursive top-level point.

## Heuristic uncertainty band

The field `r_eff_ci` is a flat-projection heuristic, not a statistical confidence interval or proof that independent measurements agree. For `n` eligible packs and `k` stale packs, uncertainty is `min(0.5, 0.3 / sqrt(n) + min(0.3, 0.1*k))`, clipped to [0,1] around the flat point; fewer than three packs is marked insufficient. With no evidence the implementation has a separate empty-evidence result.

Worked arithmetic examples, not measurements:

| Flat evidence set | Point | Heuristic band |
|---|---|---|
| One live supports/CL3 pack | 1.0 | [0.7, 1.0] |
| Five live supports/CL3 and one live weakens/CL2 | 0.4 | approximately [0.278, 0.522] |
| One expired pack | 0.1 | [0, 0.5] |

The width directly uses count and stale count, not a test of independence, CL distribution or proximity to a future expiration. A narrow band is not a release guarantee. Compare the flat CI to the recursive score deliberately, not as interchangeable values.

## How it fits the workflow

```
code → new evidence → link → score → review → activate
                              ↑
                              └── R_eff must be > 0 to pass review gate
```

Keep score review separate from evidence collection and acceptance gates. On migrated workspaces reads do not refresh a retired cached R_eff table; flat and recursive consumers can differ.

## See also

- [`forgeplan fgr`](/docs/cli/fgr/) - orthogonal F-G-R quality axis
- [`forgeplan decay`](/docs/cli/decay/) - preview R_eff impact when evidence expires
- [`forgeplan review`](/docs/cli/review/) - checks R_eff > 0 as part of the gate
- [Evidence methodology](/docs/methodology/evidence/) - CL, verdict, decay explained
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan score --help`.

```text
Compute R_eff quality score for decisions with evidence

Usage: forgeplan score [OPTIONS] [ID]

Arguments:
  [ID]  Artifact ID (omit with --all to score everything)

Options:
      --all      Score all active decision artifacts and update cached R_eff
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
