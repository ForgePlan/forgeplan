[English](QUALITY-GATES.md) · [Русский](QUALITY-GATES.ru.md)

# Quality Gates — Verification Gate + Adversarial Review

A guide to decision quality checks in Forgeplan. Combines the Verification Gate (Quint-code), Adversarial Review (BMAD), 13-Step PRD Validation (BMAD), and R_eff Quality Scoring (Quint-code/FPF).

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

> Is all evidence from this session only? -> CL1 penalty.

If all evidence was obtained within a single session (one conversation, one PoC), this is CL1 — limited congruence. External confirmations are needed: documentation, benchmarks from other projects, production experience.

**Penalty**: CL1 = 0.4 penalty to R_eff score.

### 1.4. Tail Failure Scenarios

> Scenarios with <10% probability each, but catastrophic consequences.

List 2-3 "what if everything goes wrong" scenarios. Assess: can we survive them? Is there a rollback plan?

**Example**:
- LanceDB development is abandoned (probability ~5%) -> data is in the open Lance format, migration is possible
- Embedding model BGE-M3 is removed from HuggingFace (~2%) -> model is embedded in the binary, works offline

### 1.5. WLNK Challenge — Weakest Link Verification

> Is this really the weakest link, or is there a hidden one?

R_eff = min(evidence_scores). Make sure you have identified the REAL weakest link. Often the actual risk is hidden behind what seems obvious.

**Question**: "I believe the weakest link is performance. But could the real problem be developer experience or migration of existing data?"

## 2. Adversarial Review Protocol

A review protocol from BMAD-METHOD. Applied at Deep and Critical levels.

### Core Rules

1. **The reviewer MUST find problems.** This is not optional — if the reviewer found zero problems, the review was superficial.

2. **0 problems found = repeat the review.** Zero issues indicates approval bias. The reviewer must re-conduct the review with heightened attention.

3. **Severity classification:**

| Severity | Description | Action |
|----------|-------------|--------|
| **Critical** | Blocks implementation, fundamental defect | Must fix before proceeding |
| **Warning** | Potential issue, incompleteness, ambiguity | Fix or justify why it stays |
| **Pass** | Observation, improvement, stylistic | At the author's discretion |

4. **Each finding must reference a specific section or line.** Abstract remarks ("should improve") are not accepted.

### Adversarial Review Process

```
1. Author prepares the artifact (PRD, RFC, ADR, Spec)
2. Reviewer receives the artifact
3. Reviewer SEARCHES for problems (not confirmations)
4. Reviewer compiles a list of findings with severity
5. If findings = 0 -> repeat review
6. Author addresses each finding:
   - Critical -> fixes it
   - Warning -> fixes or documents justification
   - Pass -> at their discretion
7. Re-review of fixes (for Critical items)
```

### For Critical Level — Multiple Rounds

At the Critical level, a minimum of 2 rounds of Adversarial Review are conducted. The second round focuses on:
- Verifying fixes from the first round
- Finding problems introduced by the fixes
- Cross-section consistency

## 3. BMAD 13-Step PRD Validation

Full PRD validation across 13 steps from BMAD-METHOD. Applied at Deep and Critical levels.

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

### Evidence Score Calculation

```
evidence_score = max(0, verdict_score - CL_penalty)
```

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

Each evidence item has a `valid_until` field (TTL). After expiration:
- Evidence is NOT deleted
- Score becomes **0.1** (stale, not absent)
- This differs from 0.0 — stale evidence is better than no evidence at all

### Trust Thresholds

| R_eff | Status | Action |
|-------|--------|--------|
| >= 0.5 | Adequate | Decision can be accepted |
| < 0.5 | Needs Review | Additional evidence or reconsideration required |
| < 0.3 | AT RISK | Decision is unreliable, reassessment needed |

### Calculation Example

Decision: "Use LanceDB for artifact storage"

Evidence pack:
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
