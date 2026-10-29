---
title: "forgeplan index"
description: "Run one Constellation index maintenance pass (capture → reconcile → publish, verified), optionally draining the embedding queue"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan index --help`.

```text
Run one Constellation index maintenance pass (capture → reconcile → publish, verified), optionally draining the embedding queue

Usage: forgeplan index [OPTIONS]

Options:
      --embed-profile <EMBED_PROFILE>  Also drain the embedding queue for this profile (requires the semantic-search build feature)
      --deadline-secs <DEADLINE_SECS>  Deadline for the whole pass, in seconds [default: 300]
      --list                           List a read-only page of documents in the published generation
      --offset <OFFSET>                First document to show with --list [default: 0]
      --generations                    List all generations under lance/ with size and protection state (current / leased / keepable) — the generation index
      --gc <KEEP>                      Garbage-collect generations: keep the newest N deletable ones plus CURRENT and anything a live reader holds pinned
  -h, --help                           Print help
  -V, --version                        Print version
```
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
