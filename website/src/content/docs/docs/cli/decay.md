---
title: forgeplan decay
description: "Show the current R_eff impact of already expired linked evidence."
---

`forgeplan decay` reports decisions with already expired, scoring-eligible linked
EvidencePacks. It compares R_eff as if those packs were fresh with R_eff now.
Expired evidence remains in the weakest-link calculation with its expiry penalty;
terminal displaced packs are excluded.

Use the report to plan a **Refresh sprint**: review the affected evidence,
record new observations in a `RefreshReport`, and renew only after re-verification.

## When to use

- Start of the month: "which decisions will need re-verification this sprint?"
- Before relying on an old decision - has expired evidence reduced its R_eff?
- After reviewing evidence validity dates - identify decisions already affected.
- Session start alongside `forgeplan health` for the full "what needs attention" view.

## When NOT to use

- When you have no evidence at all (R_eff already 0) - nothing to decay.
- As a one-shot gate - use `stale` for hard expiry detection.

## Usage

```text
forgeplan decay
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### See current decay impact

```bash
forgeplan decay
```

Illustrative output, not a captured runtime transcript:

```text
Evidence Decay Report — 1 artifact(s) affected:

  PRD-003 "Authentication"
    R_eff: 0.90 → 0.10 (drop: 0.80)
    ⚠ EVID-012 expired 2026-10-01 (22 days ago, score=0.9)
```

### Feed into a sprint plan

Combine with `stale` and `health`:

```bash
forgeplan decay          # current score loss from expired evidence
forgeplan stale          # what's already expired
forgeplan health         # overall blind-spot view
```

Those three commands are the full "refresh backlog" picture.

## Output interpretation

- **expired DATE (N days ago)** - the evidence's elapsed validity date.
- **R_eff X → Y** - fresh-evidence counterfactual versus current score.
- **drop** - the difference between those scores, not a future prediction.
- Prioritize re-verification; the command does not detect future expiry cliffs.

## How it fits the workflow

```
monthly review → decay → plan refresh sprint → renew evidence → score
```

Both `decay` and `stale` report elapsed validity. `decay` explains score impact;
inspect upcoming `valid_until` dates separately when planning preventive reviews.

## See also

- [`forgeplan stale`](/docs/cli/stale/) - hard expiry detection after the fact
- [`forgeplan score`](/docs/cli/score/) - recomputes R_eff after renewal
- [`forgeplan renew`](/docs/cli/renew/) - extend valid_until on stale artifacts
- [Evidence methodology](/docs/methodology/evidence/) - decay formula and CL penalties
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan decay --help`.

```text
Show evidence decay impact on R_eff scores

Usage: forgeplan decay

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
