---
title: forgeplan score
description: "Compute recursive effective reliability, R_eff, using the weakest-link minimum, never an average. Draft evidence participates; terminal evidence (superseded, deprecated) is excluded. A decision with no eligible evidence has self-score 0. A leaf EvidencePack is a separate case: it can score from its own structured fields without invented child evidence."
---

Compute recursive effective reliability, R_eff, using the weakest-link minimum, never an average. Draft evidence participates; terminal evidence (`superseded`, `deprecated`) is excluded. A decision with no eligible evidence has self-score 0. A leaf EvidencePack is a separate case: it can score from its own structured fields without invented child evidence.

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

## Structured evidence fields

Declare `verdict` (`supports`, `weakens`, or `refutes`), `congruence_level` (0–3), and `evidence_type` (`measurement`, `test`, `benchmark`, or `audit`) under Structured Fields. A recognized verdict and either explicit CL or recognized source_tier are needed to declare support. Missing or unrecognized declarations fail closed through CL0 rather than silently earning full trust. If CL and source tier disagree, the lower CL wins.

## Recursive contribution rules

For fresh evidence, contribution is `max(0, verdict_score - CL_penalty - type_penalty)`. Verdict scores: supports = 1.0, weakens = 0.5, refutes = 0.0. CL penalties: CL3 = 0, CL2 = 0.1, CL1 = 0.4, CL0 = 0.9. Type penalties: test/measurement = 0, benchmark = 0.1, audit = 0.2. Expired evidence contributes exactly 0.1 instead. The decision self-score is the minimum over eligible linked evidence. For example, fresh supports/CL3/benchmark evidence contributes 0.9, not 1.0.

## Dependencies and leaf evidence

The recursive walk follows outgoing `based_on`, `refines`, `informs`, and legacy `depends_on` edges; `depends_on` is not an accepted new relation name for `link`. Dependency scores lose 0.1 for `based_on`/`informs`, and 0 for `refines`/legacy `depends_on`, floored at 0. Final R_eff is the minimum of self-score and participating dependency scores. Dependencies in draft/deprecated/superseded state, or of kind note/memory, are skipped. A leaf EvidencePack with no linked child evidence and no terminal-child exclusions returns its own field-based contribution; all-terminal children do not qualify for this exception. Evidence outgoing `informs`/`based_on` edges to supported decisions do not feed those decisions’ trust back into the evidence. Inspect `factors` and `weakest_link` for the actual walk.

## Examples

```bash
forgeplan score PRD-001
forgeplan score --all
forgeplan score PRD-001 --json | jq '.r_eff, .weakest_link, .evidence'
```

## Refresh and replacement

The command updates cached R_eff. After linking new evidence or displacing a pack, recompute the score. A still-current refuting pack is not a problem to hide: investigate it, and supersede it only when a genuine replacement displaces its claim. `--all` refreshes active decision artifacts, including Epics.

## Raw breakdown, JSON, and confidence interval

Top-level `r_eff` is the recursive result. The compatibility `evidence[].score` and `r_eff_ci.point` use the flat raw formula without evidence-type or dependency penalties; do not expect them to equal the recursive score, especially for leaf evidence. The interval is heuristic, not a statistical guarantee. It uses eligible evidence count and expired-item count: base uncertainty `0.30 / sqrt(n)`, plus 0.10 per stale item capped at 0.30; total uncertainty is capped at 0.50 and bounds at [0, 1]. Fewer than three items are labelled insufficient evidence. Single-artifact JSON has `id`, `title`, `r_eff`, `r_eff_ci`, `weakest_link`, `factors`, `evidence`, `fgr`, and `_next_action`. `--all --json` instead returns `results` and `errors`.

## Historical output format

For comparison, the documentation’s pre-v0.17 example showed only the point estimate; it is historical, not current terminal output:

```text
PRD-001 - Auth System
  R_eff = 0.80
```

## See also

- [`forgeplan fgr`](/docs/cli/fgr/) - orthogonal F-G-R quality axis
- [`forgeplan decay`](/docs/cli/decay/) - report current R_eff impact from expired evidence
- [`forgeplan review`](/docs/cli/review/) - checks R_eff > 0 as part of the gate
- [Evidence methodology](/docs/methodology/evidence/) - CL, verdict, decay explained
- [CLI overview](/docs/cli/)
