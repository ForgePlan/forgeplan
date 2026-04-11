---
title: forgeplan gaps
description: "Find pipeline compliance gaps — Deep/Critical artifacts missing their required downstream stages."
---

`forgeplan gaps` checks each artifact against its declared `depth` and reports what
the pipeline **expects** versus what actually exists. A Deep PRD is supposed to have
at least one linked Spec and one ADR; a Critical Epic is supposed to fan out into
multiple PRDs, RFCs, and ADRs. When those downstream artifacts are missing, you have
a pipeline gap — the depth is making a promise the evidence chain doesn't keep.

This is not a validation failure (the artifact itself may be fine) — it's a
**methodology compliance** check: did you stop halfway?

## When to use

- Mid-sprint: "I said this was Deep, did I actually build out Spec + ADR?"
- After escalating depth via `calibrate` — see what's now missing.
- Pre-release health pass: "is the project aligned with its own methodology?"
- Brownfield cleanup: imported artifacts often have depth without downstream chain.

## When NOT to use

- On a workspace with only Tactical artifacts — gaps don't apply.
- As a blocker for activation — gaps are advisory, `validate` is the gate.

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

Output:

```text
Pipeline compliance gaps
─────────────────────────
Deep PRDs without linked Spec:
  PRD-004  Search Intelligence   (depth=Deep, 0 Specs, 1 ADR)
  PRD-011  FPF KB vector search  (depth=Deep, 0 Specs, 0 ADRs)

Critical Epics without linked RFCs:
  EPIC-003 Search, Discovery, Intelligence  (depth=Critical, 4 PRDs, 1 RFC)  ← expected ≥3 RFCs

Standard PRDs with missing parent Epic:
  PRD-018  OpenSpec DAG integration  (orphan)
```

### Fix gaps

For each line:

1. **Missing Spec on Deep PRD** → `forgeplan new spec "API for PRD-004"` and link.
2. **Missing ADR on Deep decision** → run `forgeplan reason` then `forgeplan new adr`.
3. **Critical Epic under-fanned** → reassess depth (maybe it's Deep, not Critical) or add the missing RFCs.
4. **Orphan** → link to parent or demote depth.

## Output interpretation

| Expected downstream by depth |                                       |
|------------------------------|---------------------------------------|
| Tactical                     | optional Note                         |
| Standard                     | PRD → RFC (ADI recommended)           |
| Deep                         | PRD → Spec → RFC → ADR (ADI required) |
| Critical                     | Epic → PRD[] → Spec[] → RFC[] → ADR[] |

A gap means the depth label is stronger than the evidence chain. Either create the
missing downstream artifact or downgrade depth via `calibrate`.

## How it fits the workflow

```
calibrate → (depth may change) → gaps → create missing downstream → validate → activate
```

`calibrate` tells you the right depth. `gaps` tells you whether your existing depth
is backed by the expected downstream chain. Together they keep the methodology honest.

## See also

- [`forgeplan calibrate`](/docs/cli/calibrate/) — recalibrate depth when gaps appear
- [`forgeplan health`](/docs/cli/health/) — project-level aggregate
- [`forgeplan blocked`](/docs/cli/blocked/) — dependency graph view
- [Artifact Model](/docs/methodology/overview/) — depth-to-pipeline mapping
- [CLI overview](/docs/cli/)
