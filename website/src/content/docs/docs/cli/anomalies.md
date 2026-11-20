---
title: "forgeplan anomalies"
description: "Issue #289 — detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, etc.). Each entry carries severity + tier + suggested_resolution so orchestrators can dispatch fixes"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan anomalies --help`.

```text
Issue #289 — detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, etc.). Each entry carries severity + tier + suggested_resolution so orchestrators can dispatch fixes

Usage: forgeplan anomalies [OPTIONS]

Options:
      --kind <KIND>          Filter by anomaly kind (snake_case: stuck_draft, orphan_link, mistyped_based_on, missing_must_section, expired_evidence, weakest_link_unresolvable, phase_mismatch, circular_dependency, duplicate_artifact)
      --severity <SEVERITY>  Filter by severity (low, medium, high)
      --since <SINCE>        Only return anomalies observed at or after this RFC 3339 timestamp
      --json                 Output as JSON instead of human-readable table
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
