---
assigned_number: 38
author: null
created: 2026-11-20
depth: critical
id: ADR-038
kind: adr
links:
- target: prd-document-k6a638805c55b95a65262c2e83492a6fe
  relation: based_on
- target: solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d
  relation: based_on
- target: evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403
  relation: based_on
- target: adr-bind-typed-analytical-facts-to-qualified-k60330dc7a76a8131b913e97303806321
  relation: based_on
- target: adr-preserve-versioned-authority-and-durable-k0c75d791cf683ce6ce085df9affa0dd1
  relation: based_on
- target: adr-expose-bounded-semantic-capabilities-and-k3314829d0ecfe5f986c14f0e41a5103f
  relation: based_on
- target: adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer
  relation: based_on
- target: hyp-revision-bound-test-evidence-can-support-k246a4d18955838d13fd19c9fcc1e1d36
  relation: informs
- target: hyp-a-frozen-independent-oracle-can-expose-k9fef2f8356bc50be55a1f0055af33a37
  relation: informs
parent_epic: null
predicted_number: 38
problem_ref: null
slug: adr-separate-evidence-eligibility-from-kbae526aed0144c49e4e5401aea631938
status: draft
tags: []
title: Separate evidence eligibility from execution, policy-change and deployment authority
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 38
created: 2026-11-20
predicted_number: 38
problem_ref: null
slug: adr-separate-evidence-eligibility-from-kbae526aed0144c49e4e5401aea631938
updated: 2026-11-20
---

# Separate evidence eligibility from execution, policy-change and deployment authority

## Status

D-EVIDENCE — Canonical ADR draft / Critical, 2026-11-20. PROPOSED and UNACCEPTED: publication records proposed contracts and open gates, not architecture activation, an A/B or backend choice, implementation authorization or delivered capability. The D-* name is a local traceability convention; the artifact's allocated canonical key/frontmatter is authoritative. No prior ADR is superseded. Active ADR-001/003/025/026 and current independently owned contracts remain binding; new analytical producer exceptions require explicit owner clarification or an approved amendment, not an implied bypass.

This publication succeeds the immutable disk-hold preparation. Corrected canonical EVID-229 (key evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403, SHA-256 865fba7aa30832200f2b7b90b7e75f3044c498319f22307f055c6b0c16167ae3) records one proposed-input round with two independent Sol judgments; it is not final Critical architecture acceptance. Actual native command/ADI/validation exits, provider, parsing and input-generation limitations are recorded under .local/design-adr-publication/, not presumed successful in this body. No F/G/R score or C4 acceptance is fabricated. Runtime/product experiments, builds, activation and downstream RFC remain NOT_RUN.

## Context

A test declaration, runner discovery, authenticated execution, coverage, assertion strength and behavioral parity answer different questions. A green job, report digest or trusted upload wrapper cannot substitute for the actual authorized report-production chain. A PR that controls evaluated code, its tests and required-check policy can weaken all three unless policy authority is separated.

Discovery itself may execute untrusted repository configuration, build scripts, proc macros, language servers or plugins. Labeling collection read-only does not make it passive. Conversely, having rights to run a tool does not grant rights to accept its evidence, waive checks, deploy, mutate production state or approve legacy cutover.

ADR-025 puts orchestration/processes above Core. ADR-026 puts canonical contracts/bundles/verdicts, local raw receipts and externally owned CI/deployment state in different classes. The analytical platform must make those boundaries useful without becoming a new scheduler or duplicating external truth.

## Decision drivers

- FC-1/5 and NFR-01/11 require exact subject/config/run/policy/report-production bindings and visible non-success outcomes.
- FR-19/21/37 require protected required checks, release provenance and actual production-chain evidence, not hashes presented as identity.
- FR-23/24/27/30/38 require an independent frozen behavioral oracle and post-write recovery, not parity inferred from execution authenticity.
- FC-3 and NFR-10 require durable promotion/history and explicit external effect recovery.
- Task usefulness, UI usefulness, backend preference, adoption and correctness claims have distinct evidence obligations.

## Considered options

### A — Accept signed uploaded reports and green CI summaries

**Pros:** easy integration with existing JUnit/coverage providers; captures transport integrity and uploader identity where properly authenticated.

**Cons:** trusted wrapper can sign attacker-authored JUnit with no execution; counts/exit/subject can conflict; evaluated head can weaken policy; signature does not establish scenario completeness or parity.

**Assessment:** necessary transport evidence in some deployments, insufficient acceptance contract.

### B — Explicit execution grants, authenticated production-chain evidence and scoped eligibility decisions

**Pros:** keeps process authority external, distinguishes L1–L6, rejects cross-subject/replay/policy substitutions and preserves auditability.

**Cons:** deployment-specific trust/configuration and raw material must be maintained; compromised authorized executor detection remains limited; a trustworthy run still needs an independent oracle for parity.

**Assessment:** proposed boundary, pending real adversarial production-chain and recovery evidence.

### C — Core runs all collection/tests and directly authorizes deployment

**Pros:** one component could coordinate inputs and result collection with less cross-component wiring.

**Cons:** expands privileges and breach scope, violates existing external process/orchestration boundary, conflates execution with normative acceptance/deploy authority and duplicates external state.

**Assessment:** not adopted. External permissioned executors may integrate without moving their ownership into Core.

## Decision outcome

Propose B. Core may issue/read versioned work/check contracts, compute read-only plans and evaluate receipt eligibility under protected policy. An authorized external runner/orchestrator performs discovery, collection, compilation, tests, traces, differential runs and sandbox/process management. The tracker still owns WHO/WHEN; artifact claims are write mutexes, not assignments. Core eligibility does not execute merge/deploy/cutover or grant an exception implicitly.

### Collection and execution authority

Each operation declares actor, subject/revision/configuration, tool/producer, requested capabilities, allowed files/network/environment/secrets, execution effects, report channel and expiry/replay identity. Security/executor owners approve the threat model and sandbox/grant enforcement before enabling repository-authored configuration, plugins, macros or hooks. A supposedly passive listing command must be classified by what it can execute. Denial/unsupported sandbox capability is explicit; no unsandboxed fallback or automatic build to make a query succeed.

Do not choose sandbox technology or invent a cryptographic trust root here. Before production, the named security/CI owner supplies allowed runner/report producers, signer issuance/revocation, isolation and artifact handoff, policy-change authority, exception scope/expiry, retention and incident response. Missing trust configuration blocks execution-based eligibility, not merely a UI warning.

### Six evidence dimensions

| Level | Claim and required source | Non-transfer rule |
|---|---|---|
| L1 declared | Test declaration in scoped source/configuration | Not discovery or a run |
| L2 discovered | Runner discovery under exact configuration with authorized collection | Not execution; collection itself may execute code |
| L3 executed | Authenticated actual producer/run/attempt/outcome and raw results | Not coverage, assertion strength or parity |
| L4 covered | Bound line/region/branch coverage with attribution scope | Aggregate coverage is not per-test→region attribution |
| L5 strength | Assertions/mutation evidence in declared scope; surviving mutant classification | Survival may mean equivalent mutant; strength is not universal correctness |
| L6 scenario/parity | Accepted SPEC scenario/effects and independent bounded oracle/run comparison | Authentic execution alone is not parity or all-behavior equivalence |

These are separate facets, not a single numeric ladder automatically completed by the highest label. Stable test identities retain revision/configuration and discovery/run linkage. Indirect scenario tests may matter without a direct call edge. Keep failed, skipped, not-run, no-record, interrupted, not-started, partial, stale and unknown attribution as first-class outcomes; zero executed tests or missing required checks never becomes PASS.

### Evidence production and policy

A receipt binds source/subject and exact revision/content/dirty digest, configuration/features/target, command/tool/producer versions, runner identity, run/attempt, timing, exits/counts, raw reports/digests, protected policy version and evidence links. Verify repository facts independently where possible, as ADR-026 requires; a host receipt is a claim, not unquestioned repository truth. A digest proves content binding, not issuer identity or actual execution.

Authenticate the report producer and its relationship to the authorized execution, not just the final signer/upload step. Correct metadata around attacker-authored JUnit with no execution is non-passing. Substitution, replay across runs/attempts/revisions/configuration, revoked/wrong signer, contradictory exit/count/report and stale output remain rejected/incomplete. Preserve the rejected material safely for audit instead of rewriting it into an accepted extract.

Required tests, workflow/baseline integrity and exceptions come from protected policy outside the evaluated change's unilateral authority. Impact analysis may recommend a smaller suite but cannot remove protected required checks. Changes to implementation, tests, oracle or required policy in the same proposal are exposed separately; changing policy/oracle requires independent authorized review and a new version. Exceptions carry authority, reason, exact scope/expiry and history and do not silently renew during rebuild.

Expose boundary analysis, job status, test outcomes, analysis completeness and final policy eligibility separately. R_eff assesses a scoped evidence chain under its model, not probability of correctness or deployment permission. A renderer, agent role, high score or signed bundle cannot elevate non-passing eligibility. Release bundles retain actual raw failures/interruption/not-started observations as well as relied-on extracts.

### Legacy parity and external effects

Before observing replacement outputs, the migration owner and independent verifier freeze authorized redacted old-system characterization, held-out inputs, boundary/config/runtime/state/effect ledger, time/random controls, tolerances/normalization and approved divergences. Compare output/error/state/effect order/serialization/idempotency/transactions and relevant consumer behavior. Inject meaningful regressions and retain unknown/unreachable dispositions with evidence. Branch counts, banned stubs, syntax agreement, source language or green existing tests are not equivalence proof.

Core builds the dossier and records scoped evidence/eligibility; the external harness performs old/new execution, and the migration/deployment owner authorizes the seam and cutover. Compare Strangler, whole-component and focused refactor against real data/transaction/consumer boundaries, not leaf/SCC topology. Preserve behavior first, then authorize idiomatic refactor with its own delta evidence. PORTING/CONSTRAINTS and isolated worker ownership remain Marketplace methodology/external orchestration; no fixed four-role runtime is added to Core. Compiler/link/runtime/platform and unsafe/FFI checks remain explicit where relevant.

Recovery must be exercised after actual new-version writes/in-flight or retried external effects, with old-reader compatibility and duplicate-effect prevention. Name the point of no return before cutover. Artifact undo is not payment/deploy/data rollback. Where reversal cannot restore effects, the external owner must authorize a declared compensation strategy and scope; no claim of universal rollback is made. If recovery is unknown/failed, acceptance and cutover remain blocked.

## Consequences

### Positive

- Test and parity consumers can tell what was actually proved and which authority may act.
- Protected policy/report-production checks address attacks that hashes or wrapper signatures miss.
- Core can support CI, legacy and release decisions without owning process scheduling or external state.

### Negative and risks

- Trust policy and production-chain integration are operationally demanding and environment-specific.
- Authorized runner compromise is not comprehensively detected by this contract; trust assumptions and incident limits must be disclosed.
- Frozen independent oracles may miss behavior; bounded acceptance must retain unknowns and cannot become universal equivalence.
- Raw failure retention and evidence promotion add privacy/storage/recovery responsibilities.

### Neutral

An authentic run may correctly fail; a valid bundle may be ineligible for acceptance; an eligible Core result may still require human/external deploy approval. Execution, evidence recording and authorization are different outcomes.

## Compliance and evidence gates

EP-4/AC-4 requires a real receipt/report→bundle/eligibility→assurance-read chain, protected-base policy and adversarial zero/skipped/missing/partial/stale/replay/substitution/contradiction/attacker-report-with-trusted-wrapper cases. EP-5 adds actual federated CI/config/consumer integration. EP-6/AC-2 requires independent frozen/held-out oracle, injected regressions and actual post-write recovery; L3 authenticity does not discharge it. EP-7 ensures every introduced consumer preserves these distinctions. EP-1 only proves delivered foundation classes/envelope and must not require future runners/UI.

Before confirmation, E0 names experiment owner and independent verifier and freezes corpus/oracle/configuration, severity/safety, baseline-derived useful effect/full cost, sample/horizon/stopping and exclusions. Missing values BLOCK the experiment. Correlated common-envelope checks or shared runs are not independent corroboration. Final acceptance needs real ADI, validation and independent Critical review; no claimed F+G+R number or native receipt is invented during hold.

## Invariants

No collection execution by implication; no process scheduler in Core; no trusted-wrapper laundering; no digest as signer identity; no head policy self-approval; no missing check as PASS; no declaration/coverage/score as parity; no artifact undo as external compensation; no accepted oracle change merely because code now behaves differently.

## Rollback Plan

Inventory current report/provenance/required-check consumers and external authority owners before integration. Version contracts and policy with preserved old receipt/bundle histories; introduce explicit rejected/incomplete states without reclassifying old unauthenticated reports as execution evidence. Retain raw preimages and promoted extracts, reconcile stable test identities and actual report producers, then prove each introduced chain. Revoke a compromised producer/grant and visibly invalidate dependent eligibility without deleting prior audit history; reexecution requires new authorized run/attempt and independent policy approval. Rolling back an adapter must not turn unknown historical evidence green. External production-state recovery is a separate authorized ledger-based action.

## Revisit Trigger

Any forged/substituted/replayed report accepted, missing required check passing, policy self-weakening, unauthorized discovery execution, signer/runner compromise, surviving material injected regression labeled parity, lost/duplicated post-write effect or failed promised recovery reopens the affected decision immediately. New tool/policy/trust-root/oracle/boundary versions require scoped reevaluation. One event suffices; preserve safe receipts and suspend affected eligibility pending reviewed correction.

## Open questions

Named runner/report/signer/policy owners, grant enforcement technology, trust-root/revocation/retention and independent oracle scope remain OPEN before enablement. All product/runtime/security/parity/recovery experiments are NOT_RUN. This body does not adopt an unaccepted FPV protocol or activate Marketplace EvidencePack drafts.

## References and proposed canonical relationships

- PRD `prd-document-k6a638805c55b95a65262c2e83492a6fe`, SOLUTION `solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d` — based_on; FC-1/3/4/5, FR-19/20/21/23/24/27/30/37/38/39.
- Active Core `adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer`, `adr-storage-classes-for-machine-written-records` — based_on, no supersession.
- `hyp-revision-bound-test-evidence-can-support-k246a4d18955838d13fd19c9fcc1e1d36`, `hyp-a-frozen-independent-oracle-can-expose-k9fef2f8356bc50be55a1f0055af33a37` — informs, unvalidated separate authenticity/parity claims.
- Marketplace `src:003b4e75d638bcca` PRD-081/RFC-026 frozen comparator pilot and PRD-079/ADR-031 EvidencePack drafts — qualified context only; exact keys/status to resolve before links. Pilot is not universal parity; draft is not trusted-host acceptance.


## Publication and upstream admission boundaries

The six allocated ADR drafts retain the complete F-01–34, FR-01–41, T01–17, NFR-01–12, FC-1–6 and AC-1–6 target and the preparation inventory's traceability, without automatic adoption of EPIC-009/FPV-00…15. Corrected EVID-229 is documentary proposed-input admission only. Native reasoning is author analysis, not independent evidence, empirical verification or final Critical approval. Raw/export Critical and any validator Deep classification, provider/exit/parse warnings and last-published-generation caveats belong to actual receipts and remain disclosed.

The preparation publication-manifest.tsv SHA-256 867fdca01fcef9e62418351216163b253925c1bb889f0128287058df902b23c2 and every preparation file remain immutable. Current publication framing replaces the historical noncanonical/disk-hold status; local labels retain traceability only. SPEC-015 is separately published in Product, absent from this Decisions input set, and is not linked as though locally present; successors meet once in the authorized downstream RFC. The future RFC must reconcile exact keys, contracts and dependencies rather than produce a competing SPEC. No absent cross-project record is a native link target.

Core FR-41 research local://analytical-fpv-disposition-research-full.md (SHA-256 589e66bffd7e9e942aedde942eb7c26d6eea5f93dc02f770ba57c9fc100c359d) and Web/Marketplace boundary research local://analytical-consumer-boundary-research.md (SHA-256 1d54e0adeb4e857ef7bc86c0ab4320b0ebb1d3b7c4b86c289354435bc20341b1) are proposed documentary dispositions, not policy overrides or implementation proof. EPIC-009 is unaccepted and PROB-082/FR-41 remains OPEN. Current FPV dependency records agree on 02+03→07; the contrary preserved import is historical. Freeze a reviewed dependency source and resolve compiler/policy bootstrap and later evidence-model/consumer edges before execution; this does not authorize sixteen inherited tasks. Current maintenance/capture/allowlist discrepancy requires current owners' reconciliation, not a precedence guess or demonstrated runtime bug.

## Marketplace policy and pilot boundaries

Core's independent-review/no-defect-quota rule does not override Marketplace's active ADR-006 activation sentinel or active ADR-013 review policy. Marketplace draft ADR-033, even with its supersedes edge and bounded bootstrap clause, needs its own owner-reviewed successor/source-propagation admission; it is not blanket product acceptance. Draft PRD-079/ADR-031 EvidencePack is not current protected-host protocol or readiness proof: the inspected body records blocker reviews and an incomplete adapter. Retain producer≠verifier, independent principal certification, recorder fidelity and Guardian recommendation≠executor activation. clientInfo/args/env/claim aliases are not authenticated independent identity.

Protected-host adoption requires the actual positive execution/evidence/authorization path plus wrong principal/role/workspace, missing parent, mixed cached protocols, replay/conflicting digest, lost acknowledgment, changed reviewed chain/policy, expiry crossing, zero/skipped/required-unavailable and producer self-certification negatives. After local admission, coordinate all actual writers/consumers and versioned rollback; handoff-only cannot count as protected acceptance and shims are not a completed cutover.

Draft Marketplace PRD-081/RFC-026 is an isolated licensed comparator/parity pilot, not a ceiling on the mature legacy target or production cutover. Reuse pinned source/oracle hashes, fresh independent inputs, raw process evidence, meaningful behavioral/mutation canaries and existing Smith route; extend to real state/transactions/errors/effects/serialization/idempotency/configurations/consumers, approved divergences and post-write recovery. PROVEN sample does not imply universal equivalence or authenticated provenance/Guardian PASS. Substituted oracles/results, reused fresh inputs, normalization hiding all differences, empty comparisons, surviving canary, missing callback/consumer/state effects and failed post-write recovery remain failures/unknown in scope. No new migration orchestrator or unapproved production caller change is authorized.

## Preconditions

Before execution-based eligibility: named external runner/report/signer/policy owners, protected required-check and grant/sandbox configuration, retained evidence/promotion contracts, independently admitted producer/verifier authority and local host-policy admission. Parity additionally needs a pre-replacement frozen independent oracle/state-effect ledger, E0 and authorized post-write recovery.

## Postconditions

Actual report-production→bundle→eligibility consumers expose L1–L6 separately and reject required missing/stale/replayed/substituted/contradictory/policy-weakened evidence. Actual scoped parity and post-write recovery need their own independent proof; eligibility never substitutes for deploy/cutover authority or external compensation.

## Affected Files

Only this new ADR file is published. Future evidence/policy/receipt ingestion, external runner/CI/host adapters, bundle/export/assurance readers and legacy harness integration require exact source-file and owner inventories in their downstream RFCs. No existing workflow, trust root, test/oracle, Marketplace policy or production caller is modified here.
