---
title: forgeplan decay
description: "Report directly linked expired evidence and compare flat helper scores with and without expiry. This is not a forecast or the recursive dependency-aware score."
---

Report non-evidence artifacts with directly linked, nonterminal evidence whose validity has expired. The report compares the flat evidence-helper score with and without expiry; it is not the recursive dependency-aware `score` result or a forecast. An entry can have zero drop if another pack already sets the same weakest-link score.

## Usage

```text
forgeplan decay
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Reading the report

Affected artifacts show fresh and current scores, their absolute difference, and expired evidence IDs with expiry date, days expired, and individual score. Expired evidence contributes 0.1. Terminal (`superseded` or `deprecated`) evidence is excluded from the weakest-link population.

## Example

```bash
forgeplan decay
forgeplan stale
forgeplan score PRD-001 --json
```

## Responding to expiry

Inspect evidence before renewal. Repeat measurements or tests when needed, create and link replacement evidence, and supersede displaced packs. Renew only after real re-review. `stale` lists expired validity dates; `decay` reports impact through linked evidence.

## See also

- [`forgeplan stale`](/docs/cli/stale/) - hard expiry detection after the fact
- [`forgeplan score`](/docs/cli/score/) - recomputes R_eff after renewal
- [`forgeplan renew`](/docs/cli/renew/) - extend valid_until on stale artifacts
- [Evidence methodology](/docs/methodology/evidence/) - decay formula and CL penalties
- [CLI overview](/docs/cli/)
