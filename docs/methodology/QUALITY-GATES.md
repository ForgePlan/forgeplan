[English](QUALITY-GATES.md) · [Русский](QUALITY-GATES.ru.md)

# Quality Gates — Verification Gate + Adversarial Review

A guide to decision quality checks in Forgeplan. Combines the Verification Gate (Quint-code), Adversarial Review (BMAD), 13-Step PRD Validation (BMAD), and R_eff Quality Scoring (Quint-code/FPF).

These are review protocols, not a claim that the CLI automatically enforces
every semantic criterion below. `validate` reports implemented schema findings;
activation separately checks length, evidence and stubs. Zero MUST findings is
not sufficient proof of activation readiness.

These are local adaptations, not verbatim upstream standards. Current FPF B.3
does not prescribe universal numeric penalties; its G is ClaimScope, whereas
Forgeplan's FGR uses Granularity. See [GLOSSARY.md](GLOSSARY.md).

## 1. Verification Gate (5 Points)

Before closing any decision (DecisionRecord, ADR, RFC), check all 5 points. This is a safeguard against self-deception and confirmation bias.

### 1.1. Deductive Consequences

> What must be true if the decision is correct?

If decision X is correct, then conditions Y, Z, W must necessarily hold. Verify each one. If even one does not hold, the decision is in question.

**Example**: If we decided to use LanceDB, then the following must be true:
- LanceDB supports our data types (structured + vectors)
- Performance on 10K artifacts is acceptable (<100ms per query)
- A stable Rust SDK exists

### 1.2. Strongest Counter-Argument

> A genuine (not strawman) counter-argument against the decision.

Formulate the strongest argument AGAINST the chosen option. A strawman (deliberately weak argument) does not count — a real risk is needed.

**Example**: "LanceDB is a young project; its API may break between versions. SQLite has been stable for 20+ years."

### 1.3. Self-Evidence Check

> Is all evidence from this session only? Review independence and congruence.

Treat single-session evidence cautiously and seek independent confirmation. A CL1 assignment here is a local review recommendation, not an automatic parser rule or an upstream equation.

**Penalty**: CL1 = 0.4 penalty to R_eff score.

### 1.4. Tail Failure Scenarios

> Scenarios with <10% probability each, but catastrophic consequences.

List 2-3 "what if everything goes wrong" scenarios. Assess: can we survive them? Is there a rollback plan?

**Hypothetical examples** (probabilities are illustrative, not measurements):
- LanceDB development is abandoned (~5%) → retain canonical Markdown and plan an index migration
- BGE-M3 is removed from HuggingFace (~2%) → offline operation remains possible only with a complete compatible cached snapshot; weights are **not** embedded in the binary

### 1.5. WLNK Challenge — Weakest Link Verification

> Is this really the weakest link, or is there a hidden one?

R_eff = min(evidence_scores). Make sure you have identified the REAL weakest link. Often the actual risk is hidden behind what seems obvious.

**Question**: "I believe the weakest link is performance. But could the real problem be developer experience or migration of existing data?"

## 2. Adversarial Review Protocol

Review protocol applied at Deep and Critical levels. Governed by core policy [ADR-027 — Finite evidence-backed independent review without a defect quota](../../.forgeplan/adrs/ADR-027-finite-evidence-backed-independent-review-without-a-defect-quota.md); cutover plan and acceptance live in [RFC-014 — Core review policy, bounded documentation cutover](../../.forgeplan/rfcs/RFC-014-core-review-policy-bounded-documentation-cutover.md). This section is the **single normative core protocol**; short definitions and instructions elsewhere link here.

> **Scope note (core policy, not propagated downstream).** This section governs the owning Forgeplan core / source docs. The shipped binary, installed plugin copies, public website surfaces and historical artifacts (session snapshots, `docs/vnext/**` audits, instantiated template bodies) keep their prior text until those surfaces are independently updated. This change does not modify the Rust validator, CLI/MCP runtime or the template embedded in a published binary, and is not an automated downstream propagation.

### Core rules

1. **Zero real defects is neither auto-PASS nor auto-FAIL.** A zero count alone proves neither thoroughness nor shallowness. The reviewer **MUST** actively search for counterexamples and contradictions before accepting a zero outcome. A zero verdict is permitted only when every condition in §D1 holds on the current, complete candidate.
2. **Mandatory active counterexample search.** The reviewer reads the artifact, adjacent docs, prior art and adverse scenarios specific to the target. Passively looking for confirmations is not a review.
3. **Mandatory structured reporting.** Every finding carries severity (`Critical`, `HIGH`, `MEDIUM`, `LOW`, `Warning`, `Pass`), exact location (section / line / range), consequence, and current state (`open`, `fixed`, `authorized-justified-acceptance`, `blocked-external`).
4. **Unresolved `HIGH` or `Critical` blocks the review** (`BLOCKER`). The artifact does not advance to the next gate while such findings remain `open`.
5. **`Warning` / `MEDIUM` / `LOW` are not erased by the zero-defect clause.** They keep their fix-or-owner-authorized-justification path. The reviewer never accepts a risk on the owner's behalf.
6. **The reviewer never invents or duplicates findings.** Re-spawning solely to manufacture a non-zero count is forbidden; repeating the review solely because the count is zero is forbidden.
7. **Generator ≠ verifier.** Authored analyses, owner-marked approvals and successful structural validation are not independent verdicts.

### D1. Bounded zero-defect PASS — seven conditions

An independent reviewer **MAY** end the review at **PASS with zero real defects** only when every condition below holds on the current, complete candidate.

1. **Independence.** Generator and verifier differ. The report names author, reviewer, roles, assigned model / effort and the limits of any independently confirmable identity. Authored analyses, owner-marked approvals and successful structural validation are not an independent verdict.
2. **Current version.** Owning workspace, exact target object, scope, criteria and exclusions are named. For an artifact: canonical slug, full body SHA-256, status / depth and significant links are pinned. For a tree: baseline commit plus digest of the actual candidate contents, including dirty files. A bare `HEAD` without working-copy contents is insufficient. The reviewer personally re-verifies currency at the moment of verdict.
3. **Coverage.** Each mandatory criterion maps to a check, a result and an evidence reference, or to a documented inapplicability. An unchecked mandatory criterion does not become N/A for PASS. The strongest counterargument, material boundaries and negative cases are recorded, not only the happy path.
4. **Evidence sufficiency.** Origin, retained excerpt / result and digest (or stable link) are listed. For executable actions: real commands, environment, observed results / exit codes, and execution volume are recorded; non-zero execution is distinguished from "nothing ran". A foreign report is marked as foreign; a guess is marked as inference. Observed, inferred, skipped, and unavailable are not mixed. A hash confirms identity, not correctness, of the cited content.
5. **Findings and limits.** All real findings remain in the report with severity, line / section reference, consequence and current state. Unresolved `Critical` / `HIGH` means **BLOCKER**. `Warning` / `MEDIUM` / `LOW` keep their fix-or-owner-authorized-justification path; the reviewer never accepts the risk on the owner's behalf. Significant unresolved doubts about coverage or evidence turn a zero count into BLOCKER or CONCERNS, not PASS.
6. **Preserved gates.** All required reviewer counts and rounds, validation / ADI / Verification Gate / R_eff and profile-specific checks in their domains are obtained. Guardian, security, runtime, consent and activation requirements are satisfied where they apply to the claimed gate. The report does not pass a partial result off as the whole pipeline.
7. **Finite report.** The reviewer explicitly names what passed, what was not checked, what external conditions remain, and why that is sufficient for exactly the claimed scope. A bare "all good" without items 1–6 is not a finished review.

### D2. Re-review and finiteness

A new independent review is required when the reviewed version, criteria or evidence changed in a relevant way, real findings were fixed, a meaningful check was skipped, or evidence is missing or insufficient. The reviewer re-checks the fix and possible new defects. Irrelevant editorial edits require a recorded reason for keeping the prior verdict applicable, not silent reuse.

The mandatory **Critical ≥ 2 rounds** remain even when round 1 finds zero issues. The assigned reviewer count and specializations are preserved: in particular **2+ audit agents** and `security-expert` + `code-reviewer` / `reviewer` under the existing `CLAUDE.md` trigger (≥5 commits OR new public API). This protocol removes from that block only the quota phrase "Each MUST find ≥3 issues; Zero findings → re-spawn", not its trigger and not the roles. For Deep, no new reviewer count replaces the one already applicable. When real problems exist, the number of repeat rounds is determined by closure of the problem and the existing gates — not by an artificial limit. Re-reviewing solely to reach a non-zero count is forbidden.

### D3. Strict scoped PASS

A design-only PASS applies to completeness of the decision project when runtime is explicitly excluded from its criteria; it is **not** a runtime / implementation / publication / activation PASS. When runtime is in scope, skipping it forbids such PASS. A missing producer or accepted contract is not a manufactured defect and not a produced evidence; the external dependency may be left to the closing implementation gate, but the whole gate cannot be declared passed. This change applies to core policy and source docs only; shipped binary, installed copies, and historical artifacts retain prior text until they are independently updated.

### D4. Owner-approved quota-only precedence and history boundary

The owner has approved an amendment to the defect-count quota for this core section, and that decision is recorded now. This **draft** does not declare itself already in force and does not let the author close P1 by themselves. After actual bounded application and independent verification, with orchestrator lifecycle actions, this ADR and the current `docs/methodology/QUALITY-GATES.md` §2 / RU §2 take priority on the defect-quota question over older quota phrases, historical artifacts, session snapshots, `docs/vnext/**` audits and installed third-party copies. Until then, narrow owner amendment-review authority supports reviewing this amendment only while pending; there is no automatic source / activation / runtime authority. All other existing gates — validation / ADI / Verification Gate / R_eff / security / runtime / consent / activation, reviewer counts and roles, Critical rounds, BMAD 13-step, profile-specific requirements — keep their prior force. Stale quota phrases in installed / website / Marketplace / session snapshots are not rewritten in bulk.

### Process

```
1. Author prepares the artifact (PRD, RFC, ADR, Spec).
2. Reviewer re-pins canonical slug, full body SHA-256, status / depth,
   significant outbound links and (for trees) working-copy digest.
3. Reviewer actively searches for counterexamples and contradictions,
   not confirmations.
4. Reviewer compiles a list of findings: severity, location, consequence, state.
5. Verdict per D1.7 and D2:
   - all applicable criteria covered AND evidence sufficient AND required rounds done AND preserved gates satisfied -> PASS
     (retained nonblocking Warning / MEDIUM / LOW findings fixed or covered by owner-authorized justification; optional nondefect observations do not block; zero real defects is permitted but neither necessary nor sufficient by count alone; the reviewer never accepts owner-authorized risk)
   - any unresolved Critical / HIGH -> BLOCKER
   - any missing mandatory coverage / evidence / preserved gate -> CONCERNS or BLOCKER by impact, never PASS
   - otherwise -> CONCERNS
6. Author handles each finding:
   - Critical / HIGH -> fixes
   - Warning / MEDIUM / LOW -> fixes or records owner-authorized justification
   - Pass -> at discretion
7. Independent re-review of fixes (mandatory for Critical, ≥ 2 rounds total).
```

### Severity classification

| Severity | Description | Action |
|----------|-------------|--------|
| **Critical / HIGH** | Blocks implementation or fundamental defect | Must fix before proceeding; unresolved = BLOCKER |
| **MEDIUM / Warning** | Material concern, incompleteness, ambiguity | Fix or record owner-authorized justification |
| **LOW** | Local concern, clarity, style | Fix or justify |
| **Pass** | Observation, improvement, stylistic | At the author's discretion |

### For Critical level — multiple rounds

At the Critical level a minimum of 2 independent rounds of Adversarial Review are conducted. The second round focuses on:
- Re-pinning current version and evidence after the first-round changes.
- Verifying that first-round findings are actually fixed (not silently downgraded).
- Searching for problems introduced by the fixes.
- Cross-section consistency.

```
1. Author prepares the artifact (PRD, RFC, ADR, Spec)
2. Reviewer receives the artifact
3. Reviewer SEARCHES for problems (not confirmations)
4. Reviewer compiles a list of findings with severity
5. If findings = 0 -> record scope and remaining unexamined risks
6. Author addresses each finding:
   - Critical -> fixes it
   - Warning -> fixes or documents justification
   - Pass -> at their discretion
7. Re-review of fixes (for Critical items)
```
8. Zero findings in round 1 does not skip round 2.

### Related canonical references

- Core policy: [ADR-027 — Finite evidence-backed independent review without a defect quota](../../.forgeplan/adrs/ADR-027-finite-evidence-backed-independent-review-without-a-defect-quota.md)
- Source cutover plan and acceptance: [RFC-014 — Core review policy, bounded documentation cutover](../../.forgeplan/rfcs/RFC-014-core-review-policy-bounded-documentation-cutover.md)
- This section is the single normative core protocol. Short definitions and instructions elsewhere link here.

## 3. BMAD 13-Step PRD Validation

The table is a local summary of the numbered BMAD validation workflow, applied at Deep and Critical levels. Thirteen validation steps are not thirteen required PRD sections or thirteen automatically enforced Rust rules.

| Step | Name | What It Checks |
|------|------|----------------|
| 1 | **Discovery & Confirmation** | Document type is correctly identified, task is clear |
| 2 | **Format Detection & Structure** | Structure matches the template, all sections are present |
| 3 | **Information Density** | No filler/fluff, every sentence carries meaning |
| 4 | **Product Brief Coverage** | Product brief is fully covered: problem, audience, goals |
| 5 | **Measurability** | FR and NFR are testable and measurable (numbers, metrics) |
| 6 | **Traceability** | Chain: Summary -> Criteria -> User Journeys -> FRs is traceable |
| 7 | **Implementation Leakage** | No framework names, libraries, or technologies in requirements |
| 8 | **Domain Compliance** | Domain requirements are considered (healthcare, fintech, etc.) |
| 9 | **Project-Type Compliance** | Project type specifics are considered (API, mobile, web, CLI) |
| 10 | **SMART Requirements** | Specific, Measurable, Attainable, Relevant, Traceable |
| 11 | **Holistic Quality Assessment** | Overall 1-5 rating on document quality |
| 12 | **Completeness** | No template variables (`{{placeholder}}`), all sections filled |
| 13 | **Report Finalization** | Final report with recommendations and action items |

### Partial Application (for Standard Level)

At the Standard level, only 3 steps are required:
- **Step 3** (Information Density) — remove filler
- **Step 5** (Measurability) — FR/NFR must be testable
- **Step 7** (Implementation Leakage) — requirements without implementation ties

## 4. R_eff Quality Scoring

A decision quality scoring system from Quint-code. Core principle: **trust in a decision = its weakest link**.

### Formula

```
R_eff = min(evidence_scores)
```

**NEVER the average.** A single weak piece of evidence brings down the entire score.

**Which evidence participates**: terminal packs (`superseded`/`deprecated`) are excluded from the minimum (ADR-020); draft evidence counts. An **unexpired** `refutes` contributes zero. Honest displacement preserves history: link re-verification evidence, then supersede the old pack.

### Evidence Score Calculation

```
flat_evidence_score = max(0, verdict_score - CL_penalty)
recursive_evidence_score = max(0, verdict_score - CL_penalty - evidence_type_modifier)
```

The explicit recursive score report uses the second formula plus eligible
dependency contributions (`based_on`/`informs`: 0.1 penalty; `refines`: 0).
Type modifiers: test/measurement 0, benchmark 0.1, audit 0.2. Flat captured
`r_eff_score` and the evidence breakdown omit type modifiers and recursion, so
they can differ from the report. Neither formula is `min(F,G,R)`.

### Verdict Scores

| Verdict | Score | Description |
|---------|-------|-------------|
| `supports` | 1.0 | Evidence confirms the decision |
| `weakens` | 0.5 | Evidence weakens confidence |
| `refutes` | 0.0 | Evidence refutes the decision |

### CL Penalties (Congruence Level)

| Congruence Level | Penalty | Description |
|------------------|---------|-------------|
| CL3 | 0.0 | Same context, internal test |
| CL2 | 0.1 | Similar context, related project |
| CL1 | 0.4 | Different context, external documentation |
| CL0 | 0.9 | Opposite context |

### Evidence Decay

`valid_until` is optional. When present and expired:
- Evidence is NOT deleted
- Score becomes **0.1** (stale, not absent)
- This differs from 0.0 — stale evidence is better than no evidence at all
- Expiry returns 0.1 **before** verdict arithmetic, including for refuting evidence; no gradual time-decay curve is implemented here.

### Trust Thresholds

| R_eff | Status | Action |
|-------|--------|--------|
| >= 0.5 | Adequate | Decision can be accepted |
| < 0.5 | Needs Review | Additional evidence or reconsideration required |
| < 0.3 | AT RISK | Decision is unreliable, reassessment needed |

These are interpretation labels, not sufficient activation conditions or
calibrated probabilities.

### Calculation Example

Decision: "Use LanceDB for artifact storage"

Illustrative **flat-score** evidence pack (not observed measurements or the recursive report):
1. Benchmark on 10K records: supports (1.0), CL2 -> score = 1.0 - 0.1 = **0.9**
2. LanceDB Rust SDK documentation: supports (1.0), CL1 -> score = 1.0 - 0.4 = **0.6**
3. Production usage reviews: weakens (0.5), CL1 -> score = 0.5 - 0.4 = **0.1**

```
R_eff = min(0.9, 0.6, 0.1) = 0.1 — AT RISK
```

The weakest link is production usage reviews. Additional evidence (CL2+) or reconsideration is needed.

## 5. When to Apply Each Check

| Depth Level | Verification Gate | Adversarial Review | 13-Step Validation | R_eff Scoring |
|-------------|-------------------|--------------------|--------------------|---------------|
| **Tactical** | No | No | No | No |
| **Standard** | Yes (3 of 5 points) | No | Partial (steps 3, 5, 7) | Optional |
| **Deep** | Yes (all 5 points) | Yes | Full 13 steps | Yes |
| **Critical** | Yes (all 5 points) | Yes, multiple rounds | Full 13 steps + domain | Yes, mandatory |

### Which Verification Gate Points at Standard Level

At the Standard level, a minimum of 3 of 5 points are required. Recommended:
1. **Deductive consequences** — always useful
2. **Strongest counter-argument** — protection against confirmation bias
3. **WLNK challenge** — identify the real weakest link

Points 3 (Self-evidence check) and 4 (Tail failure scenarios) are optional at Standard but recommended whenever there is any doubt.

## Related Documents

- [DEPTH-CALIBRATION.md](DEPTH-CALIBRATION.md) — when to use each depth level
- [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) — decision tree: which document to create
- [ARTIFACT-MODEL.md](ARTIFACT-MODEL.md) — artifact hierarchy and lifecycle
