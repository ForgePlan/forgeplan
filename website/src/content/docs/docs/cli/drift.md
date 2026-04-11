---
title: forgeplan drift
description: "Detect decisions whose affected files changed after the decision was made — de-facto stale ADRs."
---

`forgeplan drift` compares each artifact's `Affected Files` list against git history.
If the referenced source files have been modified **after** the decision's `decided_at`
(or `activated_at`) timestamp, the decision has drifted: the code has evolved away
from the documented reasoning. The decision is technically still `active`, but in
practice it is stale.

Drift is different from evidence decay. Decay is about time (`valid_until` expires);
drift is about code change (files moved). Both can make a decision untrustworthy,
and drift is often the one you forget.

## When to use

- Monthly architecture review — which ADRs are the code actively violating?
- After a big refactor — see which decisions need to be re-validated against the new shape of the code.
- Before citing an ADR in a PR review — is it still describing reality?
- Release prep — refresh drifted decisions before cutting the release.

## When NOT to use

- On artifacts without an `Affected Files` section — use `coverage --backfill` first.
- On a fresh workspace — drift needs git history to compute against.

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

Output:

```text
Drifted decisions
─────────────────
ADR-002  LanceDB schema v2
  decided:  2026-03-15
  affected: crates/forgeplan-core/src/db/schema.rs (modified 5 times since)
            crates/forgeplan-core/src/db/migrations.rs (modified 3 times since)
  verdict:  DRIFTED — reassess

ADR-005  Lifecycle state machine
  decided:  2026-02-20
  affected: crates/forgeplan-core/src/lifecycle/mod.rs (modified 2 times since)
  verdict:  POSSIBLY DRIFTED — review changes

2 decisions show drift
```

### Machine-readable for CI

```bash
forgeplan drift --json | jq '.[] | select(.verdict == "DRIFTED") | .id'
```

Feed that list into a Slack reminder or a backlog task generator.

### Remediate drift

For each drifted ADR you have three moves:

1. **Re-verify**: read the diffs, confirm the decision still holds, run `renew`.
2. **Supersede**: write a new ADR documenting the new reality, `supersede --by ADR-NEW`.
3. **Deprecate**: the decision is no longer applicable, mark it terminal.

## Output interpretation

| Verdict           | Meaning                                               |
|-------------------|-------------------------------------------------------|
| DRIFTED           | affected files heavily modified since decision       |
| POSSIBLY DRIFTED  | minor edits, review recommended                      |
| CLEAN             | no changes to affected files (not shown by default)  |

Heavy drift on a Deep or Critical ADR is a red flag — the codebase has outgrown its
own documented architecture.

## How it fits the workflow

```
scan → coverage → drift → remediate (renew | supersede | deprecate)
```

`drift` is one leg of the codebase ⟷ artifacts bridge. `coverage` asks "do modules
have ADRs?" and `drift` asks "do those ADRs still match the code?"

## See also

- [`forgeplan coverage`](/docs/cli/coverage/) — module-level decision coverage
- [`forgeplan scan`](/docs/cli/scan/) — populate the module list coverage uses
- [`forgeplan supersede`](/docs/cli/supersede/) — retire a drifted decision in favour of a new one
- [`forgeplan renew`](/docs/cli/renew/) — keep a still-valid decision alive
- [CLI overview](/docs/cli/)
