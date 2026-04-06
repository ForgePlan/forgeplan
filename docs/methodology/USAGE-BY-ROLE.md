[English](USAGE-BY-ROLE.md) · [Русский](USAGE-BY-ROLE.ru.md)

# ForgePlan — Usage Guide by Role

## Quick Start by Role

### Human: Product Manager / Product Owner

```
Your flow:
  1. forgeplan new prd "User Authentication"    ← Describe WHAT and WHY
  2. forgeplan new epic "Auth System" --prd 1   ← Group into an initiative
  3. Fill in: Problem, Users, Goals, Metrics
  4. forgeplan validate --type prd              ← Completeness check
  5. forgeplan status                           ← Dashboard

Artifacts: PRD, Epic
Don't touch: RFC, ADR, Spec (that's for engineers)
```

### Human: Tech Lead / Architect

```
Your flow:
  1. Read PRD → understand WHAT is needed
  2. forgeplan new spec "OAuth2 API" --prd 1    ← Formal contracts
  3. forgeplan new rfc "Auth Architecture" --prd 1  ← Architecture
  4. forgeplan new adr "Choose Passport.js" --rfc 1 ← Decision + rationale
  5. forgeplan score                            ← R_eff across all artifacts
  6. forgeplan coverage                         ← Blind modules

Artifacts: Spec, RFC, ADR
Read: PRD (from PM)
Don't touch: PRD (only comment)
```

### Human: Developer

```
Your flow:
  1. forgeplan status                           ← What's in progress?
  2. forgeplan context src/auth/                ← Which decisions cover this?
  3. Code according to ADR
  4. forgeplan drift                            ← Did my changes violate a decision?
  5. forgeplan new note "Edge case found"       ← Note for the future

Artifacts: Notes
Read: ADR (what was decided), Spec (contracts), RFC (why it's done this way)
```

### Human: QA / Reviewer

```
Your flow:
  1. forgeplan context {changed_files}          ← Which decisions are affected?
  2. forgeplan drift                            ← Any violations?
  3. forgeplan score                            ← R_eff < 0.5 = worth checking
  4. forgeplan validate                         ← Are all artifacts complete?

Check: drift, coverage, R_eff scores
```

---

## AI Agent Flows

### Agent: Research / Discovery

```
Trigger: user says "research", "investigate", "what do we have"

Flow:
  1. forgeplan status → check current artifacts
  2. forgeplan context {topic} → what's already decided on the topic
  3. /research or /deep-research → investigate
  4. Result → memory_retain() + forgeplan new note

Rule: NEVER create PRD/RFC/ADR without user request.
Only Notes and Research reports.
```

### Agent: Architecture / Design

```
Trigger: user says "design", "architecture", "how to build"

Flow:
  1. forgeplan status → is there a PRD?
     - No PRD → suggest: "A PRD is needed first. Create one?"
     - PRD exists → read requirements
  2. ADI cycle:
     a. Abduction: 3+ hypotheses (MANDATORY)
     b. Deduction: logical verification of each
     c. Induction: practical verification (tests, prototypes)
  3. Present options to user
  4. User chooses → forgeplan new adr

Rule: TRANSFORMER MANDATE — agent proposes, human decides.
Agent NEVER records an ADR without human approval.
```

### Agent: Implementation / Sprint

```
Trigger: user says "implement", "build", /sprint

Flow:
  1. forgeplan context {scope} → which ADRs cover the work area
  2. Read ADR → follow decisions
  3. Code according to Spec (contracts)
  4. After coding: forgeplan drift → check for decision violations
  5. If drift → notify user

Rule: code MUST conform to ADR. If deviation is needed →
new RFC → new ADR → only then change code.
```

### Agent: Review / Audit

```
Trigger: /audit, code review, PR

Flow:
  1. forgeplan context {changed_files}
  2. For each file: is there a covering ADR?
     - No → mark as "blind module"
     - Yes → verify conformance
  3. forgeplan score → R_eff for affected artifacts
  4. forgeplan drift → any violations?
  5. Report: compliance + gaps + drift

Rule: Adversarial Review — MUST find at least 1 problem.
Zero findings = incomplete review.
```

---

## Depth Calibration — When to Use What

| Complexity | Depth | Artifacts | Example |
|------------|-------|-----------|---------|
| **Tactical** | 1 call | Note | Quick fix, typo, config change |
| **Standard** | Frame → Explore → Decide | ADR | Library choice, API design |
| **Deep** | Full ADI + Evidence | PRD → RFC → ADR | New feature, architectural decision |
| **Critical** | Formal verification | Epic → PRD → Spec → RFC → ADR | Migration, security, compliance |

### Routing Rules (for the agent)

```
IF task < 1 file AND reversible in 1 hour → Tactical (Note)
IF task < 5 files AND reversible in 1 day → Standard (ADR)
IF task > 5 files OR new module → Deep (PRD → RFC → ADR)
IF task involves security OR data OR infrastructure → Critical (full cycle)
```

## Related Documents

- [FORGEPLAN-GUIDE.md](FORGEPLAN-GUIDE.md) — Complete practical guide
- [DEPTH-CALIBRATION.md](DEPTH-CALIBRATION.md) — Depth levels in detail
- [HOW-TO-USE.md](HOW-TO-USE.md) — 10 rules of the methodology
