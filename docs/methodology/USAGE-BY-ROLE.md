[English](USAGE-BY-ROLE.md) · [Русский](USAGE-BY-ROLE.ru.md)

# ForgePlan — Usage Guide by Role

Examples target Forgeplan **0.38.0**. IDs below illustrate an empty workspace;
use the IDs returned by `new`. These are suggested responsibilities, not
access-control rules. Fill bodies with `forgeplan update ID --body @file.md`
or MCP, not by editing the `.forgeplan/` projection.

## Quick Start by Role

### Human: Product Manager / Product Owner

```
Your flow:
  1. forgeplan new prd "User Authentication"    ← Describe WHAT and WHY
  2. forgeplan new epic "Auth System"          ← Group into an initiative
  3. Fill Problem, Target Users, Goals, FR; relate PRD/Epic via CLI/MCP links
  4. forgeplan validate PRD-001                ← Check this PRD (omit ID for all)
  5. forgeplan status                           ← Dashboard

Artifacts: PRD, Epic
Coordinate RFC, ADR and Spec changes with engineers.
```

### Human: Tech Lead / Architect

```
Your flow:
  1. Read PRD → understand WHAT is needed
  2. forgeplan new spec "OAuth2 API"            ← Formal contracts
  3. forgeplan new rfc "Auth Architecture"      ← Architecture
  4. forgeplan new adr "Choose an auth library" ← Decision + rationale
  5. forgeplan score --all                     ← Active decision artifacts
  6. forgeplan coverage                         ← Blind modules

Artifacts: Spec, RFC, ADR
Read: PRD (from PM)
Coordinate requirement changes with the PM; new has no --prd or --rfc option.
```

Create explicit dependencies after creation, for example
`forgeplan link RFC-001 PRD-001 --relation based_on` and
`forgeplan link ADR-001 RFC-001 --relation based_on`.

### Human: Developer

```
Your flow:
  1. forgeplan status                           ← What's in progress?
  2. forgeplan context ADR-001                 ← Context for an artifact ID
  3. Code according to ADR
  4. forgeplan drift                           ← Changed affected files, not proof of violation
  5. forgeplan new note "Edge case found"       ← Note for the future

Artifacts: Notes
Read: ADR (what was decided), Spec (contracts), RFC (why it's done this way)
```

### Human: QA / Reviewer

```
Your flow:
  1. forgeplan search "authentication"         ← Find relevant decisions
  2. forgeplan context ADR-001                 ← Read artifact, graph, validation, score
  3. forgeplan score ADR-001                   ← Inspect the weakest evidence
  4. forgeplan validate                        ← Check all artifacts

Check: drift, coverage, R_eff scores
```

Use `forgeplan drift` to find files changed since a decision; a reviewer must
determine whether those changes violate it. `context` accepts an artifact ID,
not paths, topics or a list of changed files.

---

## AI Agent Flows

`/research`, `/sprint` and `/audit` below come from the separately installed
`fpl-skills` **1.75.1** plugin. `/forge-cycle` comes from
`forgeplan-workflow` **1.23.1**. The public catalog **1.187.1** has **23 plugins**;
CLI installation and `setup-skill` do not install this whole harness.

### Agent: Research / Discovery

```
Trigger: user says "research", "investigate", "what do we have"

Flow:
  1. forgeplan status → check current artifacts
  2. forgeplan search "authentication" → find existing decisions by topic
  3. /research → investigate with the installed skill
  4. Result → forgeplan new note; retain distilled lessons only if memory is configured and authorized

Rule: NEVER create PRD/RFC/ADR without user request.
Only Notes and Research reports.
```

Hindsight is optional (`fpl-hsmem` **3.7.0**). Configure explicit `url`,
`banks` and `defaultBank` in project `.hindsight.json`; banks must exist on the
server. Recall/retain hooks default off. Check the selected bank and privacy
settings before a permitted `memory_retain`; it is not key-value storage or
the same store as `forgeplan remember`/`recall`.

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
  1. forgeplan search "authentication" → find decisions for the scope
  2. forgeplan context ADR-001 → read the selected decision
  3. Code according to Spec (contracts)
  4. After coding: forgeplan drift → identify affected-file changes
  5. Review any actual deviation and notify the user

Rule: code MUST conform to ADR. If deviation is needed →
new RFC → new ADR → only then change code.
```

### Agent: Review / Audit

```
Trigger: /audit, code review, PR

Flow:
  1. forgeplan search "authentication" → find relevant artifacts
  2. For each changed file, inspect coverage and the actual ADR:
     - No covering decision → report a coverage gap
     - Decision exists → verify conformance
  3. forgeplan score ADR-001 → inspect evidence for the selected artifact
  4. forgeplan drift → inspect changed affected files, not an automatic verdict
  5. Report: compliance + gaps + drift

Rule: Adversarial Review — follow [`QUALITY-GATES.md` §2](./QUALITY-GATES.md#2-adversarial-review-protocol). Zero real defects is permitted only when D1.1-D1.7, mandatory rounds and preserved gates are satisfied on the current complete candidate; the reviewer never invents findings to manufacture a non-zero count and never accepts risk on the owner's behalf; unresolved HIGH / Critical means BLOCKER.
```

Full protocol: [`QUALITY-GATES.md` §2](./QUALITY-GATES.md#2-adversarial-review-protocol).

---

## Depth Calibration — When to Use What

| Depth | Core artifact pipeline | Typical use |
|---|---|---|
| **Tactical** | None required; Note optional | Small reversible fix |
| **Standard** | PRD → RFC | Feature requiring requirements and design |
| **Deep** | PRD → Spec → RFC → ADR | Architectural/security-sensitive work |
| **Critical (methodology)** | Epic and multiple linked decision chains as needed | Cross-team or high-consequence work |

### Routing Rules (for the agent)

Run `forgeplan route "task description"` rather than guessing depth from a
file count. The core rule fallback takes the highest minimum depth among
matched signals; with no signals it defaults to Tactical. LLM routing requires
a configured provider and may propose a different pipeline. The core also
accepts `critical` in `update --depth`, interpreting it as Deep despite the
shorter list in help. There is no separate Critical core pipeline; the extra
review requirements are methodology policy. Generated reasoning and routing
do not replace independent review or verification.

## Related Documents

- [FORGEPLAN-GUIDE.md](FORGEPLAN-GUIDE.md) — Complete practical guide
- [DEPTH-CALIBRATION.md](DEPTH-CALIBRATION.md) — Depth levels in detail
- [HOW-TO-USE.md](HOW-TO-USE.md) — 10 rules of the methodology
