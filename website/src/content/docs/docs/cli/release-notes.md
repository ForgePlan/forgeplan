---
title: "forgeplan release-notes"
description: "Generate Keep-a-Changelog–shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/` and categorises each touched artifact: PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed. Quality gate: only artifacts with `status==active` or `r_eff_score > 0` are emitted (override with `--draft`)"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan release-notes --help`.

```text
Generate Keep-a-Changelog–shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/` and categorises each touched artifact: PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed. Quality gate: only artifacts with `status==active` or `r_eff_score > 0` are emitted (override with `--draft`)

Usage: forgeplan release-notes [OPTIONS]

Options:
      --since <SINCE>    Git ref to start from (default: latest tag)
      --until <UNTIL>    Git ref to end at (default: HEAD)
      --output <OUTPUT>  Output format: text, markdown (alias md), json. Default: markdown [default: markdown]
      --draft            Disable the quality gate — include active artifacts without evidence and drafts with r_eff_score=0
  -h, --help             Print help
  -V, --version          Print version
```
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
