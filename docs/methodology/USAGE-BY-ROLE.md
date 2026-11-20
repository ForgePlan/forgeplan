[English](USAGE-BY-ROLE.md) · [Русский](USAGE-BY-ROLE.ru.md)

# ForgePlan — Usage Guide by Role

Role boundaries are team conventions, not CLI authorization. Use actual IDs
returned by creation; examples assume PRD-001/RFC-001 exist. `context` takes an
artifact ID, not a file path or topic. `drift` reports changed affected files,
not proven violations. CLI `validate` works on legacy and migrated Constellation
layouts; full MCP validation was also exercised.
See the [pilot compatibility notes (RU)](../operations/CONSTELLATION-PILOT.ru.md).

## Quick Start by Role

### Human: Product Manager / Product Owner

```
Your flow:
  1. forgeplan new prd "User Authentication"    ← Describe WHAT and WHY
  2. forgeplan new epic "Auth System"            ← Create initiative; record membership separately
  3. Fill in: Problem, Users, Goals, Metrics
  4. forgeplan validate PRD-001                   ← Completeness check
  5. forgeplan status                           ← Dashboard

Artifacts: PRD, Epic
Don't touch: RFC, ADR, Spec (that's for engineers)
```

### Human: Tech Lead / Architect

```
Your flow:
  1. Read PRD → understand WHAT is needed
  2. forgeplan new spec "OAuth2 API"             ← Formal contracts
  3. forgeplan new rfc "Auth Architecture"        ← Link actual ID to PRD-001 with based_on
  4. forgeplan new adr "Choose Passport.js"       ← Link actual ID to RFC-001 with based_on (the ADR depends on that RFC)
  5. forgeplan score --all                       ← Refresh scores
  6. forgeplan coverage                         ← Blind modules

Artifacts: Spec, RFC, ADR
Read: PRD (from PM)
Don't touch: PRD (only comment)
```

### Human: Developer

```
Your flow:
  1. forgeplan status                           ← What's in progress?
  2. forgeplan context ADR-001                  ← Read the relevant decision and its graph
  3. Code according to ADR
  4. forgeplan drift                            ← Which affected files changed? Review conformance manually
  5. forgeplan new note "Edge case found"       ← Note for the future

Artifacts: Notes
Read: ADR (what was decided), Spec (contracts), RFC (why it's done this way)
```

### Human: QA / Reviewer

```
Your flow:
  1. forgeplan coverage                         ← Locate declared decision coverage
  2. forgeplan drift                            ← Which affected files changed?
  3. forgeplan score --all                      ← R_eff < 0.5 = worth checking
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
  2. forgeplan search "topic" → find relevant artifacts, then context ID
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
  1. forgeplan coverage → find decisions; forgeplan context ADR-001 → read one
  2. Read ADR → follow decisions
  3. Code according to Spec (contracts)
  4. After coding: forgeplan drift → review changes to affected files
  5. If drift → notify user

Rule: code MUST conform to ADR. If deviation is needed →
new RFC → new ADR → only then change code.
```

### Agent: Review / Audit

```
Trigger: /audit, code review, PR

Flow:
  1. forgeplan coverage, then forgeplan context ADR-001 for a relevant decision
  2. For each file: is there a covering ADR?
     - No → mark as "blind module"
     - Yes → verify conformance
  3. forgeplan score ADR-001 → R_eff for the relevant artifact
  4. forgeplan drift → changed affected files, followed by conformance review
  5. Report: compliance + gaps + drift

Native /forge-cycle prescribes an issue quota; that policy is not changed here.
The quota is not evidence that a defect exists. Report confirmed findings and review limits.
```

---

## Depth Calibration — When to Use What

| Complexity | Depth | Artifacts | Example |
|------------|-------|-----------|---------|
| **Tactical** | 1 call | Note | Quick fix, typo, config change |
| **Standard** | Compare moderate-impact approaches | PRD → RFC | Library choice, API design |
| **Deep** | Full ADI + Evidence | PRD → Spec → RFC → ADR | New module, security/public API |
| **Critical** | Strategic authoring convention | Epic → PRD → Spec → RFC → ADR | Multi-team initiative; core parses critical as Deep |

### Routing Rules (for the agent)

```
Illustrative authoring guidance, not the router implementation:
Trivial and reversible → Tactical; moderate alternatives → Standard.
Security, public API or irreversible choice → Deep.
Strategic multi-PRD work → Critical convention; verify route and scope explicitly.
```

## Related Documents

- [FORGEPLAN-GUIDE.md](FORGEPLAN-GUIDE.md) — Complete practical guide
- [DEPTH-CALIBRATION.md](DEPTH-CALIBRATION.md) — Depth levels in detail
- [HOW-TO-USE.md](HOW-TO-USE.md) — 10 rules of the methodology
