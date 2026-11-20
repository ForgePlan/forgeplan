---
assigned_number: 107
author: null
context: '{grouping tag}'
created: 2026-11-20
depth: critical
id: PROB-107
kind: problem
links:
- target: note-discovery-revision-bound-engineering-k3f805772fe7840b79cb018dbdca6389f
  relation: based_on
parent_epic: null
predicted_number: 107
slug: problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389
status: draft
tags: []
title: Multi-repository change owners lack a revision-bound chain from engineering promises to verified acceptance
valid_until: null
---

---
assigned_number: 107
context: '{grouping tag}'
created: 2026-11-20
predicted_number: 107
slug: problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389
---

## Problem

**Status**: assumption, unvalidated.

Engineers coordinating a significant change across3+ repositories or replacing legacy components lack an established, revision-bound path from an engineering promise to the relevant implementation, consumers, actual checks and unresolved acceptance decisions. The owner explicitly requests this path. Its prevalence, adoption value and market advantage are not validated: no interviews, customer telemetry or paired ForgePlan experiment were conducted in this intake.

This is not a claim that ForgePlan has no federation, no evidence machinery, no UI, or that competitors only draw code graphs. Existing artifact/source snapshots, qualified identities, bounded context and file-level coverage are foundations. Proposed deep code semantics, observed execution, behavior parity, contract delivery and integrated review extend those foundations, with honest unknowns. The problem is not equivalent to 'LanceDB must be removed'.

## Context

Source dossier: the linked discovery NOTE pins complete D=`dev/dialog.txt`(1999lines), R=`dev/forgeplan-research-roadmap.md`(1368), A=`dev/forgeplan-reality-audit.md`(313) under `/Users/explosovebit/Work/ForgePlan/`, SHA-256 respectively `de99aae985daac7d94aeefbf7109dc6e9e0593c1007e5fa8320409828401735c`, `a93c792e056620e80541746287fbb4ec2149862468df732823cd363b6ee64dba`, `fd2d1fd6a8eb84fcbaecdc23e543d07f5da9bf745bb0b7467ba8d1d93a78f1b4`. A factual corrections precede historical assistant assertions; D1683–1999/RPartVIII preserve latest full-target intent. No source modified.

Owner request D1201 concerns3+repos, large features and legacy; D1526 concerns independent automated work with understandable later review, especially unfamiliar stacks. These are requirements of this owner, not independent market sources. D1694–1710 explicitly rejects reducing target to a small adapter. A107–142 records scoped existing capabilities/risks, not end-to-end runtime acceptance. A266–282/D1739–1759 correct competitor assumptions: Reference A action v1.5.0 combines review plan, required tests, JUnit/provenance and JSON evidence; it does not itself run tests. New library packages remain unpublished in the recorded research. No monopoly or universal advantage follows.

## Affected users and jobs

- Cross-repository developer/agent operator: when changing a producer API/event/schema, identify pinned consumers and compatibility checks before accepting rollout.
- Migration lead: when replacing a component across languages, preserve approved behavior including errors/state/effects and authorize reversible cutover with independent observations.
- Reviewer/system owner: when asynchronous work completes in unfamiliar stacks, see a bounded first page of meaningful risks, all remaining blockers, what ran on which revision and the decision required.
- Contributor/agent: when taking an unfamiliar task, read relevant requirements, entry points, dependencies and behavioral examples in a task-specific order with explained omissions.
- CI/platform maintainer: when accepting checks or sharing contracts, prevent policy self-weakening, stale/forged reports and private graph metadata leaks.

These are proposed segments. Participant counts and customer validation: NONE recorded here.

## Impact

Potential consequences, hypotheses rather than measured losses: missed consumers and serialization changes; silently lost legacy rules; weak or stale tests accepted as proof; confident summaries masking unsupported dynamics; expensive repetitive exploration; reviewer overload or hidden deferred risks; incompatible source/config generations; unauthorized export or execution.

Absence of graph edges cannot establish independence. An unchanged signature can hide behavioral incompatibility; a changed signature can remain compatible. A test declaration, direct call edge, discovered runner case, execution, coverage, assertion strength and scenario parity are different facts. R_eff assesses evidence-chain claims under its model, not probability of correctness or deploy authorization.

## Desired outcome and scope

One own Rust analytical model connects requirements/reasons/scenarios, code semantics/behavior, cross-repository contracts and execution evidence, exposing provenance, unsupported areas and acceptance decisions across CLI/MCP/docs and useful2D/3D/web/Tauri views. Reuse Tree-sitter, compiler/LSP/SCIP, language/framework resolvers, coverage/mutation tools and trusted external runners; do not build compilers or runtime orchestration gratuitously.

Full target retains all F-01…F-34 dispositions in the linked NOTE and appendix additions: five analysis levels; six test-quality layers; task-specific SCC reading routes; generated factual documentation separated from normative intent; legacy dossier/parity/cutover/rollback; federation delivery/config/version/access trust; protected CI policy; comprehensive triage, accessibility, role controls and optional team Hub. Stage delivery by dependencies, not target reduction. SQLite/ATTACH/RRF/ActionBus/SSE and concrete crate/layout choices remain proposals, not solutions already selected by this problem.

### Out of scope

This discovery does not implement Rust, prove demand, produce a PRD/EPIC/RFC/ADR, activate anything or certify runtime behavior. Target non-goals: understanding every arbitrary program; mathematical correctness/parity from AST or R_eff; implicit permission to merge/deploy; mandatory own Git host; duplicate task tracking; blanket language/latency/token guarantees. Later P6/P7 behavior/UI/Hub work is NOT excluded.

## Constraints

- Preserve qualified `(source_id, canonical_key)`/KEY@qualifier and existing snapshot handles; no parallel alias::id registry. Portable cross-machine identity/delivery needs a justified contract.
- Respect active Core ADR-025 key `adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer`: external orchestrator owns process execution/scheduling. Core owns facts/contracts/queries; permissioned executor integration does not move orchestration ownership.
- ADR-026 key `adr-storage-classes-for-machine-written-records` and ADR-003 separate canonical records, derived indexes, local receipts and external state. Cache rebuild cannot erase accepted links, exceptions or evidence history.
- EPIC-009 remains an unaccepted historical umbrella candidate; PROB-082 collision ledger must be reconciled with later ADR-025/026 rather than assumed fully resolved. Do not modify either. Claimed EPIC-010 `epic-constellation-contextpack` and S1–S4 are independent dependencies, not accepted delivery through this note.
- Facts retain revision/digest including dirty inputs, build/config/tool/producer/span/method and resolved/heuristic/ambiguous/unknown/stale. Unknown/failed/not-run never become PASS. Index generations must be coherent/recoverable, with explicit incomplete sources and denied access.
- Protected baseline/policy/required tests cannot be silently weakened by the evaluated change. Untrusted code/indexers/build scripts need sandbox/permission boundaries. Integrity hashes alone do not identify trusted authors.

## Success criteria and falsification

Proposed pre-registered evaluation, NOT_RUN. By completion of one evaluation cycle per scenario (duration/sample/threshold TBD before execution), compare current tooling/manual integration, pinned external analyzers and the new vertical on identical revisions/tasks/models/limits:
1. A3+repo OrderCreated/API/schema change: independently enumerate necessary consumer updates and compatible/incompatible version combinations; measure omissions and false alerts, including unavailable source and old-message cases.
2. One real legacy component: independent frozen characterization oracle covers outputs/errors/state/effect order/idempotency/serialization/tolerances, deliberately injected regressions and approved differences; measure missed behavior and cutover/rollback viability.
3. An unfamiliar agent task and human review: independently judge complete solution and reasons; measure review effort, wrong approvals, missed blockers and cost including index construction/maintenance/cache.

SMART measurement target is TBD: define in Shape phase before experiments, not invented here. Quality/false-negative ceilings precede token/time optimization. Evidence and CI tests include zero tests, stale/wrong revision reports, skipped/failed/partial outcomes, changed policy and compromised/unauthorized sources. Failure to improve useful decisions, excessive false confidence/cost, or inability to preserve trusted provenance falsifies the integrated-value hypothesis in the tested scope. A bad mechanism revises design; changing agreed mature scope requires explicit owner approval.

Shared protocol C1–C4 is normative for these proposed experiments: discovery NOTE-067 §6 defines experimental units/denominators/configuration scope, non-tradeable safety ceilings derived from failure consequences/policy, baseline-derived minimum useful improvement and maximum full cost, sample/horizon justified by precision or bounded scenario coverage, exploration versus confirmation and versioned preregistration with independent approval. Three comparator arms are without ForgePlan/current practice, existing ForgePlan plus external tools, and proposed integrated vertical with equivalent access/training. Per-scenario/segment results cannot conceal critical regressions. Shared participants/runs are correlated, not independent corroboration or proof of another HYP. Common source/revision/dirty/config/analyzer/policy/evidence mismatches must remain incomplete/rejected across storage, federation and presentation.

Preservation corrections S1–S3: retain estimation/calibration outcomes with extension ownership, Core analytical facts and migration RFC/deprecation window under active ADR-025; retain on-demand language extensions without Core rebuild as a candidate (WASM optional, grammar not deep semantics); retain authorized standalone human-readable offline reports from the shared query model without mandatory server, with provenance/omissions and restricted-data checks. None reduces the mature target or selects mechanisms prematurely.

## Alternatives and unresolved assumptions

Retain current scope/manual integration; integrate existing analyzers as external sources; own Rust analytical core with reused components (owner's requested target). Compare at task level, not graph-node count. Risk-specific HYP artifacts track analytical value, semantic fidelity, federation compatibility, test-evidence trust, legacy parity, storage candidate, review UX and adoption differentiation. All remain unvalidated; no promotion to inferred/verified is justified by this author synthesizing research.

## Open questions and revisit triggers

Owners TBD: corpus/participants and authorized data; initial language/config envelope; thresholds/sample/horizon; trusted runner/signer/policy authority; portable identities and distribution; durable record ownership; EPIC-009/PROB-082 per-boundary reconciliation; triage accessibility/backlog visibility and ability to continue only independent tasks.

Revisit on changed pinned source hashes, relevant language/tool/action releases, resolution of #548/#577/#578, new comparative/customer evidence, any false PASS or private metadata disclosure, incompatible federation version/configuration, oracle disagreement, or independent Define review concerns. Do not infer correctness from issue closure, document activation or a structural validation pass.

## Next step

Independent Define review of this PROBLEM and discovery NOTE (and risk HYP records) before specification/solution/architecture work. Reviewer owns EVID PASS/CONCERNS/BLOCKER; author does not manufacture it. If accepted, specification preserves the full target and explicitly carries unvalidated hypotheses. No build/test/runtime evidence is claimed by this draft.
