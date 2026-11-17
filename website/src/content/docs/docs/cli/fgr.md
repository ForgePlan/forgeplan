---
title: forgeplan fgr
description: "Inspect Formality, Granularity, and Reliability independently of lifecycle. Each axis is from 0 to 1. Overall quality is the geometric mean (F × G × R)^(1/3), not an arithmetic average or a 0–3 scale."
---

Inspect Formality, Granularity, and Reliability independently of lifecycle. Each axis is from 0 to 1. Overall quality is the geometric mean `(F × G × R)^(1/3)`, not an arithmetic average or a 0–3 scale.

## Usage

```text
forgeplan fgr [OPTIONS] [ID]
```

## Arguments

```text
  [ID]  Artifact ID (scores all if omitted)
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Reading the scores

Formality measures validation compliance; Granularity measures content completeness. Reliability combines cached R_eff, links, and freshness using configured weights. Grades use strict thresholds: A > 0.8, B > 0.6, C > 0.4, D > 0.2, otherwise F. Without ID, all artifacts are scored. This does not replace validation.

## Examples

```bash
forgeplan fgr PRD-001
forgeplan fgr --json | jq '.results | sort_by(.overall) | reverse | .[0:10]'
forgeplan fgr --json | jq '.results[] | select(.formality > 0.8 and .reliability < 0.4)'
```

## JSON and workflow

JSON contains `results`, `_next_action`, and `hints`. Each result has `id`, `title`, `formality`, `granularity`, `reliability`, `overall`, and `grade`. Improve structure with `validate`, detail during shaping, and reliability with evidence and `score`. Use `fpf check` for FPF rules; no `fpf trust` subcommand exists.

## See also

- [`forgeplan validate`](/docs/cli/validate/) - rule-level detail for the F axis
- [`forgeplan score`](/docs/cli/score/) - R_eff that feeds the R axis
- [`forgeplan health`](/docs/cli/health/) - project-wide F-G-R aggregate
- [Quality Gates](/docs/methodology/evidence/) - FGR and Trust Calculus
- [CLI overview](/docs/cli/)
