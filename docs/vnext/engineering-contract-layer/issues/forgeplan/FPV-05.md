# FPV-05 — [CORE] Implement EvidenceBundle and ground-truth verification gates

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `2`
- **Dependencies:** `FPV-02, FPV-03, FPV-04`
- **Summary:** Verify actual git/test/CI evidence instead of trusting agent completion claims.

---

## Objective

Implement criterion-level EvidenceBundle ingestion and VerificationVerdict with core-side provenance checks.

## Existing work to incorporate

This issue coordinates rather than duplicates:

- #360 — git-delta provenance and activate-time ground-truth gate.
- #328 — core-side Evidence decay triggers.

## Scope

- EvidenceBundle persistence/schema.
- base/result SHA and git delta digest verification.
- changed-path scope validation.
- criterion-level supports/weakens/refutes mapping.
- command/report/CI references and hashes.
- VerificationVerdict and stable reasons.
- independent verifier policy integration.
- activation gate integration.

## Acceptance Criteria

- [ ] Empty relevant git delta cannot satisfy a code-changing criterion.
- [ ] Changed paths outside WorkContract scope block acceptance.
- [ ] Missing required Evidence produces `incomplete`, not success.
- [ ] Refuting Evidence fails its required claim.
- [ ] Stale Evidence cannot satisfy active gates.
- [ ] Base SHA drift is detected and requires revalidation/new contract according to policy.
- [ ] Builder self-report alone never creates accepted verdict.
- [ ] Verification can run read-only and produce deterministic results from the same inputs.
- [ ] Existing #360 and #328 are closed or explicitly superseded by this implementation.

## Evidence

Integration fixture with successful change, empty-delta false success, out-of-scope change, stale Evidence and refuting Evidence.
