# Independent discovery hypothesis review

**Content verdict: CONCERNS. Formal readiness: BLOCKED.**

The eight hypotheses honestly describe unvalidated claims and preserve the mature product scope. Their protocols need tighter decision rules before execution. These findings do **not** refute the hypotheses; no experiments were performed.

**Reviewer:** `DiscoveryHypothesisReview`  
**Actual model:** `openai-codex/gpt-6-astra`  
**Scope:** All eight HYP bodies, NOTE-067 and PROB-107; selected current source passages identified below. This is one independent content review, not two review rounds or canonical Profile B EVID.

## 1. Frozen files and fingerprints

All ten **file and body SHA-256 values match the author handoff**. Body hashing used the exact bytes after the closing outer frontmatter delimiter, including generated inner metadata.

The following aliases resolve to exact files under:

`/Users/explosovebit/Work/ForgePlan-Analytical-Planning/.forgeplan/`

| Alias | Exact relative path |
|---|---|
| NOTE-067 | `notes/note-discovery-revision-bound-engineering-k3f805772fe7840b79cb018dbdca6389f.md` |
| PROB-107 | `problems/problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389.md` |
| HYP-001 | `hypotheses/hyp-integrated-traceability-improves-complete-k4a105d2bef910728a27bd97e53ab62dd.md` |
| HYP-002 | `hypotheses/hyp-configuration-aware-semantic-facts-remain-kd6116d8df145e37aae2d82fd47825589.md` |
| HYP-003 | `hypotheses/hyp-pinned-cross-repository-contracts-can-k8afe3a348d93315c9e233dbc689a623b.md` |
| HYP-004 | `hypotheses/hyp-revision-bound-test-evidence-can-support-k246a4d18955838d13fd19c9fcc1e1d36.md` |
| HYP-005 | `hypotheses/hyp-a-frozen-independent-oracle-can-expose-k9fef2f8356bc50be55a1f0055af33a37.md` |
| HYP-006 | `hypotheses/hyp-sqlite-can-reduce-delivery-cost-while-k1b935932d0d098ef732e330d91e77872.md` |
| HYP-007 | `hypotheses/hyp-explainable-triage-and-linked-2d-3d-views-ke517e0d7400d2dbee1ca7e98973610c7.md` |
| HYP-008 | `hypotheses/hyp-target-users-prefer-an-integrated-decision-kb9a06cdf4b8513a2e5e3da335c631ae3.md` |

All `ID:line` references below mean these exact files.

| ID | File SHA-256 | Body SHA-256 |
|---|---|---|
| NOTE-067 | `3f89f10723c60c839bb0b5cdedcd06ea511ff1bdcfcde69880a3e801b432bd47` | `5a31e47513f1983a6c41bddee3c8ee1a67c502db8b280e6e9e61a6f35d1c6811` |
| PROB-107 | `47a4b4cb13a0191b7862ec32bfa349e5e883b18170609f4da93816076d56810c` | `a9c5b11cb3782a53890d9d5eb3dddcc5c13c328a8f7c20fda2a2aeaa54356c37` |
| HYP-001 | `98662ef4e4a4fe347b5bd1932c1ae33bff6cf5776e6eac67906d72f37674e9ea` | `583bc25a009c9b0466fcfa92fd7021fd58043b4dae29aca87c47b5a22b754879` |
| HYP-002 | `49fa6eb5bb8fbc49fef964f0ccb299006571e6be21309583afcb8b70072bf363` | `7cabc63830d1e0ba84d666676f4f4c7f66c5407b7ea8d7eb1ab5e618629bfb9c` |
| HYP-003 | `c79021e8d05da9acd0b8bb7ec6e50d4ae1c3ecd41ca25947772dec6fd0e1a4c0` | `12787cdf5834e0b7240d97ff1bd43111bef64e84d63c678eb411692f30600950` |
| HYP-004 | `de9165d614e211ea8c3a44e9a5ec9f060be600f31d92a657055b2c371b8c495c` | `639ece25c22dcc10967d6f85c5d24451cc45f3720a03d80cb7b7dc570bba1edc` |
| HYP-005 | `1a8ec77d7edf4fbf904ab805ada21d4643c3589f123f07c8d2e33d96fc1cc596` | `4c3a3808867dc85003153844f5cb2abda2e4edc74a22bce4e078cb021bc44013` |
| HYP-006 | `47375840130fdfe33254132c79cb68c1db2c59884ed8fef2ef6f958139154f07` | `72be1346be19943db4a69767658625c59e00a077ce517a2a1f8fccef56605b0c` |
| HYP-007 | `109e6e663264456374b0babd3ff30f15ae324b4ae40c62fdde3cf8c37f41242c` | `411b140ee57c8dbb6003a075c4100d53c94255c4238f5e6f1065ef98af5bc109` |
| HYP-008 | `24632c2a6f03635805f9119556d28eec30d1adb906edaa57fbb4e76b7d86633f` | `7c80ccd89c1cc7e2582ce13059a99a9740814f3871492cd426bb6718bb2cc73b` |

Also verified the three source-file hashes against the dossier:

- `dev/dialog.txt`: `de99aae985daac7d94aeefbf7109dc6e9e0593c1007e5fa8320409828401735c`
- `dev/forgeplan-research-roadmap.md`: `a93c792e056620e80541746287fbb4ec214986248df732823cd363b6ee64dba` **Correction:** the exact observed hash is `a93c792e056620e80541746287fbb4ec2149862468df732823cd363b6ee64dba`.
- `dev/forgeplan-reality-audit.md`: `fd2d1fd6a8eb84fcbaecdc23e543d07f5da9bf745bb0b7467ba8d1d93a78f1b4`

Source passages inspected: dialogue1761–1968, roadmap1296–1368, audit250–282, and `AGENTS.md:239–258`. I did not independently reproduce the external research or re-audit product source.

## 2. Shared protocol finding

**C1 — Thresholds are deferred honestly, but their selection remains unconstrained.**

**Observed:** Every HYP at `:49` delegates numeric thresholds, sample size and horizon to the product owner and reviewer before execution. NOTE-067`:60,161–163` and PROB-107`:80–85` do the same. They identify *who and when*, but not *how a defensible threshold is chosen*.

**[INFERENCE] Missed failure:** A study could preregister a negligible improvement, an inadequately short observation window or an excessively permissive unknown rate, then satisfy its own criteria without establishing useful benefit.

**Minimal correction:** Add one shared pre-execution protocol requirement, referenced by all eight HYPs:

- Define scope, experimental unit, denominators and supported configuration matrix.
- Choose safety ceilings from explicit failure consequences and policy; they cannot be traded for speed or savings.
- Choose minimum useful improvement and maximum adoption/operating cost from the relevant task baseline and stakeholder decision, with recorded rationale.
- Choose sample size and horizon for stated precision, detectable useful difference or bounded scenario coverage. Separate exploratory calibration from confirmatory evaluation.
- Freeze corpus, comparators, metrics, exclusions, treatment of unknowns, stopping rules, thresholds and verifier approval before confirmatory runs.
- Treat later changes as a new protocol version, not retrospective success.

No invented numbers or completed discovery results are required now.

## 3. Per-hypothesis review

All eight receive **CONCERNS**, principally because C1 applies throughout. The additional findings below identify specific conditions that the present protocol does not explicitly exercise. The hypothetical failures are **not observed defects**.

### HYP-001 — Integrated task value: CONCERNS

**Strengths:** `:35,47,53` require independent judging, complete solutions, repeated conditions and cost accounting; token savings cannot compensate for incompleteness.

**Observed gap:** `:35` names current ForgePlan plus search/LSP and an external-analyzer integration. It does not explicitly retain the **without-ForgePlan baseline** required by `dev/dialog.txt:1964`.

**[INFERENCE] Missed failure:** The new vertical beats an awkward existing integration but remains worse than ordinary search/LSP plus established manual review.

**Minimal correction:** Name three separate arms: no ForgePlan/current practitioner stack; existing ForgePlan plus external tools; proposed integrated vertical. Freeze equivalent access, task information and training. Report results per scenario, with a predeclared rule preventing one successful task from concealing a critical regression in another.

### HYP-002 — Semantic and incremental correctness: CONCERNS

**Strengths:** `:35,47,53` correctly bind configuration, distinguish unsupported/stale facts, test changed imports and unresolved answers, and compare against both an independent oracle and full rebuild.

**Observed gap:** `:53` explicitly falsifies wrong resolved edges, merges and stale facts, but does not explicitly name **expected facts silently absent**. “Unknown must remain in the denominator” does not cover a fact omitted without an unknown marker.

**[INFERENCE] Missed failure:** Incremental and full analysis share an extraction defect and both omit a generated consumer. Reported edges remain accurate, while impact analysis misses a required change.

**Minimal correction:** Freeze oracle-positive and oracle-negative fact sets, including expected omissions and unknown classifications. Measure completeness against the independent expected set, not returned facts. Define “material” by consequence before execution. Declare the exact Rust/compiler/configuration envelope; clarify that “overload” fixtures mean applicable Rust resolution ambiguities, not an unsupported language feature.

### HYP-003 — Cross-repository compatibility: CONCERNS

**Strengths:** `:35,47,53` preserve qualified identity, portable delivery, mixed-generation rejection, missing-source incompleteness and access restrictions.

**Observed gap:** The explicit mutation is a required field/default/serializer change. PROB-107`:57` recognizes that unchanged signatures can hide behavioral incompatibility, but HYP-003`:47` does not require that case or a defined rollout-version matrix.

**[INFERENCE] Missed failure:** Producer and consumer agree on schema and field types, yet a value changes units or an acknowledgment changes meaning. Schema-oriented fixtures pass although an older consumer behaves incorrectly.

**Minimal correction:** Add one unchanged-schema semantic incompatibility and one harmless structural change. Freeze relevant producer/consumer combinations, rollout order and backward/forward message expectations. Require an explicitly bounded consumer inventory; do not infer completeness from discovered consumers alone.

### HYP-004 — Test evidence and CI trust: CONCERNS

**Strengths:** `:35,47,53` preserve the six test-quality distinctions and cover zero tests, stale/replayed reports, policy weakening and mutation canaries. NOTE-067`:92,94` further separates evidence, analysis completeness and acceptance.

**Observed gap:** `:47` assumes a “trusted isolated external executor or qualified CI producer” without an explicit fixture for a trustworthy envelope enclosing an untrusted report.

**[INFERENCE] Missed failure:** A protected workflow signs or uploads attacker-authored JUnit with correct revision metadata. Signature and digest checks pass, but the asserted tests never ran.

**Minimal correction:** Freeze the trust boundary and report-production chain, not merely runner identity. Add report substitution, contradictory exit/count/report, and unauthorized policy-change cases. Bind accepted evidence to protected policy, subject revision/configuration, run/attempt and authorized producer. Verify missing required checks remain non-passing even when impact analysis recommends fewer tests.

This is a protocol extension, not a claim that executor compromise can be comprehensively detected.

### HYP-005 — Legacy oracle and rollback: CONCERNS

**Strengths:** `:35,47,53` include state, errors, effect order, serialization, idempotency, independent held-out cases and approved divergences. They reject leaf topology and branch counts as parity proof.

**Observed gap:** “Switching/recovery” at `:47` does not explicitly require rollback **after new-version writes or external effects**, or define irreversible boundaries.

**[INFERENCE] Missed failure:** Switching back succeeds before traffic, but after new data is written the old implementation cannot read it, or replay duplicates a payment/event.

**Minimal correction:** Freeze a bounded state-transition and effect ledger: initial state, writes during cutover, in-flight/retried work, point of no return, and permitted recovery or compensation. Exercise at least one post-write rollback/recovery case. Freeze nondeterminism normalization and approved differences before evaluating replacement output.

### HYP-006 — Storage candidate: CONCERNS

**Strengths:** `:35,47,53,57` treat SQLite as a candidate, preserve canonical data and federation contracts, and require comparison with retained-Lance improvements. No automatic WAL freshness or vector-speed claim remains.

**Observed gap:** The protocol names evidence retention and recovery, but not a complete ownership inventory or acceptance comparison for each durable record class.

**[INFERENCE] Missed failure:** A rebuilt projection restores artifacts and search results but loses an accepted code link, exception expiry or machine evidence bundle absent from the recovery fixture.

**Minimal correction:** Freeze a storage-class inventory identifying canonical authority, rebuildable projection, receipt and external truth. Verify retained identities/content/history for every in-scope durable class across publish failure, upgrade and recovery. Define operational-cost horizon and migration cost alongside build size and query performance; faster queries alone cannot establish lower total cost.

### HYP-007 — Triage and linked views: CONCERNS

**Strengths:** `:41,47,53` reject a hard first-page cap, require accessibility and include backlog aging, missed risks and simulated actions.

**Observed gap:** “Test backlog aging” does not define arrival conditions or an observation horizon. The claim at `:35` also permits isolated independent work without requiring a case where missing dependency information makes independence uncertain.

**[INFERENCE] Missed failure:** A finite seeded backlog is eventually reviewed, but continuous new high-ranked cards indefinitely displace an older unresolved risk. Separately, a missing cross-repository edge is mistaken for isolation.

**Minimal correction:** Include sustained arrivals and a predeclared visibility/escalation bound for unresolved items. Test unknown dependencies, not only known blockers. Pin comparison data across list/diff/2D/3D and agent views; stale or omitted information must not silently produce different acceptance decisions.

### HYP-008 — Adoption and differentiation: CONCERNS

**Strengths:** `:37,41,47,53` distinguish owner demand from market evidence, recognize existing competitors, seek non-adoption reasons and require repeated use rather than enthusiasm.

**Observed gap:** Recruitment counts, segment mix and horizon remain TBD; “matching proposed segments” and “enough users” lack operational eligibility and adoption definitions.

**[INFERENCE] Missed failure:** Enthusiastic early adopters repeat a supported prototype task while setup assistance, integration maintenance, privacy constraints or abandoned users are excluded from cost and adoption calculations.

**Minimal correction:** Freeze problem-based recruitment criteria, separate user and purchasing/deployment decision roles, record refusals and attrition, and define repeat use over a meaningful work cycle. Compare equivalent alternatives and include onboarding, assistance and maintenance costs. Report segment-specific evidence; one favorable cohort cannot substantiate the whole market claim.

## 4. Cross-cutting interactions and gaps

### C2 — Reading order and generated documentation lack explicit negative-case ownership

**Observed:** NOTE-067`:90,137–141` retains reading routes, mandatory constraints, generated factual documentation and manual ownership. The HYP protocols do not explicitly assign tests for their failure modes; HYP-001`:47` remains task-level.

**[INFERENCE] Consequence:** Aggregate task success could conceal a reading route that omits a mandatory ADR or a documentation generator that overwrites intended behavior with a current defect.

**Minimal correction:** Add explicit test cells to the existing value/semantic protocols for budget truncation, cycles, mandatory omitted constraints, stale documentation and preserved manual/normative content. A new hypothesis is not inherently necessary.

### C3 — Individual success must not substitute for an integrated acceptance result

**Observed:** HYP-002,003,004,005,006,007 test different layers. NOTE-067`:86–98` supplies the common revision/configuration/trust requirements, but no protocol explicitly exercises their composition.

**[INFERENCE] Consequence:** A correct graph, authentic test report and accurate UI can still yield a wrong decision if each refers to a different subject, configuration or policy generation.

**Minimal correction:** Add a shared acceptance-envelope check joining subject/source identities, revisions/dirty digests, configuration, analyzer version, policy version and evidence references. Deliberate mismatches must remain incomplete or rejected through storage, federation and presentation. Reuse this check across hypotheses; do not multiply nominally independent evidence from one shared run.

### C4 — Hypothesis outcomes require explicit non-transfer rules

HYP-001 task benefit, HYP-007 interface benefit and HYP-008 adoption may share participants or tasks. HYP-004 authentic execution is necessary but insufficient for HYP-005 parity. HYP-006 backend preference is not a prerequisite for every analytical capability.

**Minimal correction:** State those boundaries in the experiment matrix. Shared observations are allowed, but cannot count as independent corroboration or prove a different claim.

### Context quality

NOTE-067 and PROB-107 correctly preserve:

- Canonical records versus derived indexes, receipts and external state.
- Revision/configuration provenance and explicit unknown/stale status.
- Declaration, discovery, execution, coverage, assertion strength and scenario/parity distinctions.
- Protected CI policy and external execution ownership.
- Full Rust-core, federation, legacy, documentation and UI/Tauri/optional-Hub target.
- Vendor findings as attributed research, not ForgePlan measurements.
- R_eff as an evidence-chain assessment, not correctness probability or deployment authority.

Their content verdict is **CONCERNS**, through C1–C4 and the baseline correction, rather than a fundamental framing failure.

## 5. Metadata and formal gate

**Observed in files:** All ten artifacts are draft. HYP frontmatter contains generated `hypothesis_state: inferred` at `:6` and `:26`; bodies explicitly call the claims unvalidated at `:37`. NOTE-067`:181` records the unavailable lifecycle reader. Generated metadata is **not** a performed promotion or supporting evidence.

**Reported by handoff, not rerun:** Disk exhaustion, HYP-008 `index_pending`, incomplete NOTE relations, calibration/validation/export NOT_RUN, and failed canonical lifecycle access. NOTE-067`:173` must not be read as a completed validation/export receipt.

**Formal readiness remains BLOCKED**, independently of content corrections. No approval for PRD authoring, activation or implementation follows from this review. Canonical completion, validation, designated evidence recording and the required independent review rounds remain necessary.

No files were changed or written. No runtime experiments, builds, tests, formatters, cleanup, lifecycle mutations or commits were performed.

**Signed:** `DiscoveryHypothesisReview` — `openai-codex/gpt-6-astra`