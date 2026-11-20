## Round 1 system review — CONCERNS

**Reviewer:** RfcSystemRoundOneSol, equivalent read-only Profile B system-dev; actual model `openai-codex/gpt-6.1-sol`. Independent of RFC author IntegratedRfcSol.

### Frozen scope verified

Product branch `plan/analytical-product-20261120`, HEAD/base `b0c9259dc04d0c6d119844895c7a629fde6bea80`.

- RFC canonical key: `rfc-rust-analytics-captures-kbe91e3dbf1920005719327653381b2fe`
- Status/depth: draft/Critical; 14 links.
- File SHA-256: `fdd53ac34a1268e10ed77c09f6e40e04793b32d60cc65325df61ef81904160de`
- Exact post-frontmatter body SHA-256: `d31924ef5a7cbaf299173a7f190b880e71476dbced8a2f1dd4d72697199190af`
- All 25 manifest inputs matched their expected hashes in **both Product and original upstream locations**.
- Handoff and both FR-41 research receipts matched supplied hashes.
- All 14 current-source appendix fingerprints matched when inspected.

Reviewed relevant PRD-093/SOL-001/SPEC-015 and ADR-033–038 provisions, both research reports, handoff, and active Core ADR-001/026/031. Source fingerprints establish identity, not implementation correctness.

### Finding

**SYS-01 — MEDIUM, open: AC-5 publication timing remains ambiguous.**

**References:** RFC lines **326, 363–364, 450, 616**; PRD-093 line **312**.

P1-FOUND must actually build/publish a generation to produce EP-1 evidence; P1-REC subsequently proves publication failure/recovery. AC-5 nevertheless states P1 exit is **before migration/publication**. Line 450 distinguishes draft-document publication from runtime-generation acceptance, but does not distinguish controlled runtime test publication from production publication/cutover. The final handoff also expressly retains this clarification as unresolved.

**Consequence:** a literal interpretation prohibits the publication needed to establish eligibility, while an expansive interpretation could permit production publication before recovery proof. The DAG itself is acyclic; the uncertainty concerns the authorization boundary.

**Required disposition:** have the owning author/coordinator explicitly distinguish authorized isolated candidate/fault-test publication from accepted production publication or cutover. State which may occur under P1-FOUND and which remains blocked until P1-REC/AC-5. Preserve the parent requirement and external authorization gates; obtain owner concurrence if clarification changes its meaning.

**Strongest counterargument:** test publication before production acceptance is normal and evidently intended by EP-1. That makes this a bounded documentary concern, not a demonstrated runtime defect or a reason to redesign P1. The present wording and retained handoff caveat still leave the interpretation unsettled.

### Coverage and results

| Assigned criterion | Result and evidence |
|---|---|
| File-first authority and single publisher | Adequate design separation. Canonical authoring and physical projection publication have different authority; no second canonical store. RFC **139–144, 165–177, 208–212, 233–235**; ADR-035 **93–114**. |
| Atomic visibility, durability, held readers | Explicit Prepared→Staged→Verified→Visible→DurablyAcknowledged states; uncertain ACK reconciled rather than retried; exact pins survive until close; GC respects readers. Platform barriers and fault proofs remain required. RFC **233, 326, 364, 395**. |
| Retention, invalidation, migration, rollback | Raw receipts are not disposable cache; promotion retains extracts/digests; parse failures do not imply deletion; old facts remain stale; recreation has new incarnation. Rollback preserves history, expiry and actual caller compatibility. RFC **237–241, 395–397**; ADR-035 **99–106, 153**. |
| Qualified/coherent cross-source identity | No bare-ID winner, clone-name merge or latest-result union. Required bytes/configuration must arrive; missing/denied sources cannot prove no consumers. RFC **191–200, 225–227, 267–269**; ADR-034 **89–108**; ADR-037 **95–119**. |
| Real compatibility and rollout | Actual three-repository chain, independent consumer inventory, backward/forward/mixed versions, old messages, semantic breaks and harmless edits are required. Actual map and TaskLink consumers have owner-reviewed version/migration/rollback gates. RFC **269, 330, 372–373, 422–428**. |
| FPV/PROB-082 reconciliation | All sixteen dispositions retained, not automatic adoption. Historical FPV-07 discrepancy is separated from current 02+03→07 ordering. B2/B3 normative progress does not claim B4 migration delivery. RFC **456–477**, especially **458, 469**; **157, 434–446**. |
| ADR-001 producer/control distinction | Pure captured-byte processing is only a proposed classification. CodeIndex-like public ports and SourceAdapter precedent still need owner clarification/amendment. No trait-name exemption. RFC **150–157, 434**; matches active ADR-001 **20–47**. |
| Protected surfaces and #589 | #589/A1, read/upkeep/bootstrap boundaries and protected artifacts remain explicit owner gates. Existing allowed upkeep is neither silently revoked nor generalized to all reads. RFC **123, 145–151, 235, 345, 401, 442**. |
| Legacy effects and reconciliation | Independent pre-output oracle, meaningful regressions, actual new writes/in-flight effects, old readers and duplicate prevention are retained. Lost ACK reconciles external effect identity; artifact undo is not compensation. RFC **273–277, 331, 374–376, 397**; SPEC **S-B20-N, S-B30-N, S-X10**. |
| P1 ordering and later eligibility | P1 does not depend on P3/P5/P7 or SQLite. EXT:TRUST precedes D-CONTRACT; P4 authenticity is not required through a hidden cycle for P1. Early executed examples need independently established authenticity or remain unavailable. RFC **255, 338–370, 383**. Publication timing needs SYS-01 disposition. |
| Operational gates and covering negatives | Required faults, retention expiry, missing consumers, field substitutions, wrapper-laundered reports and actual consumer propagation are specified—not replaced by shared envelope fixtures. RFC **324–332, 579–605**; SPEC **S-B31-N, S-B32-N, S-B39-N, S-X07, S-X11**. |

### Admission boundary

No unresolved HIGH/CRITICAL finding established in this assigned documentary slice. Open backend/A–B/identity/trust/retention/current-owner decisions are legitimate **pre-enablement gates**, not implementation achievements or automatic planning defects.

Resolve SYS-01 before an unqualified system-review PASS. This report does not complete Critical round two, canonical evidence recording, guardian review, execution admission, activation or FR-41 owner reconciliation.

**NOT_RUN:** native CLI/index/validation/ADI reruns; builds, tests, lint, formatters, experiments, runtime/source correctness audit, crash/durability/rollback execution, tracker/lifecycle operations. No files or artifacts written.