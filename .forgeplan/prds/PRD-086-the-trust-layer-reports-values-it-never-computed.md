---
depth: standard
id: PRD-086
kind: prd
links:
- target: ADR-002
  relation: based_on
- target: ADR-020
  relation: based_on
status: draft
title: The trust layer reports values it never computed
---

---
assigned_number: 86
predicted_number: 86
slug: prd-the-trust-layer-reports-values-it-never-computed
---

# PRD-086: The trust layer reports values it never computed

## Problem

R_eff is the number the whole product is for. Four defects make it, and the
detector that reads it, report things that were never computed.

None of them fail. Each returns a plausible value, which is why all four
survived: a zero looks like an honest zero, and a warning about a missing link
looks like a warning about a missing link.

**An EvidencePack is asked for its evidence.** `r_eff_recursive` collects the
packs linked to an artifact and takes the min. Run against an EvidencePack it
finds nothing — a pack has no packs — and returns `self_score = 0.0` with the
factor `No evidence found (L0)`. A canonical pack (`verdict: supports`,
`congruence_level: 3`) scores zero.

The intrinsic score already exists. `score_evidence` computes it from verdict,
congruence level and expiry, and is applied to that same pack whenever it is
scored *as evidence for something else*. Only the pack itself never gets it.

Worse, the pack's outgoing `informs` edge to the artifact it supports is
classified as a **dependency**, so trust flows from the decision down into the
evidence. Backwards.

**The cascade contradicts the routing table.** A Note is the artifact for
trivial, reversible work: no ADI, no evidence, expires in 90 days. The
dependency walk then treats an active Note with no evidence as zero trust and
poisons every chain based on it. Forgeplan says a Note needs no evidence and
then scores everything downstream of it as unevidenced.

ADR-002 already resolved this shape for `draft` dependencies — skip, with a
logged factor — and rejected "count draft as 0" for penalising planning ahead.
The same reasoning applies to kinds that are ceremony-free by design.

**The anomaly detector reports three things it did not check.** It prints
`R_eff=0` for artifacts whose stored `r_eff` is nonzero but below threshold; it
labels every give-up `cycle or depth cap` in graphs with zero cycles; and its
ancestor walk gives up where `forgeplan_score` resolves the weakest link fine,
emitting `weakest_link: null, chain_depth: 0` for artifacts the scorer answers.

**`advance_phase` regresses.** It has no monotonicity guard. MCP
`forgeplan_validate` calls it with `Phase::Validate` on PASS, so validating an
already-shipped artifact walks its phase back from `done`, and
`forgeplan_health` then reports a phase mismatch that the artifact did not have
until it was validated.

## Goals

- An EvidencePack scores on its own merits, using the formula that already
  exists, without inventing child evidence to satisfy the walk.
- The weakest-link cascade is preserved exactly as it is for kinds that can owe
  evidence, and skips the kinds the methodology exempts.
- Every number and label the anomaly detector prints is one it computed.
- A phase never moves backwards on its own.

## Non-Goals

- **Weakening the weakest-link formula.** `R_eff = min(...)` stays. The three
  fixes proposed in #392 — local-only scoring, one-hop propagation, a floor at
  `self_score` — are each an average in disguise and are declined. If a
  foundation is unproven, the floor above it is not trustworthy, and a number
  saying otherwise is the failure the formula prevents.
- Changing CL penalties, decay, or the verdict scale.
- Raising the detector's depth cap. The walk is unified with the scorer's; if a
  cap is still hit afterwards it must be reported as a depth cap, not guessed
  at.
- Backfilling or migrating stored scores. `forgeplan score --all` recomputes.

## Target Users

Agents reading `r_eff` to decide whether an artifact can be relied on, and
maintainers triaging `forgeplan anomalies`. Both currently receive confident
numbers that were never computed, and neither has a way to tell which.

## Functional Requirements

- **FR-001**: An EvidencePack with no linked child evidence derives
  `self_score` from `score_evidence` over its own parsed fields, not from the
  empty-evidence path. A pack whose fields do not parse keeps `0.0` — absent
  structured fields remain CL0, per the existing contract.
- **FR-002**: An EvidencePack's outgoing `informs` / `based_on` edges to the
  artifacts it supports are excluded from its own dependency walk. Evidence
  does not depend on what it evidences.
- **FR-003**: The dependency walk skips `note` and `memory` dependencies,
  recording a factor naming the id and kind, in the same shape ADR-002 uses for
  non-active dependencies.
- **FR-004**: The anomaly detector reports the artifact's actual stored
  `r_eff`, not `0`.
- **FR-005**: The detector distinguishes a depth cap from a cycle and names
  which occurred.
- **FR-006**: The detector's ancestor walk resolves a weakest link wherever
  `forgeplan_score` resolves one, by sharing the traversal rather than
  reimplementing it.
- **FR-008**: An unparseable or unknown `verdict` fails closed. Today it falls
  through to `Supports` (score 1.0) while `congruence_level` in the same
  function fails closed to CL0 with a warning — one field punishes garbage and
  its neighbour rewards it. Protocol v1 introduces a fourth value (`unknown`),
  which the current parser would score 1.0.
- **FR-009**: Absent structured fields fail closed. A pack carrying no
  `verdict` / `congruence_level` at all currently scores its target 1.00;
  `CLAUDE.md` RED LINE #7, the `/forge` skill shipped by `setup-skill`, and
  `EVIDENCE-PROTOCOL.md` all state the opposite (CL0, 0.1). The absence is
  recorded as a factor rather than applied silently. Breaking: packs without
  fields drop from 1.0 to 0.1, and artifacts whose weakest link was such a
  pack drop with them — 3 of 166 here, unknown elsewhere. Requires a
  migration note and `forgeplan score --all`.

- **FR-007**: `advance_phase` refuses a transition to an earlier phase, leaving
  state unchanged and reporting the refusal to its caller. Explicit
  `forgeplan phase-advance --to <earlier>` remains available for deliberate
  correction.

## Related Artifacts

| Artifact | Relation |
|---|---|
| ADR-002 | based_on |
| ADR-020 | based_on |

GitHub: #325, #392, #393, #330.



