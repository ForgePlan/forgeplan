# FPV-09 — [CONFORMANCE] Add capability manifest and host/orchestrator conformance framework

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `3`
- **Dependencies:** `FPV-02, FPV-03, FPV-04, FPV-05, FPV-07`
- **Summary:** Make compatibility claims evidence-based and machine-generated.

---

## Objective

Create CapabilityManifest, host conformance and orchestrator conformance suites so ForgePlan only claims support that is automatically verified.

## Host checks

- install/discovery;
- MCP and skills;
- contract retrieval;
- path/command policy;
- actor propagation;
- execution registration;
- Evidence submission;
- independent verification;
- errors and retry/resume;
- uninstall.

## Orchestrator checks

- state ownership;
- external references;
- idempotency;
- duplicate prevention;
- result/Evidence collection;
- failure recovery;
- completion ≠ acceptance.

## Acceptance Criteria

- [ ] Manifest schema is published under Protocol v1.
- [ ] Fixture repository and canonical WorkContract exist.
- [ ] Conformance output is versioned JSON with tested versions, commit SHA and date.
- [ ] Capability levels are full/partial/advisory/unsupported/experimental.
- [ ] Website/Marketplace compatibility matrix can be generated from results.
- [ ] Failed required test prevents stable badge/publication.
- [ ] At least one reference adapter runs end-to-end in CI.
