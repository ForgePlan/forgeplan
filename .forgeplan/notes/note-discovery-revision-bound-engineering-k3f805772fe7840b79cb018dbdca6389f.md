---
assigned_number: 67
author: null
created: 2026-11-20
depth: critical
id: NOTE-067
kind: note
links:
- target: EPIC-009
  relation: informs
- target: PROB-082
  relation: informs
- target: adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer
  relation: based_on
- target: adr-storage-classes-for-machine-written-records
  relation: based_on
parent_epic: null
predicted_number: 67
slug: note-discovery-revision-bound-engineering-k3f805772fe7840b79cb018dbdca6389f
status: draft
tags: []
title: 'Discovery: revision-bound engineering decisions across code, contracts and verification'
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 67
created: 2026-11-20
predicted_number: 67
slug: note-discovery-revision-bound-engineering-k3f805772fe7840b79cb018dbdca6389f
updated: 2026-11-20
---

## 1. Sources and epistemic status

Discovery dossier, 2026-11-20. Draft product input, not an approved architecture, implementation acceptance, customer validation or independent review. Research synthesis, NOT an interview. Owner intent is authoritative for requested scope; market demand and superiority remain assumptions, unvalidated.

Pinned read-only inputs in `/Users/explosovebit/Work/ForgePlan/` (SHA-256 of complete files):
- D: `dev/dialog.txt`, 1999 lines, `de99aae985daac7d94aeefbf7109dc6e9e0593c1007e5fa8320409828401735c`. Lines 1–1682 are historical AI discussion, including false claims. Lines 1683–1999 are a 17-section editorial continuation, NOT a verbatim interview.
- R: `dev/forgeplan-research-roadmap.md`, 1368 lines, `a93c792e056620e80541746287fbb4ec2149862468df732823cd363b6ee64dba`. Parts I–VII preserve proposals; Part VIII carries current direction. F-31…F-34 came from a separate owner-supplied strategic message, not the original dialogue prefix.
- A: `dev/forgeplan-reality-audit.md`, 313 lines, `fd2d1fd6a8eb84fcbaecdc23e543d07f5da9bf745bb0b7467ba8d1d93a78f1b4`. Factual corrections take precedence over old assistant assertions. A§11 supplements the earlier code audit.
- Owner full-target rule: `AGENTS.md:239–258`. Full system first, delivery by dependencies; later is not rejected. Coordinator contract: `local://forgeplan-product-pipeline-contract.md` (orientation/dispatch, not product evidence).

All lines of D/R/A were read, including the complete corrected F-01…F-34 register and appendix. Historical source map: D1–145 Reference A;146–268 storage/Rust;269–655 graph/commands/ports;656–810 visualization/federation;811–988 Tauri/CI;989–1200 bundles/Hub;1201–1447 multi-repo/legacy;1448–1606 framework/triage;1607–1682 Bun. Current appendix D1694–1999 overrides earlier target reduction.

Research provenance, attributed to A/D rather than newly reproduced here:
- ForgePlan code reading from busy working tree based on `b0c9259dc04d0c6d119844895c7a629fde6bea80`, not a clean-tree guarantee; ForgePlanWeb narrow graph/poller reading at `54ae12d6b64180919b09ca9d13654257cc4bd9b4`.
- Reference A source 0.16.0 commit `A-19`; action README tag v1.5.0 references Reference A0.14.0; public the public demo repository map built by0.2.3 at A-20. These are different observations, not one tested release.
- Primary links: A-21 ; A-22 ; A-18 ; A-23 ; https://bun.sh/blog/bun-in-rust . Full URL bibliography is A284–313 and D1970–1999; this dossier preserves those pinned source pointers, not a claim of fresh web verification.
- Prior recorded browser interaction: asset.dto.ts 10 direct/30 two-hop; base.service.ts65/281; depth1→65. Demonstrates published map interaction only, not graph accuracy or renderer speed. No browser interaction by this author.
- External benchmark numbers are vendor/author reports. A74–88 distinguishes100 impact queries from55 agent questions; mean tokens−32%, with30 lower/25 higher. TS token growth does not prove broken barrels. Build timing belongs to older releases. Bun report describes19 known regressions despite existing tests. No universal savings, timings, monopoly or mathematical parity claim follows.
- This author ran no Rust/Reference A/action tests, Cargo, benchmarks, migrations or fault injection. NOT_RUN is not PASS. Reading supplied research is not independent replication.

## 2. Brief: whose problem and which job

Problem: engineers changing a multi-repository system cannot presently rely on a single revision-bound account connecting an engineering promise to relevant code, consumers, actual checks and remaining uncertainty. The requested improvement is an explanation-backed decision about change, not merely an import map or a cheaper index. The prevalence, willingness to adopt/pay and magnitude of benefit outside this owner's case are assumption, unvalidated.

Specific segments/JTBD (proposed segments, not interviewed personas):
1. Developer/agent operator coordinating backend, web and SDK in3+ repositories. When changing an API/event, I want producer/consumer versions, affected scenarios and required checks, so I can avoid a compatible-looking but behavior-breaking rollout.
2. Legacy migration lead crossing languages or component boundaries. When replacing a component, I want observable contracts, state/errors/effects and an independent frozen oracle, so I can choose a seam and authorize cutover/rollback without silently deleting rules.
3. Reviewer/system owner without expertise in every stack. When asynchronous agents finish independent work, I want understandable risk cards with source examples, checks and unresolved decisions, so I can review the right things without accepting hidden risk.
4. New contributor/agent on an unfamiliar task. When exploring, I want a bounded reading route explaining why each file/scenario matters, so I can solve the task rather than dump a whole graph into context.
5. Platform/CI maintainer across teams. When accepting reports or publishing contracts, I want pinned inputs, permissions and trustworthy provenance, so untrusted PRs cannot weaken their own policy or leak private metadata.

Owner quotations (verbatim, not repaired into fabricated interview evidence): D1201 «я хчоу максимум пользы от fogeplan и чтобы можно было разрабатывать репозитория илипроекты где 3 и более реппозтиоиев и где нужно фичи продумывать большие а также переписывать ьлегаси проекты». D1526 «и да видел чт оды там где т описал чт опока юзер не поострит гадазами-  сут ьв том чтобы если у нас был аткомтаичекий какой то процесс то он шел бы без юзера но давал юзеру понимание где потом отревьювить и помним про когнитивную нагрузку особенно тех мест где например нет экспертизы - поэмтоу эт онужн оучест ькак в cli тулах ак и в tuari ux».

Success metrics, proposed SMART experiment definitions, NOT results: by the end of each pre-registered evaluation cycle (duration/sample/threshold TBD in Shape), measure task completeness and missed necessary changes against an independent oracle, false alerts, review time and end-to-end cost for (a) a3+repo contract change, (b) one real legacy component replacement, (c) an unfamiliar agent task. Freeze the threshold and sample before execution; never optimize token savings at the expense of correctness. Record cold build, incremental maintenance, cache reads/writes, failed runs and distributions separately. Runtime SLOs, renderer targets, supported configurations and adoption targets remain TBD, not imaginary numbers.

In scope: mature own Rust analytical core owning facts/provenance/query/policy/evidence links, integration with existing parsers/compiler/LSP/SCIP and test tools; all capabilities in§4–5; whole product formalization and tracker handoff. Out of scope for THIS discovery delivery: Rust/source changes, runtime acceptance, PRD/EPIC/RFC/SPEC/ADR authoring before independent Define review, automatic activation, customer-proof claims. Non-goals of target: universal program understanding; own compiler/test runner/Git host for its own sake; covert auto-approval or duplicate task tracker. Later-stage behavior/UI/Hub capabilities are NOT excluded from the target.

## 3. Current capabilities, alternatives and existing boundaries

A107–148 establishes existing file-level artifact mapping, configurable YAML affected_files in dispatch, BGE-M3/1024 resolver,76full/9read-profile MCP tools, qualified KEY@qualifier references, registry and pinned Git snapshots, deterministic bounded context packs, generation manifests and evidence scoring. Smart search uses weighted max(keyword,semantic), not RRF. Activity reports tool time/status, not provider billing. Production storage still couples to Lance despite partial ports; code semantics and behavioral parity are not established as delivered. Web already has numerous graph modes and an existing constrained3D design; desktop readiness is not established. Existing guard controls session transitions, not test execution/correctness. Empty drift after a failed probe is not evidence of no change.

Three live alternatives for future solution comparison: retain current scope plus manual integration; integrate pinned external analyzers as sources; own mature Rust core reusing parsers/semantic tools and external execution. Owner requests the third as target; experiments may change mechanism/order, not silently reduce the agreed target to the second. Rust alone guarantees neither accuracy nor binary size. SQLite is a candidate requiring its own decision, not prerequisite for every scenario. A66's earlier adapter-first recommendation is bounded by A233/D1700's subsequent owner clarification.

Reconciliation, no mutation or blanket adoption of existing artifacts:
- Core `src:c3baf53b9bec0501`: EPIC-009 draft is closest historical umbrella but explicitly unaccepted; PROB-082 is the collision/boundary gate. Later active ADR-025 external-orchestrator and ADR-026 storage-class decisions resolve some historical concerns, not the whole program. This discovery informs that reconciliation, does not accept/supersede it.
- EPIC-010 key `epic-constellation-contextpack` is claimed independent dependency; no edit, takeover or S1–S4 acceptance. Existing qualified identities and snapshots are reused, not replaced with alias::id.
- ADR-025: Core owns graph/contracts; external orchestrator owns scheduling/processes/worktrees. Execution adapters produce/consume permissioned receipts; no new autonomous runtime in Core by implication.
- ADR-003 and key `adr-storage-classes-for-machine-written-records` (ADR-026): canonical versioned authored decisions/accepted code links/policies/exceptions plus canonical machine bundles where designated; local receipts and external CI/tracker truth remain separately owned. A rebuild cannot erase accepted links or verdict history.
- PRD-016 remains file-level only; EPIC-006 is narrowed document migration; EPIC-008 preserves factum/intended separation. Semantic graph/legacy engine are deltas, not delivered by their active statuses.
- RFC-003 draft and RFC-006 active do not authorize SQLite. ADR-028/029 drafts and ADR-030/SPEC-009 identity work need explicit reconciliation with current PRD-092/RFC-021/SPEC-013/ADR-032 maintenance contracts, especially read-path writes versus read-only captured views. Do not settle precedence from dates alone.
- SPEC-010 bounded federated context is the reading-order foundation; PRD-082 git provenance excludes running tests. PRD-034/085 artifact CI is not federated runtime acceptance.
- Web source `src:61600c6e7e40d4b0`: EPIC-001, SPEC-006 forgeplan.map/v1 and ADR-011/RFC-0363D constraints are reused; ADR-010 optional daemon and ADR-012 external tracker boundaries remain.
- Marketplace `src:003b4e75d638bcca`: ADR-023 discovery skill/no new orchestrator; PRD-081/RFC-026 frozen comparator pilot is not universal parity; EvidencePack PRD-079/ADR-031 drafts are not trusted-host acceptance; PRD-077/SPEC-004 tracker integration needs qualified identity rather than bare display ID.
- A138–142 risks #548 dispatch cross-bucket collision, #577 affected-files drift discrepancy, #578 snapshot writer atomicity stay visible. #303/#305 are closed, not assumed current defects. No new layer magically fixes them.

Competitor finding: Reference A already provides navigation/architecture/impact/read-first and action review-plan→JUnit/required-tests/evidence integration. JUnit is from a separate step; none-found/no-record/partial are not complete. Baseline policy changes, report provenance and test results are separate dimensions. Three advertised evidence/acceptance/controls libraries were unpublished at source reading; their ideas are not absent and their future packages are not shipped. Hash/sidecar does not authenticate a compromised executor. Compliance material is not certification. Joern demonstrates code-property-graph composition, SCIP provides interchange, neither proves universal language depth or product uniqueness. A broad competitor/adoption study has NOT_RUN; positioning is a hypothesis about end-to-end decisions/federation/migrations, not 'no competitors'.

## 4. Full target capability envelope and proposed verification

P1 facts/storage/reproducibility: canonical versus derived versus exchange; durable evidence bundles and retention; source/revision/config/tool version/span/method/status for every fact. Resolved/heuristic/ambiguous/unknown/stale remain distinct. Rename/move/overload matching cannot silently merge uncertain entities. Publish a verified new generation atomically while readers retain a coherent old one; crash/rebuild/dirty-tree tests and recoverability are required, not performed.

P2 structure/semantics: five analysis levels must coexist: A syntax/error regions; B compiler/LSP/SCIP definitions/references/implementations/reexports/aliases/generics/macros and configuration; C control/data flow, errors, reads/writes, state, transactions, network/queues/async; D observed tests/coverage/traces/differential runs; E intended requirements/reasons/scenarios. Static path ≠ executed path, observed path ≠ all possible paths, extracted rule ≠ approved requirement. Deep Rust dogfooding then TS/JS/Python remain target languages, with capability profiles and unsupported configurations, not85%market coverage. Validate pinned language/framework corpus, negative cases and incremental/full-build equivalence; no claim all semantics are supplied by Tree-sitter.

P3 links/work queries/docs: approved canonical scenario↔code linking; impact/place/boundaries/compare/change queries; task-specific reading order for bug/API/onboarding, SCC groups for cycles, declared directions/edge kinds/depth and cut reasons, why-read file:line/revision/status, omitted/unknown and continuation. Budget never silently removes mandatory constraints. Generated factual subsystem/entry/contract/test/ownership documentation shares query output with JSON/CLI/agent/UI; observed versus intended and manual explanation ownership prevent a generator from rewriting a normative SPEC to match a bug. Validate lost/deleted/renamed links and explanations, not merely node counts.

S1 ownership preservation: active ADR-025 explicitly assigns estimate/calibrate to MOVE TO EXTENSION even though they read graph facts. Retain the analytical outcome; Core supplies facts, while the extension owns estimation/calibration behavior. Any move needs a migration RFC/deprecation window, or a separately reviewed amendment; this NOTE does not amend ADR-025.

S2 language delivery preservation: retain on-demand language/parser extensions without rebuilding Core as a P2/P7 candidate. WASM is an option, not an approved design. Verify producer/core/schema compatibility, offline/online distribution, licensing, permissions and capability declarations; syntax grammar installation never implies semantic or behavioral support.

S3 report preservation: P3/P7 includes an authorized, self-contained human-readable report usable offline without a running server or development environment. Use the same query model and carry subject revision/configuration, provenance, omissions and access/redaction policy. Container format remains open; verify offline viewing, stale-data disclosure and restricted-data exports independently of evidence-bundle replay or connected Web/Tauri.

P4 tests/evidence: separately answer declared test, runner-discovered test, actual execution outcome/configuration, changed line/region/branch coverage, assertion strength/mutation analysis, SPEC scenario/effects verified. Stable test identity; source reports with failed/skipped/not-run/no-record/partial/stale. Zero tests never PASS; indirect scenario tests count and direct calls alone do not prove quality. Reuse cargo-llvm-cov/cargo-mutants and other tools; aggregate coverage is not test→region attribution; surviving mutants may be equivalent. A frozen independent oracle and trusted executor are not produced by the author inventing fixtures. Verify missing/stale/tampered reports, mutation canaries and simultaneous test/implementation changes.

P5 federation/CI: reuse qualified identity/snapshot handles. Two-level local detail and published contract/consumer/evidence graph covering API/schema/event/package/FFI/ABI. Pin coherent repository versions, feature configs, producer versions, dirty content, missing/denied sources and delivery. A HEAD/path lock lacks remote contents/portable identity. OrderCreated required-field scenario tests serializers/defaults/backward messages on pinned consumers, not name-matching HTTP heuristics. CI separates boundary result/job/test evidence/analysis completeness/acceptance; baseline and required tests come from protected base, policy weakening needs separate authorization. Isolated permitted executors or trusted CI reports; retain failure/interruption/not-started outcomes, replay bundles offline, invalidate on subject/config/tool/policy changes. Hash integrity ≠ signer trust. No secret-bearing privileged runner for untrusted PR/indexer/proc macro/plugin. Acceptance and deploy require policy, not numeric R_eff alone.

P6 behavior/legacy: dossier of seams/contracts/input/output/errors/state/effect order/data/transactions/consumers/dynamics/tests/gaps; compare Strangler/whole-component replacement/refactor, not SCC/leaf-first as business truth. Characterization/differential harness with controlled time/randomness/state, redacted data, tolerances and approved divergences; cutover/rollback criteria with external execution ownership. Two stages preserved behavior then idiomatic refactor. Anti-stub/error-loss checks are explainable contextual signals; branch counts and banned words are not proof. Unsafe/FFI need appropriate compiler/Miri/sanitizer/runtime checks; no automatic UB prevention guarantee.

P7 working UI/team operation: exact list/table/diff plus2D/3D, change/contract/evidence/migration/unknown lenses, base/head/old/new comparison, explainable node panel, editor links, saved filters and shared agent context. Keyboard/text alternatives, permissions and honest stale state are required. Triage first page3–5 is UX hypothesis, not a hidden remainder; prevent starvation, show what changed/who affected/what checked/unknown/required decision. High R_eff cannot hide fresh drift. External orchestrator may continue isolated independent tasks; merge/deploy/schema changes still need authorization. Web/Tauri common data/query contract with platform-specific transport/local rights/watchers/deep links. SSE requires reconnect/resume/staleness contract, not automatic replacement. Role presets/UI-focus/dry-run never pretend actions executed. Optional team Hub supplies authorized bundles/distribution/trust/updates at scale; Gitea/Forgejo is an adapter, not mandatory Git hosting or local-analysis prerequisite.

## 5. Complete F-01…F-34 preservation ledger

Phase labels P1…P7 refer to§4 dependencies, not approved releases. All rows preserve an outcome or evaluate a mechanism; 'candidate' is not rejection. Each verification below is proposed, NOT_RUN. Source for every row: R1256–1293 original register + A184–223 corrected register, with current appendix overrides in§4.

|ID|Outcome / disposition relative to current capability|Phase|Verification needed|
|---|---|---|---|
|F-01|SQLite candidate; production Lance coupling remains, no migration accepted|P1|Search RU/EN relevance, generations/readers, crash/recovery, build/size/cost comparison|
|F-02|Own Rust codegraph, reuse syntax and semantic producers; new beyond scoped production|P2|Pinned Rust/TS/Python semantic corpus, negatives and supported configs|
|F-03|New durable artifact↔symbol bridge, not just table|P1/P3|Rename/delete/overload/revision identity and retained tombstone evidence|
|F-04|Retain seven command-family outcomes; Core supplies facts, estimation/calibration behavior belongs to extensions under ADR-025|P3/P4/P5|Drift/decay, read-only dispatch, route inputs, guard, coverage/context; estimate/calibrate extension migration requires its own RFC and deprecation window, not new Core ownership|
|F-05|Existing partial ports; separate genuinely replaceable dependencies|P1|Production caller cutover and equivalent public behavior, no needless trait multiplication|
|F-06|Existing hybrid search not RRF; RRF independent candidate|P1/P3|RU/EN identifiers/relevance corpus, embedding profile/dimension/normalization fidelity|
|F-07|YAML supported; new canonical code_refs/doc-tag/diff proposal resolution|P3|One authority, explicit versus inferred links, deletion/rename and conflicting provenance|
|F-08|2D/3D target retained, Web already richer than five views|P7|Impact/contract/evidence navigation and accessible text alternative, no60FPS promise|
|F-09|10sdefault polling observed; SSE/version notification candidate|P7|Latency/load measurement, reconnect/resume and stale snapshots|
|F-10|Shared web/Tauri data contract candidate, portability not established|P7|Inventory dependencies/platform permissions and identical query semantics|
|F-11|Desktop watcher/editor/multiworkspace/inspector target|P7|Rights, deep links, update race/recovery and platform scenarios, no<2ms guarantee|
|F-12|Reuse existing qualified KEY@qualifier, reject parallel alias::id without migration|P1/P5|Ambiguity, canonical key and portable source qualification tests|
|F-13|Federation exists; ATTACH only optional bounded implementation|P5|Consistent cross-source reads, limits, multi-machine delivery and missing-source behavior|
|F-14|Registry/Git locks exist; new portable public-contract bundles|P1/P5|Remote delivery/addressing, coherent versions/features/evidence and dirty digests|
|F-15|Per-call resolver exists, #303 closed; no speculative replacement registry|P1|Only reopen on reproducible per-call/maintenance/worktree isolation failure|
|F-16|Optional Hub/team source adapters retained; not local prerequisite|P7|Access/redaction/trust/update/recovery at representative scale, no mandatory Git server|
|F-17|Profiles already split; workflow-sized MCP packs remain candidate|P3/P7|Schema cost/tool-selection/task-success experiment, no invented20-tool limit|
|F-18|Reference A navigation/place/align/compare/change/read-first; retain on-demand language extensions without rebuilding Core as a candidate|P2/P3/P7|Versioned distribution, compatibility, licensing, permissions and semantic capability declarations; WASM is one mechanism, not selected; loading a grammar does not supply deep semantics|
|F-19|Four CI stages retained; session/health alone not deploy gate|P4/P5|Protected policy, actual execution provenance, required checks, separate verdicts|
|F-20|Receipts/undo exist; bounded command journal where useful, not universal ActionBus mandate|P1/P7|Permissions/dry-run/history/UI consistency; external effects not undone by artifact undo|
|F-21|Release provenance target; digest is not signature|P4/P5|Trusted signer scope/key policy, replay/integrity/revision and rejected forged bundle|
|F-22|Qualified UI chips retained, not hallucination cure|P3/P7|Existence/permission/version/ambiguous target checks and source jump|
|F-23|Brownfield exists; new migration planning/comparison/cutover target|P6|One genuine seam, data/transaction ownership, external consumers and rollback|
|F-24|Extraction dossier/branches/effects retained; reject FR≥branches proof|P6|Behavior table↔scenario disposition: preserved/changed/unreachable/unknown, independent oracle|
|F-25|Language/framework resolvers and reexports retained; Reference A already supports some|P2/P6|Aliases/shadowing/cycles/cfg/framework dynamics, explicit heuristic and unknown coverage|
|F-26|New explainable risk inbox/triage target|P7|Risk→evidence→decision path, independent progress vs merge gate, remainder visibility/starvation|
|F-27|Bun practice reuse via external orchestrator, not fixed4-role engine|P6|Pinned porting rules/ownership, calibration, isolation/permissions, parity then refactor|
|F-28|Role modes/pinning/UI focus/dry-run retained|P7|Human task success, correct shared context, simulation never claims execution|
|F-29|Day/Week/Month effort presentation candidate; estimation/calibration stays extension-owned under ADR-025, Core provides analytical facts|P6/P7|Calibrate observed effort/uncertainty; preserve migration RFC and deprecation window; no schedule inferred as truth from graph metrics|
|F-30|Behavioral parity requires actual harness, not graph/R_eff|P6|Inputs/errors/state/effects/serialization/idempotency/tolerances and approved differences|
|F-31|Schema version useful; reject destructive open-cache deletion/rebuild guarantee|P1|Verified generation publish, retained canonical data, schema/crash/readers recovery|
|F-32|Partial parse with explicit stale previous revision retained|P1/P2|Broken code/import/config invalidation and unresolved negative-cache re-resolution|
|F-33|Context already deterministic; stable provider prefix/caching hypothesis|P3|Full client/tool prefix, cache reads/writes, total price and task quality|
|F-34|Real paired benchmark program retained, no fabricated measurements|P1–P7|Equal tasks/revisions/models, independent oracle, repetitions/distributions/full cost|

Appendix delta beyond coarse register: five analysis levels D1781–1793→P2/P6; six test-quality layers D1795–1811→P4; independent policy/execution trust D1867–1886→P4/P5; task-specific SCC reading routes D1813–1829→P3; generated factual docs/manual ownership D1845–1851→P3/P7; portable restricted federation D1853–1865→P5/P7; accessibility/full risk visibility D1888–1908→P7; phased verification outputs D1928–1968→all phases. These are first-class target capabilities, not optional footnotes to an AST wrapper.

## 6. ADI framing and risky assumptions

Author assessment (not independent EVID):
- Abduction A: the owner mainly wants a light replacement DB. Explains early Lance questions but fails later explicit full-target/behavior/UX requirement. Do not use it as the frame.
- Abduction B: the owner wants a wrapper around Reference A. Explains initial inspiration and comparative tooling, but D1700 expressly rejects that as target ceiling; useful as experiment comparator/source only.
- Abduction C: the owner needs traceable change/verification decisions across code, requirements and repositories, including legacy and understandable review. Best fit to D1201/D1526 and appendix; still does not establish market demand or integrated superiority.
- Deduction: if C is valuable, the three end-to-end evaluation scenarios should improve independently judged completeness/review decisions, not only token count. If graph precision or provenance is weak, a confident-looking UI can be worse than manual work. Core ownership must remain consistent with ADR-025.
- Induction: retain mature core target, compare mechanisms with existing-tool baseline, leave product and market hypotheses unvalidated until recorded experiments. No numeric confidence invented.


### Shared pre-execution protocol C1–C4

Every HYP-001…008 references this protocol. No numerical thresholds, participants or results are invented here. Before confirmatory execution, product owner and independent verifier approve a versioned protocol defining the decision, experimental unit, scope/configuration matrix, denominators (including expected-but-missing facts and unknowns), comparators and access/training equivalence.

Choose non-tradeable safety ceilings from explicit failure consequences and protected policy. Derive minimum useful improvement and maximum adoption/operating cost from the measured task baseline and stakeholder decision, with recorded rationale; an arbitrary tiny improvement is not useful merely because preregistered. Choose sample size/horizon for stated precision, detectable useful difference or bounded scenario coverage, including sustained-load conditions where relevant. Separate exploratory calibration from confirmatory evaluation; calibration cannot be relabeled as confirmation.

Freeze corpus/independent oracle, exact tool/model/source/config versions, metrics, exclusions, attrition/unknown treatment, error severity, thresholds, stopping rules and verifier approval before confirmatory runs. Later changes create a new protocol version and fresh decision, not retrospective success. Report each scenario/segment and failures; a favorable aggregate cannot conceal a policy-critical regression in another cell. Full cost includes onboarding/help, integration, cold build, refresh/storage, operation, review, migration, recovery and ongoing maintenance over the justified horizon.

C2 negative-case ownership: HYP-001 owns task-level reading/documentation failures; HYP-002 owns underlying fact/completeness failures. Required cells include tight-budget truncation of a mandatory ADR, SCC/cycles and wrong-direction traversal, missing oracle-positive facts, stale documentation, preserved manual explanations and normative SPEC content. Generated observed behavior cannot overwrite intended requirements, even when the implementation contains a defect. P3/P7 also tests offline authorized reports and on-demand extension capability gaps.

C3 common acceptance envelope: compose source/subject identities, revisions and dirty digests, configuration/features/target, analyzer/producer version, protected policy version, evidence references and provenance through storage→federation→query→report/UI. Deliberate mismatches must stay rejected/incomplete end to end; individually correct layers referencing different subjects cannot yield integrated PASS. This is one shared check reused across HYPs, not multiple independent corroborations.

C4 non-transfer rules: HYP-001 task benefit is not HYP-007 interface benefit or HYP-008 adoption; shared participants/tasks/runs remain correlated and must be disclosed, not double-counted as independent evidence. HYP-004 authenticated execution is necessary but insufficient for HYP-005 behavioral parity. HYP-006 backend preference is neither proof nor prerequisite of every analytical outcome. Each claim requires its own scoped decision rule and evidence interpretation; no cross-claim promotion.

Canonical risk index for final reasoning and independent review (all draft, all based_on PROB-107 key `problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389`; PROB-107 based_on this NOTE):
- HYP-001 analytical value — `hyp-integrated-traceability-improves-complete-k4a105d2bef910728a27bd97e53ab62dd`; three-arm baseline, scenario-specific safety, reading/docs negatives.
- HYP-002 semantic fidelity — `hyp-configuration-aware-semantic-facts-remain-kd6116d8df145e37aae2d82fd47825589`; independent positive/negative facts, silent omissions, exact Rust/config envelope.
- HYP-003 federation — `hyp-pinned-cross-repository-contracts-can-k8afe3a348d93315c9e233dbc689a623b`; unchanged-schema behavior, harmless structural case, bounded consumers and rollout matrix.
- HYP-004 evidence trust — `hyp-revision-bound-test-evidence-can-support-k246a4d18955838d13fd19c9fcc1e1d36`; trusted wrapper/untrusted report, substituted and contradictory records, mandatory checks.
- HYP-005 parity — `hyp-a-frozen-independent-oracle-can-expose-k9fef2f8356bc50be55a1f0055af33a37`; frozen state/effect ledger, post-write recovery and irreversible boundaries.
- HYP-006 storage — `hyp-sqlite-can-reduce-delivery-cost-while-k1b935932d0d098ef732e330d91e77872`; every durable class, authority/history retention and full migration/operational horizon.
- HYP-007 triage — `hyp-explainable-triage-and-linked-2d-3d-views-ke517e0d7400d2dbee1ca7e98973610c7`; continuous arrivals, escalation/visibility horizon and unknown independence.
- HYP-008 adoption — `hyp-target-users-prefer-an-integrated-decision-kb9a06cdf4b8513a2e5e3da335c631ae3`; eligible segments/deployment roles, repeat use, refusals/attrition and full cost.

Lifecycle provenance clarification: all eight canonical `new hypothesis` calls generated `hypothesis_state: inferred` in metadata by default. It is observed file metadata, not an author-performed promotion, experiment result or independent evidence. Claims remain explicitly assumption, unvalidated; the canonical runtime lifecycle reader remains unavailable as recorded below. Do not silently treat the generated default as verification or manually patch state to simulate a lifecycle tool.
Decision-changing hypotheses are separate real HYP artifacts based on the PROBLEM: analytical task value; semantic/incremental correctness; cross-repository compatibility; trustworthy test evidence; legacy parity; storage candidate; human triage/UI usefulness; market differentiation/adoption. Each carries scoped falsification, owner/TBD and revisit conditions, no manufactured experiment outcome or lifecycle promotion. Canonical ADI tool invocation/output is recorded separately in handoff; unavailable memory is not recalled knowledge. `mm-fpf-examples` and recall both failed before bank requests: Hindsight /version unreadable; fpl-hsmem3.7.0 requires API>=0.10.2,<0.11.0. No bank settings changed.

## 7. Operational plan and open questions

Discover→Define only in this delivery; JTBD primary, Lean evidence loops secondary, Double Diamond internal walk. No new orchestrator/session-phase taxonomy. Independent Profile B review of this NOTE+PROBLEM is the next gate; HYP mapping here records risks, not permission to proceed past Define into design/PRD. No author-created EVID.

Eight-step coordinator plan preserved: (1) admission/inventory; (2) sourced discovery/assumptions; (3) independent Define review; (4) accepted input→full product PRD and umbrella reconciliation; (5) solution/architecture alternatives and contracts; (6) independent architecture/guardian gates with frozen hashes; (7) Orchestra placement/gates/dependencies matching real artifacts; (8) exact final reconciliation. Existing tracker home is discovered by the coordinator, not duplicated by this author. Implementation remains Backlog/Blocked; documentation checks are not Rust acceptance.

Open questions, owner TBD unless named:
- Which real3+repo contract, legacy component and unfamiliar task form a representative, authorized, redacted corpus? Product owner + reviewer choose before thresholds.
- Which language/configuration capability envelope and non-negotiable checks define initial acceptance without excluding later TS/Python/deep analysis?
- What minimum useful task-completeness/review gain, maximum false-positive cost and measurement horizon should experiments preregister?
- Which retained EPIC-009 protocol pieces survive PROB-082 and ADR-025/026 reconciliation? Coordinator + independent architecture review; no umbrella mutation here.
- Which producer/runner/signature identities and access/redaction policy are trusted, and who may grant exceptions/change required tests?
- Which portable source identity and delivery contract preserves existing qualified references between machines?
- Which durable code links/exception/evidence records remain canonical through storage rebuild? Architecture owner; SQLite not presumed.
- What research participants and alternatives substantiate differentiation/adoption? No interviews yet.
- How do reviewers see the entire triage backlog without losing a small useful first page, including unfamiliar stacks and accessibility?

## 8. Handoff and limitations

Leave all owned artifacts draft, validated only for document structure. Do not activate, supersede existing decisions, import to main, modify source/tests/config/AGENTS/raw research, run builds or commit. Selective canonical export contains only these new artifacts and source-side relations; qualified existing references are context, not imported copies. Independent reviewer must check full-target coverage, factual provenance and honest hypotheses, then decide PASS/CONCERNS/BLOCKER. Structural validate success cannot certify demand, graph correctness, behavior parity or runtime readiness.

Suggested next step: coordinator dispatches independent Define reviewer (Astra) on frozen NOTE/PROBLEM/HYP bodies and pinned sources. Only after accepted Define input, dispatch specification; reconcile rather than overwrite EPIC-009/PROB-082. Hypothesis experiments and lifecycle transitions require actual eligible evidence. No mocked evidence, guessed IDs or body lifecycle hack.

Operational limitations observed by this author: the first canonical `reason` call on NOTE-067 used the existing configured `openai/MiniMax-M3.1-Flash-Preview`, failed exit1 because `http://127.0.0.1:8791/v1/chat/completions` refused connection (os61), and produced no ADI result. It was not Sol output or independent review. The coordinator separately authorizes one documented per-process `codex-cli/gpt-6.1-sol` retry without config/auth changes; its actual result belongs in the final handoff, never presumed successful here.

The first `new problem` timed out after120s before any projection/output. Inspection of canonical CLI creation shows a pre-create advisory origin/dev fetch; causal attribution of the timeout to that fetch is an inference, not reproduced proof. After comparing existing NOTE/PROBLEM titles, EPIC-009/PROB-082 and ADR-025/026 content, the product problem is distinct from the historical collision gate and tool defects. No existing HYP directory was present. Subsequent new calls use `--allow-duplicate` to omit the remote advisory check after that substantive comparison; no existing claim is bypassed, no similar-artifact warning is treated as permission to overwrite. New problem PROB-107 is an explicitly unvalidated product framing, not replacement of PROB-082. Remote uniqueness remains unverified.

Hypothesis lifecycle limitation: canonical `forgeplan_hypothesis_status` was called with explicit workspace `/Users/explosovebit/Work/ForgePlan-Analytical-Planning` and failed `-32602`: workspace found but store could not be opened, `Table 'artifacts' was not found`. CLI0.39.0 has no hypothesis lifecycle subcommand (its help rejects `forgeplan hypothesis`). All eight records are real canonical hypothesis artifacts with draft artifact status; lifecycle state is UNOBSERVED through the available canonical reader. No promotion was attempted without evidence, no legacy table was recreated, and no `## Lifecycle State` text was fabricated to simulate the unavailable operation.

Subsequent canonical ADI, explicitly authorized by coordinator: per-process overrides `FORGEPLAN_LLM_PROVIDER=codex-cli FORGEPLAN_LLM_MODEL=gpt-6.1-sol` with `forgeplan reason` on NOTE-067, no `--save`, exited0 and reported `codex-cli/gpt-6.1-sol`. Six tool-produced hypotheses separate value/framing, semantic profiles, trust/federation, legacy parity, triage and storage independence. Recommendation retains full target and independent Define review; no runtime/demand validation. Raw receipt: `/Users/explosovebit/Work/ForgePlan-Analytical-Planning/discovery-note-sol-reason.log`. Effective provider-side effort is not verified. The tool warned it read the last published generation while derived maintenance was busy; the substantive NOTE body was already published. This outcome supplements, does not erase, the first configured-provider failure. Author's assessment in§6 stays separately attributed; ADI is not independent EVID or reviewer acceptance.

Reasoning input boundary: the canonical ADI analyzed the then-published NOTE/context, not the final complete NOTE/PROBLEM/eight-HYP packet. Its context did not expose exact HYP identities/cluster links; it is not a frozen-final-graph review. The receipt precedes this provenance addition and final metadata calibration; no claim of identical final-body analysis is made.

Critical cycle calibration: this is a strategic cross-project Critical cycle. The earlier three-level CLI help text was incomplete: coordinator inspected unrestricted clap input and backend support for critical. Canonical metadata is requested as `critical` for all ten owned records; final receipts must establish whether each update succeeded. Two independent Astra slices (DiscoveryScopeReview and DiscoveryHypothesisReview) constitute one initial review round, both CONCERNS, not two rounds or canonical EVID. This revision addresses S1–S3, C1–C4 and eight per-HYP gaps; a second independent closure/gate round on frozen corrected bodies is still required before dependent specification. No manufactured defect quota, author ADI or structural validation can substitute for that review. Later PRD/architecture gates remain separate.
