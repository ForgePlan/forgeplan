---
title: forgeplan gaps
description: "Find pipeline compliance gaps - Deep/Critical artifacts missing their required downstream stages."
---

Help registration alone does not establish runtime acceptance for every workspace layout.

`forgeplan gaps` checks linked artifact kinds for PRDs and also reports
missing/weak evidence on active decisions, old drafts and orphan artifacts.
It examines links in either direction; it does not verify a complete ordered
pipeline or a minimum Epic fan-out.

This is not a validation failure (the artifact itself may be fine) - it's a
**methodology compliance** check: did you stop halfway?

## When to use

- Mid-sprint: "I said this was Deep, did I actually build out Spec + ADR?"
- After reviewing a `calibrate` suggestion and deliberately updating depth - see what's missing.
- Pre-release health pass: "is the project aligned with its own methodology?"
- Brownfield cleanup: imported artifacts often have depth without downstream chain.

## When NOT to use

- For an ordered dependency plan: this report checks presence, not execution order. Tactical artifacts can still have evidence, age or orphan findings.
- As a blocker for activation - gaps are advisory, `validate` is the gate.

## Usage

```text
forgeplan gaps
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### See all pipeline gaps

```bash
forgeplan gaps
```

Illustrative pipeline example, not a captured command transcript:

```text
Pipeline compliance gaps
─────────────────────────
Deep PRDs without linked Spec:
  PRD-004  Search Intelligence   (depth=Deep, 0 Specs, 1 ADR)
  PRD-011  FPF KB vector search  (depth=Deep, 0 Specs, 0 ADRs)

Standard PRD without linked RFC:
  PRD-018  OpenSpec integration  (no linked RFC)

Orphan artifact:
  PRD-022  Import design  (no links)
```

### Fix gaps

For each line:

1. **Missing Spec on Deep PRD** → `forgeplan new spec "API for PRD-004"` and link.
2. **Missing ADR on Deep decision** → run `forgeplan reason` then `forgeplan new adr`.
3. **Missing RFC on Standard PRD** → create and link the relevant RFC.
4. **Orphan** → add justified links; lowering depth does not remove the orphan condition.

## Output interpretation

| Stored PRD depth | Checks in this handler |
|---|---|
| Tactical / Note | No depth-specific pipeline check |
| Standard | Missing linked RFC is MUST |
| Deep | Missing RFC is MUST; missing Spec or ADR is SHOULD |
| Literal `critical` legacy value | Missing parent Epic, RFC or ADR is MUST; no minimum fan-out is checked |

A gap signals missing expected artifacts. Create them or review whether depth is
justified; `calibrate` suggests a depth but does not change it.

## How it fits the workflow

```
calibrate → (depth may change) → gaps → create missing downstream → validate → activate
```

`calibrate` suggests depth. `gaps` compares the recorded depth and linked artifacts.
Neither substitutes for reviewing the decision or its evidence.

## See also

- [`forgeplan calibrate`](/docs/cli/calibrate/) - recalibrate depth when gaps appear
- [`forgeplan health`](/docs/cli/health/) - project-level aggregate
- [`forgeplan blocked`](/docs/cli/blocked/) - dependency graph view
- [Artifact Model](/docs/methodology/overview/) - depth-to-pipeline mapping
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan gaps --help`.

```text
Show pipeline compliance gaps by depth

Usage: forgeplan gaps

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
