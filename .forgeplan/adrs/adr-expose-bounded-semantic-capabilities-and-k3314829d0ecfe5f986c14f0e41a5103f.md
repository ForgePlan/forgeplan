---
assigned_number: 36
author: null
created: 2026-11-20
depth: critical
id: ADR-036
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
- target: hyp-configuration-aware-semantic-facts-remain-kd6116d8df145e37aae2d82fd47825589
  relation: informs
- target: hyp-explainable-triage-and-linked-2d-3d-views-ke517e0d7400d2dbee1ca7e98973610c7
  relation: informs
parent_epic: null
predicted_number: 36
problem_ref: null
slug: adr-expose-bounded-semantic-capabilities-and-k3314829d0ecfe5f986c14f0e41a5103f
status: draft
tags: []
title: Expose bounded semantic capabilities and one explanatory captured-query contract
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 36
created: 2026-11-20
predicted_number: 36
problem_ref: null
slug: adr-expose-bounded-semantic-capabilities-and-k3314829d0ecfe5f986c14f0e41a5103f
updated: 2026-11-20
---

# Expose bounded semantic capabilities and one explanatory captured-query contract

## Status

D-CAPABILITY — Canonical ADR draft / Critical, 2026-11-20. PROPOSED and UNACCEPTED: publication records proposed contracts and open gates, not architecture activation, an A/B or backend choice, implementation authorization or delivered capability. The D-* name is a local traceability convention; the artifact's allocated canonical key/frontmatter is authoritative. No prior ADR is superseded. Active ADR-001/003/025/026 and current independently owned contracts remain binding; new analytical producer exceptions require explicit owner clarification or an approved amendment, not an implied bypass.

This publication succeeds the immutable disk-hold preparation. Corrected canonical EVID-229 (key evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403, SHA-256 865fba7aa30832200f2b7b90b7e75f3044c498319f22307f055c6b0c16167ae3) records one proposed-input round with two independent Sol judgments; it is not final Critical architecture acceptance. Actual native command/ADI/validation exits, provider, parsing and input-generation limitations are recorded under .local/design-adr-publication/, not presumed successful in this body. No F/G/R score or C4 acceptance is fabricated. Runtime/product experiments, builds, activation and downstream RFC remain NOT_RUN.

## Context

Syntax extraction, configuration-aware semantics, bounded behavior, observed execution and intended obligations have different authorities and failure modes. Collapsing them into one graph-confidence score makes a static call or test declaration look like observed behavior. At the opposite extreme, isolated tool outputs leave agents and unfamiliar-stack reviewers without an understandable connected answer.

FC-4 and FC-6 require composition without substitution. A language grammar can offer syntax without type/DI/runtime understanding; an observed trace covers its execution, not every path. Facts-to-docs must not overwrite intended SPEC or manual explanations. A common result contract must preserve these differences across CLI/MCP, agent packs, Web/Tauri and offline reports without forcing identical permissions or transport.

## Decision drivers

- Preserve all five analysis levels, declared configurations, honest gaps and wider/on-demand language delivery (FR-02/18/24/25/32/38).
- Provide explained impact, placement/alignment, task-specific reading and unfamiliar-stack review, not merely command names or graph pictures (FR-18/26/28/33).
- Preserve mandatory constraints, deterministic captured semantics and substantive human/machine parity (FC-1/6; FR-35/36).
- Reuse compiler/LSP/SCIP/framework and runtime tools without delegating acceptance authority or requiring Core rebuild for every supported extension mechanism.
- Avoid treating full/incremental agreement, a test link or R_eff as semantic completeness/correctness.

## Considered options

### A — Syntax-first graph with one confidence score and generated summaries

**Pros:** easy uniform ingestion; broad grammar coverage; simple ranking/UI vocabulary.

**Cons:** hides distinct configuration/authority gaps; syntax does not establish runtime behavior; a score obscures whether a missing edge is unsupported, denied or genuinely absent; summaries can launder uncertain facts into certainty.

**Assessment:** insufficient normative model. Syntax remains a useful level, not the whole platform.

### B — Capability-scoped assertions composed into an explanatory query result

**Pros:** preserves producer-specific strengths and limits; supports both normalized and compositional Rust kernels; gives every consumer the same substantive answer and explicit omissions.

**Cons:** capability evolution, conflict resolution and explanation quality are substantial work; consumers must handle partial/unknown states rather than a single green status.

**Assessment:** proposed. Enable individual profiles only after scoped independent conformance and integration evidence.

### C — Keep language/tool reports separate and require users to reconcile them

**Pros:** retains rich native detail; reduces premature common semantic assumptions.

**Cons:** does not satisfy connected placement, reading routes, dossier or common multi-surface explanations; reconciliation burden and authority mistakes fall on each consumer.

**Assessment:** useful raw evidence/native drilldown, not sufficient as the mature product boundary. B can preserve C's native detail within typed provenance.

## Decision outcome

Propose B. A capability profile declares language/framework, producer/tool/schema version, supported constructs, configuration/features/target/generated dependencies, delivery/install/upgrade/license, required permissions/trust and explicit unsupported behavior. Rust/TS/JS/Python are reference depth profiles, not a ceiling on mature language support. On-demand language/parser extensions without Core rebuild remain required where the chosen mechanism supports it; WASM is a candidate, not selected. Loading a grammar never silently enables deep semantics.

### Five distinct analysis planes

| Plane | Authority and output | What it cannot imply |
|---|---|---|
| A syntax | Reusable parser/scanner, nodes/spans and ERROR regions | Type resolution, dynamic DI or executed behavior |
| B semantics | Configuration-aware compiler/LSP/SCIP or scoped producer definitions/references/reexports/aliases/generics/macros/generated inputs | Complete runtime dispatch/effects or all configurations |
| C bounded behavior | Declared control/data/errors/state/read-write/transactions/network/queues/async/effect models with path assumptions | Universal program understanding or complete dynamic paths |
| D observation | Authorized run/trace/coverage/differential evidence bound to subject/configuration/run | Unobserved paths, assertion strength or parity without its oracle |
| E intention | Accepted requirements, scenarios, reasons, constraints and human-approved exceptions | Proof implementation conforms merely because a link exists |

Each assertion carries origin/method, scope and producer, not a guessed confidence percentage. Resolved/heuristic/ambiguous/unknown and freshness remain distinguishable. Unknown reasons include unsupported configuration/language, missing generated input, parse/read failure, dynamic resolution and missing/denied source. Absence of expected facts is a completeness failure even if both full and incremental computations omit them. Conflicting producers retain their assertions and explanation; no arbitrary winner manufactures certainty.

Invalidation includes imports/exports/config/features/target/generated input and previously unresolved dependencies. Last-good blocks keep old revision and stale status; partial new parse plus old facts is not a coherent current view. A failure to enumerate/read must not become deletion or empty success.

### Explanatory query contract

Every query declares authorized source/revision/configuration scope, relation kinds and direction, depth, result/context budget and intended task. Results retain capture/policy/provenance bindings, completeness/unknown diagnostics, inclusion paths/spans/status, why a node/file matters, cut reasons, omissions and safe continuation. Stable ordering/digest is required for identical captured deterministic inputs; any nondeterministic component is disclosed with its normalization policy.

Impact reports dependency/risk, not guaranteed breakage. SCCs organize cycles, not business boundaries or safe migration seams. Placement proposes a location and explains its consistency with applicable accepted architecture; conflicting rules or insufficient facts remain explicit. Alignment reports declared-rule violations with supporting paths/spans and unchecked areas separately from inferred architecture. Compare/changes/read-first retain base/head and typed relationship semantics.

Bug routes prioritize observed failure, relevant entry points/state/effects and checks; API routes prioritize accepted contracts and pinned producers/consumers/version/rollout; onboarding routes prioritize intended constraints, subsystem/entry boundaries and examples. Each route explains why it chose the order. Required constraints remain whole. If the budget cannot fit them, the answer is explicitly incomplete with omissions/continuation, not a truncated ADR presented as complete. Full-client prefix caching is an optional client optimization; its value requires full tool/provider/cache/retry/review cost measurement, not a promised token discount.

### Consumer and documentation boundary

One versioned captured query result supplies JSON, human CLI, MCP/agent context, list/diff/2D/3D, Web/Tauri and authorized standalone offline reports. Renderers may change layout/detail, not subject/status/omissions/verdict or authority. Actual HTTP/IPC parity is proved when introduced; common DTOs alone do not prove portability or identical permissions. Workflow-sized MCP packs and a permissioned UI bridge retain the existing full/read split; focus/pinning is not a mutation or execution receipt.

For unfamiliar-stack review the initial presentation explains behavior, premises and consequences in plain domain language without requiring code drilldown first. Where actual characterization/test evidence exists, include applicable source/revision-bound input/output/error/effect examples; otherwise say unavailable. Do not generate fictional examples as observations. Detailed code/native reports remain accessible under rights. Triage exposes the whole backlog, aged critical/unknown risks and re-evaluation; a small first page is not a hidden cap. Comprehension and correct decisions are evaluated separately from speed/accessibility/backlog visibility.

Generated factual docs keep per-section source/capture/status lineage and mark precise stale sections/reasons. Manual explanations preserve their owner. An observed defect never automatically revises normative SPEC to match code. Offline reports must include permitted content/assets and show revision/config/provenance/unknown without a server or development environment; evidence replay is a separate obligation.

### Deep behavior and legacy consumers

The same model supports an ExtractionDossier with boundaries, inputs/outputs/errors/state/effects/transactions/consumers/dynamics/tests/gaps and an explicit behavior ledger: preserved, approved-changed, unreachable with evidence, or unknown. Compare Strangler, whole-component and focused refactor against actual state/transaction/consumer ownership. Leaf/SCC topology cannot authorize a seam. Contextual anti-stub/error-loss detection is an explained supplemental signal, including false positives, not proof from banned words or FR/branch counts. Deep C/D integration, frozen parity oracle and post-write cutover/recovery remain P6 obligations owned with the external migration/harness owner; the query does not execute a migration.

## Consequences

### Positive

- Users and agents can distinguish what was parsed, inferred, observed and intended.
- Common substantive results prevent renderer/summary certainty inflation and support source-bound explanations.
- Wider language support can grow by declared capability without pretending all languages have equal depth.

### Negative and risks

- Honest incomplete answers may be less convenient than confident summaries; consumers need meaningful continuation and failure handling.
- Profile/corpus maintenance grows with frameworks, configurations and engine upgrades.
- A common normalizer/explainer can introduce correlated omissions; shared output does not make its conclusions independently verified.
- Maintaining factual/manual/normative document boundaries complicates regeneration and merge workflows.

### Neutral

No physical graph, provider sandbox technology, ranking algorithm, transport or UI framework is selected. Bounded behavior may legitimately answer unknown for a scope without removing the obligation to deliver declared supported capabilities later.

## Compliance and evidence gates

Before profile enablement: named producer owner, version/license/permission matrix and independent oracle-positive/negative/unknown corpus; include reexports/barrels/shadowing/cycles, Cargo rename/path/cfg/target, generated consumers, conditional exports, ambiguous receivers and dynamic DI. EP-2 proves actual producer→projection→captured facts, plus incremental/full and independently expected missing facts.

EP-3 proves actual facts/links→task query→CLI/MCP/context/docs/report/offline when introduced: mandatory constraint loss, wrong direction/SCC, missing fact, stale docs/manual/normative preservation; placement valid/conflicting/prohibited/unresolved cases. EP-6 proves actual behavior/oracle/run→dossier/parity. EP-7 proves every real consuming surface, editor navigation, role handoff, standalone assets/redaction, updates and opted-in team delivery. Earlier fixtures do not waive later actual integration. E0 blocks confirmation until owners/thresholds/horizon/oracle are frozen; no fabricated native ADI or F+G+R result substitutes.

## Invariants

AST ≠ semantics ≠ runtime ≠ intention. Observed path ≠ all paths. No unsupported/denied/failed region as empty success. No normative rewrite by generated docs. No renderer verdict upgrade. No mandatory constraint silently omitted for budget. No SCC/LOC as business boundary or effort truth. Estimation/calibration remains extension-owned under ADR-025, with its migration RFC/window.

## Rollback Plan

Version producer/capability/result schemas, inventory actual consumers and preserve native source provenance before normalization changes. Run old/new profiles on independently expected facts and complete tasks, disclose differences and affected scopes, preserve old query capture/report history and revalidate dependent links/docs. Roll back a faulty profile or renderer without deleting canonical links or reinterpreting past results as current. A capability withdrawal invalidates dependent acceptance claims visibly; it is not permission to silently delete mature requirements.

## Revisit Trigger

Any material wrong resolved fact, silent expected-fact omission, current-labeled stale result, lost mandatory constraint, ungrounded example, normative/manual overwrite, unexplained placement/alignment result or consumer verdict mismatch reopens the affected decision/profile. A compiler/framework/configuration/schema upgrade triggers scoped reevaluation before support expansion. Any one event suffices.

## Open questions

Profile-specific mechanisms, unsupported ceilings/materiality and independent oracles remain E0 decisions. A/B computation placement remains conditional. Current Web map/lazy3D/daemon, Desktop permissions, extension delivery, offline container and triage scheduling bounds require their owners' RFC/evidence; none is assumed implemented.

## References and proposed canonical relationships

- PRD `prd-document-k6a638805c55b95a65262c2e83492a6fe`, SOLUTION `solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d` — based_on; FC-1/2/4/6, FR-02/04/08–11/17/18/22–30/32/33/35/36/38.
- `hyp-configuration-aware-semantic-facts-remain-kd6116d8df145e37aae2d82fd47825589`, `hyp-integrated-traceability-improves-complete-k4a105d2bef910728a27bd97e53ab62dd`, `hyp-explainable-triage-and-linked-2d-3d-views-ke517e0d7400d2dbee1ca7e98973610c7` — informs, unvalidated.
- Core `src:c3baf53b9bec0501`: active ADR-025/026; EPIC-008 intended/factum, PRD-016 file-level mapping, PRD-082 provenance, SPEC-010 bounded context — reused context, not deep-semantics readiness proof. Exact keys/status of unpinned dependencies must be resolved before canonical linking.
- Web `src:61600c6e7e40d4b0` map/lazy3D contracts and Marketplace `src:003b4e75d638bcca` methodology/client/legacy pilot contracts retain ownership; no cross-project display-ID guessing.


## Publication and upstream admission boundaries

The six allocated ADR drafts retain the complete F-01–34, FR-01–41, T01–17, NFR-01–12, FC-1–6 and AC-1–6 target and the preparation inventory's traceability, without automatic adoption of EPIC-009/FPV-00…15. Corrected EVID-229 is documentary proposed-input admission only. Native reasoning is author analysis, not independent evidence, empirical verification or final Critical approval. Raw/export Critical and any validator Deep classification, provider/exit/parse warnings and last-published-generation caveats belong to actual receipts and remain disclosed.

The preparation publication-manifest.tsv SHA-256 867fdca01fcef9e62418351216163b253925c1bb889f0128287058df902b23c2 and every preparation file remain immutable. Current publication framing replaces the historical noncanonical/disk-hold status; local labels retain traceability only. SPEC-015 is separately published in Product, absent from this Decisions input set, and is not linked as though locally present; successors meet once in the authorized downstream RFC. The future RFC must reconcile exact keys, contracts and dependencies rather than produce a competing SPEC. No absent cross-project record is a native link target.

Core FR-41 research local://analytical-fpv-disposition-research-full.md (SHA-256 589e66bffd7e9e942aedde942eb7c26d6eea5f93dc02f770ba57c9fc100c359d) and Web/Marketplace boundary research local://analytical-consumer-boundary-research.md (SHA-256 1d54e0adeb4e857ef7bc86c0ab4320b0ebb1d3b7c4b86c289354435bc20341b1) are proposed documentary dispositions, not policy overrides or implementation proof. EPIC-009 is unaccepted and PROB-082/FR-41 remains OPEN. Current FPV dependency records agree on 02+03→07; the contrary preserved import is historical. Freeze a reviewed dependency source and resolve compiler/policy bootstrap and later evidence-model/consumer edges before execution; this does not authorize sixteen inherited tasks. Current maintenance/capture/allowlist discrepancy requires current owners' reconciliation, not a precedence guess or demonstrated runtime bug.

## Scoped Web and Marketplace consuming-contract deltas

Reuse existing Web real/derived decomposition, accessibility/shared focus/layout, lazy client-only minimap, existing 3.5 MiB Web-image cap, user-launched 127.0.0.1/read-only daemon and older/missing-daemon Tier-0 fallback. These are scoped current constraints, not universal platform thresholds or full analytical 3D acceptance. Do not import framework/package choices, grant Web `/api/*` execution, or equate HTTP reads with IPC/file/editor/execution rights. Owner-approved delta RFC must supply actual consuming paths for the larger mature surfaces while preserving these boundaries until explicitly amended.

Required actual surface negatives include unavailable WebGL with usable 2D, reordered/reconnected/mid-animation updates without generation mixing, historical mode never substituting live data, denied Desktop actions not escaping via Web fallback, hidden assets/graph not leaking metadata, standalone report viewing without service/network and optional Hub absence not disabling local work. Preserve substantive first-pass behavior/premise/consequence explanations, source-bound examples or unavailable, placement conflicts/unchecked regions, whole backlog and separate comprehension/correct-decision evaluation. Existing delivery/bundle measurements are source-document claims, not rerun measurements here.

Marketplace retains skills-first discovery and the existing pipeline/session/advisory-marker separation, Pack vocabulary and owner-reviewed extension compatibility. No sixth orchestrator, lifecycle work queue, checklist-as-correctness or implicit notification permission. Wider language/on-demand support remains in scope, but Core-owned external producer exceptions first require ADR-001 clarification/amendment; an extension label cannot bypass it.

## Preconditions

Before profile/surface enablement: responsible producer/query/consumer owners, ADR-001 boundary admission, version/license/permission/configuration matrix, independent expected positive/negative/unknown corpus and accepted captured-result contract. E0 and separate execution authorization precede experiments; Web/Marketplace deltas require those owners, not Core override.

## Postconditions

Actual producer/query/context/docs/report/UI paths preserve five analysis planes, explicit missing/stale/unknown facts, whole mandatory constraints and substantive source-bound explanations. Profile and surface claims require their actual integration evidence; normative/manual content and current consumer permissions remain intact.

## Affected Files

This publication changes only its new ADR. Future language/index/query/context/documentation/export and CLI/MCP/Web/Desktop consumers are logical scope; exact source files/current owners must be inventoried in the downstream RFC before source work. No grammar, framework, renderer, transport or existing policy is changed here.
