---
assigned_number: 33
author: null
created: 2026-11-20
depth: critical
id: ADR-033
kind: adr
links:
- target: prd-document-k6a638805c55b95a65262c2e83492a6fe
  relation: based_on
- target: solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d
  relation: based_on
- target: evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403
  relation: based_on
- target: adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer
  relation: based_on
- target: ADR-001
  relation: based_on
- target: hyp-integrated-traceability-improves-complete-k4a105d2bef910728a27bd97e53ab62dd
  relation: informs
- target: hyp-target-users-prefer-an-integrated-decision-kb9a06cdf4b8513a2e5e3da335c631ae3
  relation: informs
parent_epic: null
predicted_number: 33
problem_ref: null
slug: adr-own-cross-domain-rust-analytics-keep-kf1722a1f4ac7e2bfa083f25edf92c62e
status: draft
tags: []
title: Own cross-domain Rust analytics; keep normalization degree conditional
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 33
created: 2026-11-20
predicted_number: 33
problem_ref: null
slug: adr-own-cross-domain-rust-analytics-keep-kf1722a1f4ac7e2bfa083f25edf92c62e
updated: 2026-11-20
---

# Own cross-domain Rust analytics; keep normalization degree conditional

## Status

D-KERNEL — Canonical ADR draft / Critical, 2026-11-20. PROPOSED and UNACCEPTED: publication records proposed contracts and open gates, not architecture activation, an A/B or backend choice, implementation authorization or delivered capability. The D-* name is a local traceability convention; the artifact's allocated canonical key/frontmatter is authoritative. No prior ADR is superseded. Active ADR-001/003/025/026 and current independently owned contracts remain binding; new analytical producer exceptions require explicit owner clarification or an approved amendment, not an implied bypass.

This publication succeeds the immutable disk-hold preparation. Corrected canonical EVID-229 (key evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403, SHA-256 865fba7aa30832200f2b7b90b7e75f3044c498319f22307f055c6b0c16167ae3) records one proposed-input round with two independent Sol judgments; it is not final Critical architecture acceptance. Actual native command/ADI/validation exits, provider, parsing and input-generation limitations are recorded under .local/design-adr-publication/, not presumed successful in this body. No F/G/R score or C4 acceptance is fabricated. Runtime/product experiments, builds, activation and downstream RFC remain NOT_RUN.

## Context

The corrected PRD requires an own Rust analytical core connecting engineering obligations, source/configuration-aware facts, cross-service contracts, tests and evidence. A thin wrapper around language tools or an import graph cannot satisfy this target. Conversely, owning every compiler, resolver, test runner and coverage engine would add responsibility without satisfying an independent product requirement.

The architectural collision is between a uniform model that makes cross-domain reasoning tractable and language-native models that retain distinctions a uniform ontology may lose. The frozen SOLUTION explicitly admits both as full-target candidates. Neither has measured accuracy, operating-cost or performance superiority. The existing system's Lance coupling and weighted-max search are attributed baseline observations, not proof that backend replacement is necessary.

ADR-025 fixes a separate ownership boundary: graph/contracts remain in Core; agent scheduling, processes, worktrees and budgets do not. Read-only dispatch is Core planning, not execution. Estimate/calibrate and playbook runtime require their own extension migration RFC and deprecation window; no new analytical requirement silently cancels that decision.

## Decision drivers

- Own Rust computation must include cross-domain identity, relation meaning, reconciliation, provenance, query explanations and evidence eligibility; reusable tools remain sources rather than the entire product (PRD Goals G2, FC-1/2/4/6).
- Full FR-01–41, NFR-01–12 and AC-1–6 remain required. Deep behavior, wider/on-demand languages, legacy parity, documentation, offline operation, Web/Tauri, list/diff/2D/3D and optional collaboration are not scope reserves to cut.
- Semantic fidelity, privacy, completeness and recovery are gates; a faster or cheaper candidate cannot compensate for their failure.
- Existing qualified source/snapshot and ownership contracts must survive; backend and transport are independent choices.
- Independent expected facts and consumers must expose correlated omissions, not merely agreement between two computations using the same faulty model.

## Considered options

### A — Common normalized model and Rust query kernel

Producers supply versioned scoped facts; Core owns normalized typed relations, lineage, invalidation, traversals, placement/alignment, obligation/evidence linkage and captured-result explanations. Language-specific inference is reused where appropriate.

**Pros:** common relation semantics make cross-domain joins and explanations explicit; common invalidation/publication can reduce coordination boundaries as a design inference; surfaces can consume one query contract.

**Cons:** maintaining mappings and an evolving semantic ontology is substantial permanent work; normalization may erase language-specific distinctions; a common resolver defect can corrupt every downstream surface together. These are not measured advantages or costs.

**Assessment:** provisional direction, NEEDS MORE DATA. Accept only if independent semantic/completeness and whole-path conformance preserve the full target without unacceptable cost under E0.

### B — Compositional Rust kernel with native analytical partitions

Language engines retain rich native facts and selected local queries in versioned partitions. Core still performs its own cross-domain analysis, identity/provenance reconciliation, query planning, contract/scenario/test linkage, placement/alignment, reading routes, dossier construction and evidence eligibility.

**Pros:** preserves native language distinctions; permits independently upgraded engines; may avoid premature normalization of complex types/effects.

**Cons:** coherent capture across engines and cross-engine completeness become Core responsibilities; some providers lack suitable snapshot/offline APIs; heterogeneous licensing, installation and query contracts increase upgrade and operational complexity. A missing provider capability must be implemented or replaced, not converted into a hidden feature cut.

**Assessment:** genuine full-target alternative, NEEDS MORE DATA. Prefer B if A materially loses required semantics or exceeds the preregistered full-cost criterion and B passes capture, privacy, completeness, explanation and offline gates.

### C — Current tooling plus manual or external-analyzer integration

Use search/LSP/manual review, or current ForgePlan with pinned external tools, as experimental controls.

**Pros:** realistic alternatives for task usefulness and investment cost; may outperform an integrated platform on the tested work.

**Cons:** not an architecture demonstrating all mature invariants and consuming surfaces; manual discipline does not establish durable linkage, common eligibility, coherent generations or full integration acceptance.

**Assessment:** retain as investment controls, not full-target conformance. A favorable control result can challenge investment; only the product owner can authorize a changed target.

## Decision outcome

Propose own Rust cross-domain analytics with explicitly separated producer, query-result, canonical-operation and external-executor boundaries. Use A as the dependent draft's working arrangement while retaining B as an unresolved selection. Do not encode A-only physical storage assumptions in producer/result contracts. This does not require a universal trait hierarchy: future RFC must inventory actual replaceable production callers and introduce only boundaries with real consumers (FR-05).

Core owns facts/identity/graph queries, policy interpretation, evidence eligibility, lifecycle and read-only planning. Language/framework producers own scoped language analysis and declared capabilities. Marketplace owns methodology/client packs and extension estimation/calibration. External orchestrator/runner owns processes, sandboxing, execution and budgets; tracker owns WHO/WHEN. Web owns browser/2D/3D, Desktop its local file/watcher/editor permissions, and an optional service owns opt-in team delivery. Their state is not duplicated as mutable Core truth.

This choice fixes responsibility, not superiority or exact placement of every local traversal. Backend, physical graph layout, ranking, export encoding, HTTP/IPC transport and renderer remain orthogonal. Neither SQLite nor .sexp is selected. No own compiler/coverage implementation is mandated.

## Consequences

### Positive

- Cross-domain behavior has an accountable owner rather than falling between adapters.
- A/B can be compared against identical required observable results and source/configuration binding.
- Existing process/extension/tracker boundaries remain enforceable as the analytical scope grows.

### Negative and risks

- Both candidates entail substantial integration, compatibility and maintenance work; ownership does not establish feasibility.
- A may correlate wrong answers; B may correlate omissions across fragmented snapshots. Independent oracles remain necessary.
- Keeping selection conditional requires the RFC to avoid accidentally freezing one implementation through public DTOs or deployment assumptions.

### Neutral

Language engines may perform more computation under B without owning normative requirements or acceptance authority. Local work and authorized offline reports cannot depend on optional Hub deployment under either candidate.

## Compliance and evidence gates

Before final architecture acceptance: retain published proposed-input EVID-229; complete genuine ADI on final candidate/context; independent architecture/system/security review; preserve raw Critical versus validator Deep disclosure and the prior last-published-generation ADI warning. EVID-228 concerns PRD admission, not SOLUTION acceptance.

Before confirmatory comparison: named experiment owner and independent verifier, E0 corpus/oracle/config/license/access matrix, baseline-derived useful-effect/full-cost criteria, justified sample/horizon and non-compensable safety conditions. Missing owners or thresholds BLOCK the experiment; no numeric score or F+G+R total is invented here.

Required discriminators: independent positive/negative/unknown semantic corpus; cross-domain placement/alignment and mandatory-reading constraints; partial/crash/concurrent capture; real three-repository consumers; permitted offline materialization; full upgrade/migration/parallel/recovery/operating cost. P1 proves foundation only; P2–P7 each prove their actual introduced paths. Earlier contract fixtures do not waive those exits.

## Invariants

No adapter-only ceiling; no total-program-understanding claim; no process spawning by implication; no estimation ownership move into Core; no score-as-deploy-authority; no absent edge as proven independence. Dispatch must retain file and semantic conflict uncertainty across buckets. All seven FR-04 outcome families remain: drift, dispatch, route/calibrate, guard, blindspots, estimate and context, with their assigned owners.

## Rollback Plan

No implementation change occurs in this preparation. Future A↔B changes use a reviewed capability/identity/result compatibility delta, retained raw provenance and canonical history, side-by-side output comparison and actual old-reader/export/replay/recovery proof. Do not migrate source identity or backend as an incidental A/B change. External side effects require their own owner's cutover/compensation authority, not artifact undo. Existing protected programs and claims are reconciled before new execution tasks.

## Revisit Trigger

Reopen on any material lost native fact, false resolved relationship, silent missing consumer, incoherent cross-engine capture, inability to provide authorized offline results, changed provider license/trust or failed preregistered useful-effect/full-cost criterion. One confirmed event suffices; do not wait for all events. Preserve the counterexample and affected profile; suspend the affected acceptance claim pending reviewed correction or a separately authorized replacement decision.

## Open questions

A/B normalization degree, exact supported producer profiles, operating horizon and experiment owners remain OPEN. Program placement/FR-41 and FPV-00…15 dispositions remain coordinator gates; EPIC-009 is unaccepted. Neither this body nor the SOLUTION accepts EPIC-010 S1–S4 or repairs protected main read-maintenance artifacts.

## References and proposed canonical relationships

- `prd-document-k6a638805c55b95a65262c2e83492a6fe` — proposed based_on; FC-1–6, FR-02/04/05/18/25/27/29/34/40/41.
- `solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d` — proposed based_on; full A/B/control comparison, dependency and experiment obligations.
- Core `src:c3baf53b9bec0501`, `adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer` — based_on, no amendment.
- Core ADR-003 and `adr-storage-classes-for-machine-written-records` — binding authority context, no supersession.
- `hyp-configuration-aware-semantic-facts-remain-kd6116d8df145e37aae2d82fd47825589`, `hyp-integrated-traceability-improves-complete-k4a105d2bef910728a27bd97e53ab62dd` — proposed informs, unvalidated.
- Companion local bodies `subject-graph-contract.md`, `authority-recovery.md`, `capabilities-and-queries.md`, `federation-access.md`, `evidence-execution.md` delimit the contract; local filenames are not canonical links.


## Publication and upstream admission boundaries

The six allocated ADR drafts retain the complete F-01–34, FR-01–41, T01–17, NFR-01–12, FC-1–6 and AC-1–6 target and the preparation inventory's traceability, without automatic adoption of EPIC-009/FPV-00…15. Corrected EVID-229 is documentary proposed-input admission only. Native reasoning is author analysis, not independent evidence, empirical verification or final Critical approval. Raw/export Critical and any validator Deep classification, provider/exit/parse warnings and last-published-generation caveats belong to actual receipts and remain disclosed.

The preparation publication-manifest.tsv SHA-256 867fdca01fcef9e62418351216163b253925c1bb889f0128287058df902b23c2 and every preparation file remain immutable. Current publication framing replaces the historical noncanonical/disk-hold status; local labels retain traceability only. SPEC-015 is separately published in Product, absent from this Decisions input set, and is not linked as though locally present; successors meet once in the authorized downstream RFC. The future RFC must reconcile exact keys, contracts and dependencies rather than produce a competing SPEC. No absent cross-project record is a native link target.

Core FR-41 research local://analytical-fpv-disposition-research-full.md (SHA-256 589e66bffd7e9e942aedde942eb7c26d6eea5f93dc02f770ba57c9fc100c359d) and Web/Marketplace boundary research local://analytical-consumer-boundary-research.md (SHA-256 1d54e0adeb4e857ef7bc86c0ab4320b0ebb1d3b7c4b86c289354435bc20341b1) are proposed documentary dispositions, not policy overrides or implementation proof. EPIC-009 is unaccepted and PROB-082/FR-41 remains OPEN. Current FPV dependency records agree on 02+03→07; the contrary preserved import is historical. Freeze a reviewed dependency source and resolve compiler/policy bootstrap and later evidence-model/consumer edges before execution; this does not authorize sixteen inherited tasks. Current maintenance/capture/allowlist discrepancy requires current owners' reconciliation, not a precedence guess or demonstrated runtime bug.

## Explicit ADR-001 boundary and program disposition proposal

Active Core ADR-001 explicitly rejected `CodeIndex`/tracker/plugin adapter traits and direct external-system integration; ADR-025 reaffirms it rather than granting a new producer exception. Propose the following classification for owner review: reusable in-process parsing and pure ingestion of already authorized, versioned facts belong to the analytical contract; running compiler/LSP/plugins, acquiring external state, credentials or process management belongs to the authorized external executor/extension host. Whether a new Core-owned producer port falls within that pure ingestion boundary is an explicit ADR-001 owner clarification or separately approved narrow amendment before implementation. Calling it a producer or trait does not resolve the collision. This draft does not amend ADR-001, add a Core external-integration runtime, or select a universal trait design.

FR-41 proposed dispositions retain read-only dispatch/order/blocked/progress/graph/tree/stale/blindspots; claims are mutexes, session noncanonical, phase advisory pending explicit absorption, estimate/calibrate extension-owned with migration RFC/window, memory retirement only after NOTE migration/export, playbook process runtime extension-owned with its own migration and eventual ADR-011 decision. Retain declarative playbook/Pack data and graph ingestion responsibility; no whole ADR-009 supersession. ADR-025 resolves orchestration direction and ADR-026 storage authority, not actual migrations or readiness. PRD-082's provenance completion does not cover execution/scope enforcement/scoring/activation. Defer R_eff v2, named host/runtime integrations and service mechanisms until separately authorized need/evidence/migration gates; retain every mature analytical capability. Optional service overlap with draft ADR-018 requires reconciliation, not an active-policy override. Full FPV-00…15 matrix in the referenced research remains a proposal for owner/independent review, not sixteen adopted tasks.

### Allocated decision inventory for the future RFC

| Local handle | Allocated draft canonical key | Integration responsibility |
|---|---|---|
| D-KERNEL | adr-own-cross-domain-rust-analytics-keep-kf1722a1f4ac7e2bfa083f25edf92c62e | Core architecture/product: ownership and conditional A/B |
| D-GRAPH | adr-bind-typed-analytical-facts-to-qualified-k60330dc7a76a8131b913e97303806321 | Identity/query/Constellation: qualified subjects and relations |
| D-AUTHORITY | adr-preserve-versioned-authority-and-durable-k0c75d791cf683ce6ce085df9affa0dd1 | Storage/canonical-operation/privacy: class recovery and open backend |
| D-CAPABILITY | adr-expose-bounded-semantic-capabilities-and-k3314829d0ecfe5f986c14f0e41a5103f | Language/query/docs/surface owners: capabilities and explanations |
| D-FEDERATION | adr-federate-authorized-captured-contracts-k8f01e6d9cba774418cb3feb0c15ac5d2 | Federation/source/security: coherent delivery and access |
| D-EVIDENCE | adr-separate-evidence-eligibility-from-kbae526aed0144c49e4e5401aea631938 | Evidence/policy/external CI/migration: trusted production and parity |

These are six proposed decisions, not six implementation modules or assigned people. Freeze GRAPH/AUTHORITY foundation interfaces together; CAPABILITY and EVIDENCE share result/observation contracts before sequencing actual producers and consumers. Native metadata dependencies remain directed and acyclic; reciprocal prose references describe compatibility, not circular prerequisite links. P1 depends only on accepted foundation/program contracts, not P3/P5/P7 delivery; later actual integration gates remain mandatory. Core/Web/Marketplace successors meet once in the downstream RFC after independent admission.

## Preconditions

Before implementation: explicit ADR-001 producer-boundary disposition, reviewed FR-41/current-owner reconciliation, independent Critical architecture admission and separately authorized implementation. E0 owner/verifier/corpus/threshold/horizon gates precede empirical selection; no A/B or backend choice is made here.

## Postconditions

Acceptance must prove the full declared ownership/capability contract and each actual introduced producer→consumer path, preserving external process/tracker/extension authority and every mature requirement. Failure leaves the proposal unaccepted or reopens the affected boundary; document publication is not runtime completion.

## Affected Files

This cycle changes only this newly allocated ADR and its five companion ADR files. Future implementation scope is the Core analytical/model/query and producer-boundary callers plus authorized CLI/MCP/extension consumers; exact source-file census and owners belong to the downstream RFC before execution. No existing source path, policy or ADR is changed by this publication.
