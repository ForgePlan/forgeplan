---
title: forgeplan drift
description: "Find active ADR/RFC affected files with Git commits after artifact creation."
---

`forgeplan drift` checks active ADR/RFC artifacts with `Affected Files` entries.
It queries Git history after each artifact's `created_at` timestamp.
A changed path is a review signal, not proof that the decision was violated.
The command does not change the artifact's lifecycle status.

Drift is different from evidence decay. Decay is about time (`valid_until` expires);
drift is about code change (files moved). Both can make a decision untrustworthy,
and drift is often the one you forget.

## When to use

- Monthly architecture review - which ADR/RFC source paths changed after creation?
- After a big refactor - see which decisions need to be re-validated against the new shape of the code.
- Before citing an ADR in a PR review - is it still describing reality?
- Release prep - refresh drifted decisions before cutting the release.

## When NOT to use

- On artifacts without an `Affected Files` section - use `coverage --backfill` first.
- On a fresh workspace - drift needs git history to compute against.

## Usage

```text
forgeplan drift [OPTIONS]
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Find drifted decisions

```bash
forgeplan drift
```

Illustrative summary, not a captured runtime transcript:

```text
  ⚠ ADR-002 "Storage design" — STALE
    Created: 2026-03-15T00:00:00
    Changed: crates/forgeplan-core/src/db/schema.rs (2026-10-01 10:00:00 +0000)

  ✓ ADR-005 "Lifecycle design" — up to date
  2 decisions checked, 1 stale
```

### Machine-readable for CI

```bash
forgeplan drift --json | jq '.reports[] | select(.is_stale) | .id'
```

Feed that list into a Slack reminder or a backlog task generator.

### Remediate drift

For each drifted ADR you have three moves:

1. **Re-verify**: read the diffs and record the review; use `renew` only if a lifecycle renewal is appropriate.
2. **Supersede**: write a new ADR documenting the new reality, `supersede --by ADR-NEW`.
3. **Deprecate**: the decision is no longer applicable, mark it terminal.

## Output interpretation

| Field / label | Meaning |
|---|---|
| `is_stale: true` / `STALE` | At least one affected path has a later Git commit |
| `is_stale: false` / `up to date` | No later commit was found by the probe |
| `changed_files` | Path patterns and their last commit timestamps |

There is no heavy/minor drift verdict. Missing Git history or a failed Git probe
can leave no changed paths, so a clean result is not proof of architectural conformance.

## How it fits the workflow

```
scan → coverage → drift → remediate (renew | supersede | deprecate)
```

`drift` is one leg of the codebase ⟷ artifacts bridge. `coverage` asks "do modules
have ADRs?" and `drift` asks "do those ADRs still match the code?"

## See also

- [`forgeplan coverage`](/docs/cli/coverage/) - module-level decision coverage
- [`forgeplan scan`](/docs/cli/scan/) - populate the module list coverage uses
- [`forgeplan supersede`](/docs/cli/supersede/) - retire a drifted decision in favour of a new one
- [`forgeplan renew`](/docs/cli/renew/) - keep a still-valid decision alive
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan drift --help`.

```text
Check for drifted decisions (affected files changed after decision)

Usage: forgeplan drift [OPTIONS]

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
