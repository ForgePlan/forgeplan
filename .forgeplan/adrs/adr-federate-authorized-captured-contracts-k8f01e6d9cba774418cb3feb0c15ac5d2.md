---
assigned_number: 37
author: null
created: 2026-11-20
depth: critical
id: ADR-037
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
- target: hyp-pinned-cross-repository-contracts-can-k8afe3a348d93315c9e233dbc689a623b
  relation: informs
parent_epic: null
predicted_number: 37
problem_ref: null
slug: adr-federate-authorized-captured-contracts-k8f01e6d9cba774418cb3feb0c15ac5d2
status: draft
tags: []
title: Federate authorized captured contracts without replacing source owners
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 37
created: 2026-11-20
predicted_number: 37
problem_ref: null
slug: adr-federate-authorized-captured-contracts-k8f01e6d9cba774418cb3feb0c15ac5d2
updated: 2026-11-20
---

# Federate authorized captured contracts without replacing source owners

## Status

D-FEDERATION — Canonical ADR draft / Critical, 2026-11-20. PROPOSED and UNACCEPTED: publication records proposed contracts and open gates, not architecture activation, an A/B or backend choice, implementation authorization or delivered capability. The D-* name is a local traceability convention; the artifact's allocated canonical key/frontmatter is authoritative. No prior ADR is superseded. Active ADR-001/003/025/026 and current independently owned contracts remain binding; new analytical producer exceptions require explicit owner clarification or an approved amendment, not an implied bypass.

This publication succeeds the immutable disk-hold preparation. Corrected canonical EVID-229 (key evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403, SHA-256 865fba7aa30832200f2b7b90b7e75f3044c498319f22307f055c6b0c16167ae3) records one proposed-input round with two independent Sol judgments; it is not final Critical architecture acceptance. Actual native command/ADI/validation exits, provider, parsing and input-generation limitations are recorded under .local/design-adr-publication/, not presumed successful in this body. No F/G/R score or C4 acceptance is fabricated. Runtime/product experiments, builds, activation and downstream RFC remain NOT_RUN.

## Context

The mature product must explain compatibility and required changes across at least three actual repositories and support local/folder/offline operation plus optional collaboration. A registry entry, path, OID or JSON lock does not itself deliver the referenced content to another machine. Independently correct repository results also do not establish a coherent version/configuration combination.

Existing Constellation source qualification and snapshots are foundations to reuse, not proof that every new semantic/CI/team feature is delivered. Core, Web and Marketplace have separate source identities and overlapping display IDs. Private names, paths, code and relationship existence can leak through a graph, context pack, error, chip, hidden report asset or team export even when the visible node body is redacted.

The collision is between useful cross-domain joins and preserving each source's access, version and authority boundaries. Physical single-connection ATTACH cannot stand in for that contract or imply unlimited scale. Optional Hub/Git hosting must not become a hidden dependency of local work or permitted standalone reports.

## Decision drivers

- FC-1/2/3 require qualified identity, exact capture/configuration, authority classes and recovery across sources.
- FR-12/13/14/16/39 require explicit portability, real delivery, coherent federation, compatibility and optional team access/trust.
- NFR-01/02 prohibit unauthorized execution and export, including metadata/hidden assets.
- FR-19/21 require trusted protected CI evidence; contract discovery alone does not prove rollout compatibility.
- Preserve existing independently owned Constellation, current-main read/maintenance and Web/Marketplace contracts.

## Considered options

### A — Query each live repository independently and merge latest results

**Pros:** low upfront capture/delivery coordination; uses current provider interfaces.

**Cons:** can mix revisions, configurations, policies or producer states; unavailable repositories can look like absent consumers; cannot reliably reproduce an acceptance answer or offline view.

**Assessment:** unsuitable for coherent acceptance. Explicit exploratory partial queries may use declared live observations, never label the union as one current complete snapshot.

### B — Implementation-neutral qualified capture with authorized portable materialization

**Pros:** preserves existing identities/owners; supports pinned replay, explicit incomplete sources and multiple backends; permits local operation and bounded team delivery.

**Cons:** capture/compatibility, access-aware traversal and content packaging require real coordination; snapshot availability/retention and revocation cannot be guaranteed merely by hashes.

**Assessment:** proposed logical direction, conditioned on current-owner reconciliation and actual delivery/access/recovery evidence.

### C — Centralize all source facts and mutable team state in a mandatory Hub

**Pros:** a central service can simplify some access and publication coordination; may suit an opted-in deployment.

**Cons:** creates another truth for externally owned state if not constrained; expands breach/availability/deployment boundary; violates local/offline independence if mandatory; copies do not become authorized merely because centralized.

**Assessment:** not a mandatory architecture. An optional service may implement B's public delivery/access contracts without changing Core/source authority.

## Decision outcome

Propose B. Reuse existing qualified keys, registry and snapshot/capture routes with a versioned implementation-neutral federation boundary. A capture identifies each required source, canonical subjects/content, exact revision/dirty digest, configuration/features/target/generated inputs, producer/tool/schema and policy bindings, availability/access state and provenance. It identifies the evaluated source combination, not an unsupported claim of simultaneous atomic capture across arbitrary repositories.

A strict coherent request either obtains a valid declared combination or returns typed incomplete/busy/denied/unsupported/missing-source diagnostics. Optional partial exploration carries per-source coverage and original stale revisions; it cannot answer no-consumers or compatibility PASS from a missing required source. Keep independently expected consumer inventory to detect discovery omissions. The absence of an edge is not proof of independence.

### Two-level detail and contract exchange

Local detailed code/behavior stays under its source owner. Published contract/consumer/evidence views expose only permitted API/schema/event/package/FFI/ABI and behavioral commitments, exact versions/configurations and permitted evidence extracts. Name similarity does not create a verified cross-service edge. Compatibility checks include required fields/defaults, serializers/old messages, backward/forward combinations, unchanged-schema semantic breaks, harmless structural edits and rollout ordering. An unchanged signature is not compatibility proof.

A portable registry/lock/export manifest declares what actual contents must be delivered, how digests/configuration/trust are verified, supported schema versions and every missing dependency. Delivery may use authorized Git objects, permitted bundles or an approved source adapter. Receiving only a path/OID/URL without available required bytes is incomplete, not portable success. Dirty content must be included or explicitly unavailable; a lock pointing only at HEAD cannot stand in for it. Offline evidence replay and a human-viewable report have separate completeness/asset obligations.

Keep current path-derived source identity/qualification semantics until the owning identity/Constellation work decides a portable mapping. Independent clones must not silently merge by repository name or assume one logical source. Relocation/cross-machine continuity requires explicit mapping and alias ambiguity handling; no new universal UUID/alias::id registry is selected here.

### Access and privacy contract

A caller's authority is explicit for source discovery, metadata, content, relation traversal, query/context use, export and later publication. Source adapters cannot elevate that authority. Check permission before admitting restricted facts to query planning, expansion, ranking/snippets/context and export assets, not only before rendering a body. Derived paths/explanations and error messages must not reveal unauthorized names or relationship existence. Retained authorized captures carry policy/version and distribution scope; subsequent use is checked under the applicable current authorization and declared offline policy.

Where a permitted consumer may know incompleteness but not restricted metadata, return a non-disclosing incomplete result rather than identifying the denied object. Security/privacy owners define this observable denial/redaction matrix before enablement; partial disclosure cannot masquerade as no consumers or allow acceptance. Do not expose a continuation token, hidden graph layer, browser payload, embedded asset or telemetry field that restores denied details. Agent contexts and UI focus/pinning follow the same rights. Editor URIs cannot grant local execution or filesystem access through the Web adapter.

Access to a source does not grant redistribution. Before ingestion/export, the privacy/deployment owner determines jurisdiction, data classification, licenses/notices, allowed recipients/content/assets, retention and revocation. Revoking a grant blocks future service access/update/publication under that policy; already delivered copies cannot be promised physically recalled. Offline expiry/trust status must be explicit, not hidden network dependence or an impossible revocation guarantee.

### Deployment, update and failure boundaries

Local/offline/folder federation works without Hub or Git hosting. Opted-in collaboration/Hub/Gitea/Forgejo adapters have their own identity, authorization, trust, delivery, update and recovery acceptance. Their tracker/assignment state remains externally owned under ADR-025/026. Desktop watchers/file/editor actions require local capabilities; Web rights do not imply those privileges.

A/B kernels may use bounded local joins, partition queries, transport or materialized views. Backend limits, concurrency, repository count/size and retention must be measured on a declared matrix. Over-limit is explicit incomplete/unsupported or a bounded alternative plan, not silent omission. SSE/version-hash/polling choice stays empirical; duplicate/lost/out-of-order updates and reconnect cannot mix captured generations. The result remains the same qualified query semantics across HTTP/IPC/offline representations.

## Consequences

### Positive

- Federation can evolve independently of physical backend and optional team deployment.
- Source owners retain truth/access responsibility while consumers receive reproducible permitted content.
- Missing/denied/version-incompatible sources remain visible without false negative consumer claims.

### Negative and risks

- Access-aware joins, redaction and non-disclosing errors complicate query planning and testing.
- Portable materialization adds licensing, storage, retention and recovery costs; remote references may expire.
- Global completeness is bounded by the declared independent inventory and available authorized sources, not universally provable.
- Offline copies limit revocation; a service cannot retroactively guarantee deletion on an untrusted recipient.

### Neutral

A permissioned team service may centralize delivery, not canonical source ownership. ATTACH, JSON or .sexp export, SSE and UI graph layout remain mechanisms, not the federation semantics.

## Compliance and evidence gates

Before affected P1 acceptance: current source-identity/read-maintenance owners reconcile contracts with protected main artifacts. P1 portability fixtures prove foundation only. Before P5/AC-1/EP-5: actual delivered sources/contracts from three or more real authorized repositories→federated query→compatibility/review plan/CI, independent consumer/version/configuration/rollout oracle, all missing/denied/over-limit/mixed/stale and unchanged-schema-break cases. EP-4 must separately establish authentic report eligibility.

Before P7/EP-7 optional distribution and surface readiness: real access/redaction/revocation/update/recovery and standalone report asset proof at declared scale, plus same-envelope consumer semantics. Earlier foundation fixtures do not discharge actual federation/UI/Hub paths. E0 requires named experiment owner and independent verifier, exact matrix/corpus/threshold/horizon before confirmation; missing values block runs. No performance, privacy or integration experiment is reported as executed here.

## Invariants

No bare display ID as global address; no latest union as coherent capture; no missing/denied source as no consumers; no unbounded ATTACH; no optional Hub as local prerequisite; no export merely because data was readable; no renderer-only redaction; no tracker/CI state copied into a second mutable truth; no source-adapter execution permission by implication.

## Rollback Plan

Inventory current qualification, snapshot handles, source adapter and consumers with their owners. Add versioned contract deltas, permitted content delivery and explicit relocation mappings without rewriting history or aliases in bulk. Verify old consumers and pinned captures, access/denial behavior, actual bundle replay and recovery before switching. On failure preserve old captures/canonical records and withdraw affected publication rights or feature eligibility; do not synthesize a complete graph. Hub removal must leave permitted local/offline materializations usable within their promised scope. External hosting/tracker/deploy compensation remains its owner's authorized action.

## Revisit Trigger

Any private name/path/content/relationship leak, mixed-generation compatibility result, missed independent consumer, failed remote-byte delivery, replay/relocation ambiguity, measured backend limit or changed grant/license/producer trust reopens the affected decision. Verify and retain the qualified counterexample without re-exporting secret material. One event suffices; suspend affected claims and obtain owner-reviewed correction before reenabling.

## Open questions

Portable identity mapping, exact capture protocol and bounded backend strategy, transport, per-deployment access/retention/revocation policy and team topology are OPEN. FR-41/PROB-082 reconciliation and EPIC-009 placement remain coordinator gates; EPIC-010 and RFC-022 are protected dependencies, not absorbed or accepted by this draft.

## References and proposed canonical relationships

- PRD `prd-document-k6a638805c55b95a65262c2e83492a6fe`, SOLUTION `solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d` — based_on; FC-1/2/3/6, FR-09–16/19/21/22/36/39.
- `hyp-pinned-cross-repository-contracts-can-k8afe3a348d93315c9e233dbc689a623b` — informs, unvalidated.
- Core `src:c3baf53b9bec0501`: `epic-constellation-contextpack`, draft `adr-constellation-manifest`, qualified identity/snapshot and bounded-context contracts — reuse/current-owner reconciliation, not readiness acceptance.
- Active Core `adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer` and `adr-storage-classes-for-machine-written-records` — based_on authority/ownership; no supersession.
- Web `src:61600c6e7e40d4b0` and Marketplace `src:003b4e75d638bcca` — source-qualified map/daemon/tracker/EvidencePack/pilot context. Resolve exact current keys/status before native links; display numbers are not invented keys.


## Publication and upstream admission boundaries

The six allocated ADR drafts retain the complete F-01–34, FR-01–41, T01–17, NFR-01–12, FC-1–6 and AC-1–6 target and the preparation inventory's traceability, without automatic adoption of EPIC-009/FPV-00…15. Corrected EVID-229 is documentary proposed-input admission only. Native reasoning is author analysis, not independent evidence, empirical verification or final Critical approval. Raw/export Critical and any validator Deep classification, provider/exit/parse warnings and last-published-generation caveats belong to actual receipts and remain disclosed.

The preparation publication-manifest.tsv SHA-256 867fdca01fcef9e62418351216163b253925c1bb889f0128287058df902b23c2 and every preparation file remain immutable. Current publication framing replaces the historical noncanonical/disk-hold status; local labels retain traceability only. SPEC-015 is separately published in Product, absent from this Decisions input set, and is not linked as though locally present; successors meet once in the authorized downstream RFC. The future RFC must reconcile exact keys, contracts and dependencies rather than produce a competing SPEC. No absent cross-project record is a native link target.

Core FR-41 research local://analytical-fpv-disposition-research-full.md (SHA-256 589e66bffd7e9e942aedde942eb7c26d6eea5f93dc02f770ba57c9fc100c359d) and Web/Marketplace boundary research local://analytical-consumer-boundary-research.md (SHA-256 1d54e0adeb4e857ef7bc86c0ab4320b0ebb1d3b7c4b86c289354435bc20341b1) are proposed documentary dispositions, not policy overrides or implementation proof. EPIC-009 is unaccepted and PROB-082/FR-41 remains OPEN. Current FPV dependency records agree on 02+03→07; the contrary preserved import is historical. Freeze a reviewed dependency source and resolve compiler/policy bootstrap and later evidence-model/consumer edges before execution; this does not authorize sixteen inherited tasks. Current maintenance/capture/allowlist discrepancy requires current owners' reconciliation, not a precedence guess or demonstrated runtime bug.

## Cross-source migration and policy gates

Owner-reviewed Web/Marketplace deltas must cover map typed relations and source/revision/configuration provenance, qualified TaskLink migration and actual HTTP/IPC/offline consumers. Web unknown-relation acceptance conflicts with Marketplace's closed eleven-relation guardian and four-extra-key invariant; neither side is overridden by Core graph semantics. Rollout must version mandatory meaning and prove backward/forward behavior through real emitter/guardian/GET/validator/renderer paths. Preserve permitted derived tracker checklists versus authoritative-body duplication by explicit agreement; no bare ID/name backfill overwrites non-null links, task history or operator authority.

Team/host trust must obtain local policy admission. Marketplace draft PRD-079/ADR-031 and draft ADR-033 do not replace active ADR-006/ADR-013 by installation, a supersedes edge or Core activation. Protected-host success needs actual independently authorized principal/producer/verifier binding and local migration/replay/version rollback; handoff-only is not successful protected-host acceptance. FailedFields, wrong workspace/role, missing parent, stale/replayed chain, expiry crossing, lost acknowledgment and changed reviewed policy remain non-passing. The report's four certified external keys are documentary references only here; absent local records are not native targets.

## Preconditions

Before federation/team enablement: source/identity/Constellation owners admit capture and actual content-delivery contracts, security/privacy define non-disclosing access/export/retention policy, consuming owners approve schema/TaskLink deltas and separately authorize work. E0 fixes real repository/consumer/version/configuration matrix and independent verifier before confirmation.

## Postconditions

Actual delivered-source→federated-query→compatibility/CI and introduced team/offline/surface paths preserve qualified capture, coherent combinations, rights and explicit missing/denied/over-limit states. Local operation remains independent of optional Hub; actual consumer/rollout and recovery evidence precedes acceptance.

## Affected Files

Only this new ADR is changed here. Future qualified source/capture/delivery/federation/access/export and opted-in service or client consumers require an exact current-owner/source-file census in the downstream RFC. No existing registry, protected Constellation artifact, Web map, tracker state or deployment policy is edited now.
