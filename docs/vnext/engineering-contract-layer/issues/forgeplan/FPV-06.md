# FPV-06 — [CORE] Introduce claim-centric Evidence scoring and R_eff v2

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `2`
- **Dependencies:** `FPV-02`
- **Summary:** Preserve weakest-link reasoning while scoring required claims rather than arbitrary attached items.

---

## Objective

Replace coarse `min(all linked evidence)` behavior with criterion/claim-centric scoring while retaining conservative weakest-link semantics.

## Existing work to incorporate

- #325 — leaf Evidence scoring bug.
- #329 — per-source F/G/R breakdown.
- #328 — decay trigger enforcement.

## Model

- required and informational claims;
- Evidence relations: supports, weakens, refutes;
- claim_score with congruence, reliability, freshness and provenance;
- `R_eff = min(required_claim_scores)`;
- audited Evidence dismissal rather than silent deletion.

## Acceptance Criteria

- [ ] Leaf Evidence with valid structured fields can receive a non-zero self score.
- [ ] Per-source F/G/R and contribution are available in JSON.
- [ ] Informational low-quality Evidence does not automatically destroy the entire artifact score.
- [ ] Missing Evidence for a required claim creates a blind spot.
- [ ] Refuting Evidence for a required claim blocks acceptance.
- [ ] Evidence dismissal records actor, reason, timestamp and policy permission.
- [ ] Migration preserves historical scores and exposes old/new comparison.
- [ ] Existing #325, #329 and relevant #328 scope are closed or superseded.

## Compatibility

Document whether R_eff v2 is opt-in, schema-version gated or introduced by major version.
