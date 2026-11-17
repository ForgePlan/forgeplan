[English](DEPTH-CALIBRATION.md) · [Русский](DEPTH-CALIBRATION.ru.md)

# Depth Calibration — When to Use Each Depth Level

A guide for choosing the appropriate level of decision rigor. Based on Quint-code depth calibration and adapted for Forgeplan.

## Level Overview

In Forgeplan **0.38.0**, runtime `Mode` values are `note`, `tactical`, `standard`, `deep`. Input `critical` maps to `deep`: strategic work can warrant an Epic and a larger hierarchy, but core does not add a distinct Critical gate.

| Level | Decision guidance | Core route pipeline |
|-------|-------------------|---------------------|
| **Note** | Capture a lightweight observation | None |
| **Tactical** | Obvious, reversible, limited change | None; Note optional |
| **Standard** | Multiple approaches, moderate impact | PRD → RFC |
| **Deep** | High stakes, security, migration or long-lived consequences | PRD → SPEC → RFC → ADR |
| **Critical** | Strategic, cross-team initiative | Alias of Deep; Epic hierarchy is methodology guidance |

Duration alone does not determine depth. A short security change can require Deep treatment.

## Decision Tree

```
Obvious and reversible?                 → Tactical
Multiple approaches, moderate impact?   → Standard
Security / migration / broad impact?    → Deep
Strategic initiative with many parts?   → Deep gates + Epic grouping as needed
```

`forgeplan route "task"` proposes a depth and pipeline. `forgeplan calibrate` analyses existing artifact content and reports suggested escalation; it does not rewrite depth or create missing artifacts.

## Automatic Escalation Triggers

These are **calibrate suggestions**, not automatic mutations or guarantees that every risk is detected:

| Content signal | Suggested minimum |
|----------------|-------------------|
| More than 3 checkbox-like lines (`- [` or `* [`) | Standard |
| More than 10 such lines | Deep |
| Security/compliance/authentication heading, or “security considerations” | Deep |
| “breaking change”, `## Migration`, or “backwards compatibility” | Deep |
| More than 2 graph links / more than 5 | Standard / Deep |
| Non-empty `parent_epic` | Standard |
| More than 4 `## ` headings / more than 8 | Standard / Deep |
| More than 50 body lines / more than 200 | Standard / Deep |

The suggestion is the maximum detected minimum; without signals it is Tactical. Review user-data, public-API, reversibility and cross-team risks yourself: those are decision criteria, not a promise that the calibrator understands them.

## Artifact Requirements by Level

### Tactical

- Core route has no artifact pipeline. If creating a PRD anyway, its base PRD MUST sections still apply.
- The Product Brief (`templates/brief/_TEMPLATE.md`) is a lightweight authoring template, not a separate runtime kind or an exemption from PRD validation.
- Evidence is not a prerequisite to making an artifact-free change. If activating a PRD/Epic/SPEC/RFC/ADR, the shared activation evidence gate still applies.
- No automatic 90-day `valid_until` is set by `forgeplan new`; expiry must be supplied deliberately.

### Standard

- Core route recommends PRD + RFC; it does not create or enforce the complete chain at each activation.
- PRD adds target audience MUST; problem density (50 words) and no technology leakage in FR/NFR are SHOULD.
- ADR invariants and rollback are SHOULD; RFC invariants/rollback are COULD.
- Independent review, justified alternatives, measurable outcomes and appropriate evidence remain methodology work, not a “3 of 5” core acceptance rule.

### Deep

- Core route recommends PRD + SPEC + RFC + ADR.
- PRD adds Timeline, Stakeholders, Acceptance Criteria, Risks and measurable Success Metrics MUST; problem density and no implementation leakage become MUST. Rollback and Dependencies are SHOULD.
- RFC adds Risks MUST. ADR invariants and rollback become MUST.
- SPEC accepts either structural contracts or behavioural requirements/scenarios at every depth. The scenario-driven marketplace workflow freezes the oracle before code; core does not require every SPEC to look like a PRD.
- Full evidence, rollback planning and adversarial review are methodology expectations; core validation does not execute tests or certify the review.

### Critical

Use Deep runtime checks. Group strategic deliverables in an Epic where useful. Neither a separate Epic gate, a fixed number of evidence packs nor a 13-step review is added merely by passing `critical`.

### Shared activation and validity boundaries

For PRD/Epic/SPEC/RFC/ADR, normal activation checks MUST validation, a trimmed body of at least 100 bytes, evidence presence and detected stubs. It does not require a universal positive R_eff threshold or automatically run ADI/tests. Review SHOULD findings even when `validate --ci` exits successfully. `--force` bypasses methodology gates, not the lifecycle state machine.

Set `valid_until` based on the claim's useful lifetime; the validator does not require it for every Deep/Critical artifact. A visible date in prose is not necessarily indexed validity metadata.

## Examples

### Example 1: Fixing a Typo in a Template — Tactical

**Situation**: A typo was found in a section heading of `templates/prd/_TEMPLATE.md`.

**Why Tactical**: Obviously what to fix, 1 file, easy to revert, zero risk.

**Action**: Fix and commit. No Note needed.

### Example 2: Adding a New Export Format (JSON) — Standard

**Situation**: Users are requesting artifact export to JSON in addition to Markdown.

**Why Standard**: Multiple approaches exist (serde_json vs. tera templates, nesting format), feature takes 2-3 days, affects the `artifact/writer` module.

**Action**: Create a Standard PRD with measurable format requirements, then an RFC describing the implementation. Keep the product needs separate from storage/serialization choices.

### Example 3: Migrating from SQLite to LanceDB — Deep

**Situation**: Decision to replace the storage engine to support vector search.

**Why Deep**: Irreversible (data migration), affects the entire storage subsystem, security implications (user data), implementation timeline 1-2 weeks.

**Action**: PRD (why LanceDB) -> Spec (table schema, API contracts) -> RFC (migration plan, phases) -> ADR (why LanceDB and not Qdrant/Milvus/SQLite+extensions).

### Example 4: Launching Desktop App (Tauri) — Critical

**Situation**: Strategic decision to build a desktop application alongside the CLI.

**Why Critical**: Strategic initiative, multiple subsystems (UI, IPC, state management), multiple "teams" (frontend/backend), roadmap-level decision spanning months.

**Action**: Epic (Desktop App v1.0) -> PRD[] (UI/UX, Data Sync, Settings) -> Spec[] (Tauri Commands API, React Components) -> RFC[] (IPC architecture, state management) -> ADR[] (Tauri vs Electron, React vs Svelte, bundling strategy).

## Related Documents

- [QUALITY-GATES.md](../operations/QUALITY-GATES.md) — which checks to apply at each level
- [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) — decision tree: which document to create
- [ARTIFACT-MODEL.md](ARTIFACT-MODEL.md) — artifact hierarchy and lifecycle
