---
title: forgeplan blindspots
description: "Report decisions with no linked evidence and non-evidence artifacts with no incoming or outgoing relations. This checks link presence, not cached R_eff."
---

Report evidence-requiring decision kinds (`prd`, `rfc`, `adr`, `epic`, `spec`, `problem`, `solution`) with no linked evidence, excluding draft, deprecated, and superseded decisions. It also lists non-evidence artifacts with no incoming or outgoing relations. This is a link-presence check, not a zero cached-R_eff check: stale or refuting evidence can count as linked while still providing little or no scoring support.

## Usage

```text
forgeplan blindspots
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
forgeplan blindspots
# For an actual nonzero CI result on configured health debt:
forgeplan health --ci --fail-on "blind_spots=0,orphans=0"
```

## Exit behavior and triage

`blindspots` is a report and returns success even when findings exist. It is not itself a failing CI gate. Use `health --ci` for threshold-based gating. For each decision, inspect and improve evidence, link it with `informs`, and recompute `score`. For orphans, add a meaningful relation or retire an obsolete decision; do not add arbitrary links merely to hide a finding.

Requires an initialized workspace and valid configuration. Output is text only; there is no `--json` option. A successful report does not prove evidence quality: inspect `score` and evidence content separately.

## See also

- [`forgeplan health`](/docs/cli/health/) - full project dashboard (with
  `--ci` mode)
- [`forgeplan journal --risk`](/docs/cli/journal/) - time-axis view of
  blind decisions
- [`forgeplan score`](/docs/cli/score/) - R_eff for one artifact
- [`forgeplan new evidence`](/docs/cli/new/) - create an EvidencePack
- [`forgeplan link`](/docs/cli/link/) - attach evidence with `--relation informs`
