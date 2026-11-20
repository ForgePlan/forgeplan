---
assigned_number: 35
author: null
created: 2026-11-20
depth: critical
id: ADR-035
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
- target: adr-storage-classes-for-machine-written-records
  relation: based_on
- target: ADR-003
  relation: based_on
- target: hyp-sqlite-can-reduce-delivery-cost-while-k1b935932d0d098ef732e330d91e77872
  relation: informs
parent_epic: null
predicted_number: 35
problem_ref: null
slug: adr-preserve-versioned-authority-and-durable-k0c75d791cf683ce6ce085df9affa0dd1
status: draft
tags: []
title: Preserve versioned authority and durable evidence independently of projection backends
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 35
created: 2026-11-20
predicted_number: 35
problem_ref: null
slug: adr-preserve-versioned-authority-and-durable-k0c75d791cf683ce6ce085df9affa0dd1
updated: 2026-11-20
---

# Preserve versioned authority and durable evidence independently of projection backends

## Status

D-AUTHORITY — Canonical ADR draft / Critical, 2026-11-20. PROPOSED and UNACCEPTED: publication records proposed contracts and open gates, not architecture activation, an A/B or backend choice, implementation authorization or delivered capability. The D-* name is a local traceability convention; the artifact's allocated canonical key/frontmatter is authoritative. No prior ADR is superseded. Active ADR-001/003/025/026 and current independently owned contracts remain binding; new analytical producer exceptions require explicit owner clarification or an approved amendment, not an implied bypass.

This publication succeeds the immutable disk-hold preparation. Corrected canonical EVID-229 (key evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403, SHA-256 865fba7aa30832200f2b7b90b7e75f3044c498319f22307f055c6b0c16167ae3) records one proposed-input round with two independent Sol judgments; it is not final Critical architecture acceptance. Actual native command/ADI/validation exits, provider, parsing and input-generation limitations are recorded under .local/design-adr-publication/, not presumed successful in this body. No F/G/R score or C4 acceptance is fabricated. Runtime/product experiments, builds, activation and downstream RFC remain NOT_RUN.

## Context

The original ADR-003 inverted authority from Lance to versioned files. ADR-026 amends its markdown-only wording to house machine-issued contracts, bundles and verdicts, separates local raw material, and leaves externally owned state external. Neither decision permits deleting every database as disposable: the actual stored class determines authority and recovery obligations.

The collision is between rebuilding analytical projections cheaply and preserving the decision history, accepted code links, exceptions, report extracts and execution provenance needed after a machine or provider disappears. A graph representation, database engine or export extension cannot settle that authority question. Current capture/maintenance work is independently owned and may differ from inherited draft ADR-029. The known maintenance allowlist/document discrepancy is an owner reconciliation gate, not a demonstrated runtime defect.

## Decision drivers

- FC-3/AC-5 require per-class authority, identity/history, expiry, retention and recovery, not merely a successful search rebuild.
- FC-1 binds every published/read/exported result to exact inputs; atomic visibility and crash durability are separate obligations.
- FR-01/05/06/20/31 require measured backend choice, actual caller migration, preserved query/search semantics and bounded authorized operations.
- FR-35/36 require factual docs and standalone permitted reports without overwriting normative/manual content or assuming external services stay available.
- No SQLite migration, unlimited ATTACH, universal event bus or .sexp winner is authorized by the full-product requirement.

## Considered options

### A — All state authoritative in one database, documents as projections

**Pros:** a single engine may simplify local transactions and structured queries; rich records fit naturally.

**Cons:** contradicts active versioned-file authority; clone/review/recovery and cross-machine trust would need replacement contracts; external tracker/CI truth would be duplicated if copied as mutable records.

**Assessment:** not adopted. Reconsidering authority requires a separate reviewed amendment, not a backend swap.

### B — Preserve authority classes and publish rebuildable projections coherently

**Pros:** reuses ADR-003/026; supports independent index implementations; makes durable promotion and external references explicit.

**Cons:** canonical operation and projection protocols must cooperate without claiming cross-store atomicity they lack; promotion/retention and crash recovery require real operational ownership.

**Assessment:** proposed logical direction. Physical backend remains undecided.

### C — Everything in tracked files, including all raw runs and external state

**Pros:** broad clone/offline availability; simple file-based access for agents.

**Cons:** high-volume raw data/secret retention and merge pressure; violates one owner per external state; does not authenticate reports or define a safe publication protocol.

**Assessment:** reject as universal rule. Preserve selected durable extracts through authorized promotion rather than indiscriminate copying.

## Decision outcome

Propose B, explicitly following the existing authority amendment. Introduce no second writer: accepted links/policies/exceptions and machine records are written through authorized canonical operations. Derived projections never rewrite their source to match observed code or a cache.

### Authority and recovery ledger

| Class | Authority and mutation | History/expiry/retention | Recovery and verification |
|---|---|---|---|
| Human-authored PRD/ADR/SPEC/scenarios, accepted code links/policies/exceptions | Versioned canonical files, existing canonical operations and human normative ownership | Version history and provenance retained; exception expiry is data, not silently extended by rebuild | Restore canonical versions and references; verify content/key/link/expiry census before rebuilding projections |
| ADR-026 Class A WorkContract, EvidenceBundle, VerificationVerdict | Tracked schema-validated machine records; contract/verdict JSON and EVID structured section plus prose as designated by ADR-026; append-only versions, digest links | Supersession/revocation append history, never in-place rewriting of prior issued records; retention policy approved before operation | Clone/authorized bundle restores relied-on extracts and links; verify schema/digests/issuer/trust/policy and exact subject; malformed/unavailable evidence remains non-passing |
| ADR-026 Class B raw execution receipts and existing journal | Per-machine raw material, not global authority; controlled append/access | Valuable local material is not automatically safe to delete. Retain until required promotion/verification/recovery dependencies are resolved; exact horizon and quota owned by operations/privacy before collection | Preserve recoverable raw streams and terminal/interrupted states; promote the relied-on extract plus receipt digest into Class A before disposal; a lost unpromoted receipt is unavailable, not reconstructed as success |
| ADR-026 Class C CI/deploy/tracker state | External owner; reference/digest and observation time in Class A; no second mutable truth | Snapshot observation and explicit freshness; external retention guarantees declared | Re-query authorized owner where available or inspect retained frozen extract with its scope. It proves an observation, not current external state or renewed authorization |
| Graph/lexical/vector/search/score projections | Declared deterministic or explained derived computation over captured sources/configuration | Generation history/eligibility explicit; retention must protect active readers and recovery; model/dimension/body/layout/profile digests retained | Rebuild declared sources, verify candidate completeness, publish coherent generation; never delete canonical/local durable classes while resetting index |
| Saved/issued reports and offline materializations | Derived at creation; designated retained report/extract becomes durable evidence of that captured result, not a new mutable source of normative intent | Owner records access/redaction, query/capture/policy, expiry and verification horizon; manual annotations preserve their owner | Restore authorized content/assets and referenced extracts needed for declared offline viewing/replay. Missing dependencies disclosed; no promise to physically revoke already exported copies |

Each enabled deployment must instantiate this ledger with a named responsible owner, exact locations/identity, access policy, retention/expiry, backup/restore and deletion authority. Numeric horizons remain TBD pending privacy/jurisdiction and operational decision; that uncertainty blocks relevant collection/disposal acceptance, not the full mature scope. Schema-defined append-caps must preserve required trust history by reviewed compaction/export, not erase relied-on transitions.

### Publication and read contract

Capture declared canonical sources/configuration without treating mtime/HEAD/watch events as content identity. Build and verify a candidate generation before visibility changes. Serialize cooperating publication through one declared owner of the physical projection; source mutations and index maintenance retain their own authority boundaries. Recheck candidate input bindings and publication preconditions before switching visibility. A failed/incomplete scan is not deletion or a valid empty baseline.

Readers hold one captured immutable view with exact generation/input bindings. Persisted and permitted ephemeral reads must not mix live/latest rows, stale facts or independent file rereads into a current-labeled result. Last-good facts keep original revision and explicit stale status. A read cannot gain maintenance, download, execution or write rights merely to obtain freshness; current protected maintenance/capture owners must reconcile the precise operation allowlist before affected acceptance.

Atomic publication establishes a visibility point, not power-loss durability by itself. The future backend/platform protocol must define write completion, durable closure, pointer/manifest visibility, acknowledgment, interrupted/uncertain publication, restart recovery and reader retention. Acknowledgment after an uncertain switch cannot be faked as success or blindly retried. Preserve the previous usable generation and pending canonical changes; never delete an open database or take a live lock based on PID/timeout alone.

### Orthogonal backend and representation gate

Compare retained Lance with necessary improvements, storage-only SQLite and justified alternatives under HYP-006. Evaluate RU/EN/identifier relevance, exact current search behavior and any ranking delta, vector provenance, cold/incremental/full query cost, concurrency/held readers, crash/rebuild/schema/restore, federation scale/limits, binary/build/CI and full migration/parallel/maintenance horizon. WAL alone does not solve stale held transactions. ATTACH is only a bounded candidate, not the federation contract.

Logical graph meaning is D-GRAPH; physical graph/backend is OPEN; exchange/transport is a separate versioned contract; UI map/2D/3D is a representation. `.sexp` may be considered as an encoding only with justified reader/schema/evolution/diff/streaming/security/size/replay evidence; it is not an assumed authoritative or storage winner. Ranking weighted-max versus RRF is another independent comparison. Model/dimensions/normalization/body digest mismatch cannot become a valid embedding because the backend changed.

## Consequences

### Positive

- Existing truth ownership survives analytical growth and backend experimentation.
- Per-class recovery prevents a successful index rebuild from masking lost evidence/history.
- Document generation can explain observed facts while preserving normative and manual ownership.

### Negative and risks

- File/canonical operation, local receipt and projection protocols increase coordination and operational work.
- Offline verification requires retained authorized material; references alone may expire or become inaccessible.
- Retention/privacy conflicts cannot be solved by keeping everything forever; deletion may reduce future verification scope and must be explicit.
- Backend migration can fail despite faster queries if history, relevance, crash durability or total cost regress.

### Neutral

Immutable machine records can contain a human explanation; derived reports can be durably retained without becoming normative authority. External compensation remains outside artifact history.

## Compliance and evidence gates

Before P1 execution: reviewed FR-41/program disposition, accepted authority/identity/read-capture contracts and current owner reconciliation. EP-1/AC-5 proves actual foundation record/reference→build/publish→captured read→snapshot/export, deliberate FC-1 mismatches, failed publish/crash/schema/live-reader/restore and every durable class. It cannot depend on future P3/P5/P7 delivery and cannot discharge their actual integration proofs.

Before backend selection/cutover: named storage experiment owner and independent verifier, complete class/caller inventory, E0 baseline/corpus/threshold/horizon, side-by-side compatibility and recovery. Missing owners/thresholds block confirmation. No benchmark, native ADI or F+G+R score is fabricated. Final acceptance still needs genuine reasoning, validation and whole-chain independent evidence; structural validation is not recovery proof.

## Invariants

No authoritative DB by accident; no raw receipt silently promoted to trusted evidence; no destructive cache reset of durable classes; no external state copied as a second mutable authority; no generated observed documentation rewriting intended requirements or manual explanations; no policy/exception expiry lost in compaction; no normal read acquiring extra rights through maintenance fallback.

## Rollback Plan

Before any future migration, inventory all stored classes and actual production callers, preserve canonical preimages and local material, freeze compatibility and rollback protocol, and verify restoration before cutover. Build a separate eligible projection or versioned candidate using the backend's proven isolation mechanism; do not mandate full copies per generation. Migrate every real caller as one reviewed cutover with old readers accounted for, not unused ports or long-lived dual authorities. Retain previous usable state until recovery/retention gates permit retirement. Unknown or failed restoration blocks migration acceptance. Undo of a canonical artifact does not undo payments/deployments/new writes; only the external owner may authorize a state/effect-aware rollback or compensation.

## Revisit Trigger

Any canonical/history/expiry loss, mixed current generation, falsely acknowledged crash-durability claim, read-path privilege expansion, failed offline verification within promised retention, or measured backend criterion failure reopens the affected decision. A new workload/license/platform or current-owner contract change also requires reevaluation before expansion. One event suffices; retain the receipt and suspend the affected claim, not silently relax its contract.

## Open questions

Backend/physical schema/ranking/encoding remain OPEN; per-deployment retention/jurisdiction, exact recovery ACK protocol and current maintenance/capture allowlist require named owner review. Local durable records are not all rebuildable cache. No protected main document or prior receipt changes are authorized here.

## References and proposed canonical relationships

- PRD `prd-document-k6a638805c55b95a65262c2e83492a6fe`, SOLUTION `solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d` — based_on.
- Core active ADR-003 and `adr-storage-classes-for-machine-written-records` — proposed refines operational analytical obligations, with no supersession and no change to class homes/formats; `adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer` — based_on ownership.
- `hyp-sqlite-can-reduce-delivery-cost-while-k1b935932d0d098ef732e330d91e77872` — informs, unvalidated backend gate.
- Core draft `adr-constellation-manifest` — context, not activated/readiness proof. Current main ADR-032/PRD-092/RFC-021/SPEC-013 and EPIC-010/RFC-022 remain protected owner dependencies.


## Publication and upstream admission boundaries

The six allocated ADR drafts retain the complete F-01–34, FR-01–41, T01–17, NFR-01–12, FC-1–6 and AC-1–6 target and the preparation inventory's traceability, without automatic adoption of EPIC-009/FPV-00…15. Corrected EVID-229 is documentary proposed-input admission only. Native reasoning is author analysis, not independent evidence, empirical verification or final Critical approval. Raw/export Critical and any validator Deep classification, provider/exit/parse warnings and last-published-generation caveats belong to actual receipts and remain disclosed.

The preparation publication-manifest.tsv SHA-256 867fdca01fcef9e62418351216163b253925c1bb889f0128287058df902b23c2 and every preparation file remain immutable. Current publication framing replaces the historical noncanonical/disk-hold status; local labels retain traceability only. SPEC-015 is separately published in Product, absent from this Decisions input set, and is not linked as though locally present; successors meet once in the authorized downstream RFC. The future RFC must reconcile exact keys, contracts and dependencies rather than produce a competing SPEC. No absent cross-project record is a native link target.

Core FR-41 research local://analytical-fpv-disposition-research-full.md (SHA-256 589e66bffd7e9e942aedde942eb7c26d6eea5f93dc02f770ba57c9fc100c359d) and Web/Marketplace boundary research local://analytical-consumer-boundary-research.md (SHA-256 1d54e0adeb4e857ef7bc86c0ab4320b0ebb1d3b7c4b86c289354435bc20341b1) are proposed documentary dispositions, not policy overrides or implementation proof. EPIC-009 is unaccepted and PROB-082/FR-41 remains OPEN. Current FPV dependency records agree on 02+03→07; the contrary preserved import is historical. Freeze a reviewed dependency source and resolve compiler/policy bootstrap and later evidence-model/consumer edges before execution; this does not authorize sixteen inherited tasks. Current maintenance/capture/allowlist discrepancy requires current owners' reconciliation, not a precedence guess or demonstrated runtime bug.

## Current-owner and local-policy admission

No authority/back-end delta is selected by these drafts. Active ADR-003 remains as already amended by ADR-026; physical backend change requires its own measured decision and class/caller/recovery proof. Preserve ADR-015/016 per-call workspace responsibility and current protected main maintenance/capture contracts pending owner reconciliation. A read-only capture requirement cannot silently negate a currently authorized explicit maintenance operation, and a maintenance allowlist cannot silently grant writes or downloads to every read. Resolve the exact boundary with current owners before affected P1 execution/acceptance, preserving the known documentary discrepancy as OPEN rather than reporting a runtime defect.

Marketplace EvidencePack successors remain draft/local-policy-gated. Core issuance of a record, activation eligibility or installed protocol does not authorize a Marketplace protected-host cutover, rewrite its active activation sentinel/reviewer policy, or substitute Core history for tracker/CI authority. Retention/promotion/replay contracts must survive both producer and consumer migrations, including old version rollback and expiry crossing. Core no-defect-quota policy is not a cross-project policy override.

## Preconditions

Before implementation/cutover: accepted authority/class and current read-maintenance reconciliation, complete durable/local/external inventory, responsible retention/privacy/recovery owners and separately authorized source work. Any backend selection additionally requires named E0 owner/verifier and frozen full-horizon comparison criteria.

## Postconditions

Actual foundation write/reference→publish→captured read→snapshot/export and class-specific crash/schema/live-reader/restore evidence preserves identity/content/history/expiry and coherent old/new visibility. Future consumers prove their own integration; an index rebuilt without durable evidence/history is not recovery success.

## Affected Files

Only this new ADR file is canonical publication scope. Future canonical-operation/storage/generation/capture/search/export and local-receipt promotion callers require an exact inventory in the downstream RFC, including every real writer/reader. No existing authoritative data, index backend, retention policy or source file is migrated here.
