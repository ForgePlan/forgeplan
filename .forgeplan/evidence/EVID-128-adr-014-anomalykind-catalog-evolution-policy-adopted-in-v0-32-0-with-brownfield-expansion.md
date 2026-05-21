---
depth: tactical
id: EVID-128
kind: evidence
links:
- target: ADR-014
  relation: informs
status: active
title: ADR-014 AnomalyKind catalog evolution policy — adopted in v0.32.0 with brownfield expansion
---

# EVID-128: ADR-014 AnomalyKind catalog evolution policy — adopted in v0.32.0 with brownfield expansion

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

## Observation

ADR-014 was authored during audit-r6 closure (commit `6efde4b`) to govern how the
`AnomalyKind` enum (in `crates/forgeplan-core/src/anomalies/`) evolves as new
artifact kinds land. The Epic #287 brownfield extension added 6 new ArtifactKind
variants which triggered the consolidation discussion (audit-r6 ARCH-H1) — that
discussion crystallised into the ADR.

## Evidence

- ADR-014 lives at `.forgeplan/adrs/ADR-014-anomalykind-catalog-evolution-policy.md`,
  status: active.
- Catalog enumeration: 24 anomaly kinds at the time of ADR drafting; +5 for brownfield
  (`hypothesis_duplicate`, `coverage_business_low`, `contradictions_invariant`,
  `orphans_use_case`, `orphans_glossary_term`).
- Section 4 of ADR codifies the 4-step extension protocol for adding new
  AnomalyKind variants (purpose / surfaces / payload contract / catalog
  registration).
- Audit-r6 (commit `6efde4b`) closes ARCH-H1 — consolidates pipeline + brownfield
  anomaly surfaces under the same dispatching policy.

## Result

Catalog evolution is now governed by an explicit ADR rather than ad-hoc enum
extension. Future brownfield-style expansions follow the documented protocol;
auditors can verify policy compliance against ADR-014 §4.



