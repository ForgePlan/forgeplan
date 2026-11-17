---
title: forgeplan activate
description: "Publish an artifact by changing its lifecycle status to active. For PRD, Epic, Spec, RFC, and ADR, the default path requires methodology gates and schema validation; activation is not just a status edit or proof that code has been implemented."
---

Publish an artifact by changing its lifecycle status to `active`. For PRD, Epic, Spec, RFC, and ADR, the default path requires methodology gates and schema validation; activation is not just a status edit or proof that code has been implemented.

## Usage

```text
forgeplan activate [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID
```

## Options

```text
      --force    Force activation even if validation has MUST errors
  -h, --help     Print help
  -V, --version  Print version
```

## Default gates

For the five guarded kinds, the trimmed stored body must be at least 100 UTF-8 bytes, an evidence-link heuristic must pass, recognized stub content must be absent, and schema MUST findings must be clear. Without `--force`, short bodies, missing evidence links, stub content, or MUST errors refuse activation. SHOULD findings do not block it. Run `review` for the shared methodology/validation report and `score` to inspect evidence quality before activating.

## Evidence is not verified by the link gate

The released heuristic accepts an outgoing `informs` or legacy `supports` relation, or an incoming relation whose source ID begins `EVID-`. It does not establish the evidence’s substantive adequacy, verdict, congruence, freshness, or a positive R_eff. Use a real EvidencePack linked evidence → decision with `informs`, fill its structured fields, and inspect the recursive score. Do not create arbitrary links merely to satisfy the predicate.

## Examples

```bash
# After filling the existing PRD and linking real evidence:
forgeplan validate PRD-001
forgeplan score PRD-001
forgeplan review PRD-001
forgeplan activate PRD-001
```

## Force and lightweight kinds

`--force` bypasses the methodology and MUST-validation refusals, not the lifecycle transition guard or the dedicated hypothesis-state rule. A forced activation without adequate evidence can leave an active blind spot. It is an explicit exceptional override, not the ordinary completion path. Kinds outside PRD/Epic/Spec/RFC/ADR skip these quality gates; hypothesis activation instead uses `forgeplan_hypothesis_promote` on MCP. No activation runs tests or proves the decision correct.

## State and failure handling

Use a real draft ID or slug and resolve refusal messages before retrying. Already-active and terminal artifacts are not made activatable by `--force`. For reviewed stale decisions, use the documented renewal workflow; do not use the 0.38.0 renewal defect as a way around activation gates.

## See also

- [`forgeplan validate`](/docs/cli/validate/) - run the validation gate without changing state
- [`forgeplan score`](/docs/cli/score/) - verify R_eff before and after activation
- [`forgeplan review`](/docs/cli/review/) - readiness dry run
- [`forgeplan supersede`](/docs/cli/supersede/) - replace one active artifact with another
- [`forgeplan deprecate`](/docs/cli/deprecate/) - retire an active artifact without replacement
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)
