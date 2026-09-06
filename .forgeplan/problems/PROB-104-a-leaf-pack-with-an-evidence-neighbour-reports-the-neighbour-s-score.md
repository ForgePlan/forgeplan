---
depth: tactical
id: PROB-104
kind: problem
links:
- target: PRD-086
  relation: informs
status: draft
title: A leaf pack with an evidence neighbour reports the neighbour's score
---

---
assigned_number: 104
context: '{grouping tag}'
created: 2026-09-06
predicted_number: 104
slug: prob-a-leaf-pack-with-an-evidence-neighbour-reports-the-neighbour-s-score
---

# PROB-104: a leaf pack with an evidence neighbour reports the neighbour's score

## Signal

PRD-086 FR-001 gave an EvidencePack an intrinsic score computed from its own
structured fields. The guard is:

```rust
if is_evidence_kind && evidence_items.is_empty() && terminal_skips == 0 {
```

`evidence_items` is built from `linked_evidence_ids` — every outgoing target id
plus every incoming source id, filtered to `kind == "evidence"`, with **no
relation-type filter**. So the moment a pack has any evidence-kind neighbour in
either direction, the early return is skipped, its own `verdict` and
`congruence_level` are never parsed, and `self_score` comes from the
neighbour's fields instead.

Measured on 0.36.0 with the prebuilt binary in a scratch workspace:

| Pack under test | isolated | after one link to a supports/CL3 pack |
|---|---|---|
| body is pure prose, no fields | 0.10 | **1.00** |
| `verdict: refutes`, `congruence_level: 3` | 0.00 | **1.00** |

`forgeplan score` on the second row prints
`Evidence breakdown: EVID-001 [Supports] CL3 = 1.0` for a pack whose own body
says `refutes`.

## Why this is low and not critical

An adversarial reviewer raised it as a critical trust-laundering vector. A
second agent tasked with refuting it measured the claims and downgraded it,
correctly:

- **Fail-closed parsing is intact.** FR-008/FR-009 govern how a pack scores its
  *target*, and that call site runs `parse_evidence_from_record` from the
  consumer's side. A PRD informed by the prose pack reports 0.10 before and
  after the laundering link; informed by the refuting pack, 0.00 both times.
- **Nothing propagates.** Only the pack's own self-report changes.
- **It gates nothing.** `forgeplan activate` on an EvidencePack is not R_eff-gated,
  and succeeds at 0.0 and 1.0 alike.
- **Zero impact on this graph.** All six EVID→EVID edges here
  (EVID-087→086, 089→088, 090→088, 091→089, 097→096, 099→098) are supports/CL3
  packs pointing at supports/CL3 packs, so the substituted value equals the true
  one everywhere it currently fires.

## Mostly pre-existing, partly ours

The evidence-collection code is untouched by PRD-086, and "self_score is the min
over linked evidence, never the artifact's own fields" is the original design for
every kind. Pre-diff, a prose pack scored 0.0 isolated and 1.0 once linked — so
"one link makes it report 1.00" predates this work; PRD-086 only introduced the
0.10 baseline that makes the jump visible.

The genuinely new residue is narrow: for a pack with a **negative** verdict plus
an evidence neighbour, FR-002 removed the dependency edge whose recursion used
to pull the result back down. That accidental correctness *was* the backwards
trust flow FR-002 deliberately removed, so the loss is a side effect of an
intended change, not a regression to undo.

## Fix direction

The guard should ask whether the pack has *supporting children*, not whether it
has *any evidence-kind neighbour*. Two candidate shapes:

1. Filter `linked_evidence_ids` by relation direction when the artifact is an
   evidence kind — only incoming `informs` / `based_on` / `supports` count as
   children; outgoing edges point at what the pack supports.
2. Fall back to the intrinsic score whenever the pack's own fields parse and the
   collected set contains no *incoming* evidence, rather than gating on
   emptiness.

Either touches evidence collection, which is shared by every artifact kind, so
it needs its own change with its own tests — the reason it was not folded into
PRD-086.

Regression test to write first: a `refutes`/CL3 pack linked to a `supports`/CL3
pack must not report 1.00.

## Related

| Artifact | Relation |
|---|---|
| PRD-086 | informs |


