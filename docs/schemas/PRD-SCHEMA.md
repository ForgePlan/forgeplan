[English](PRD-SCHEMA.md) · [Русский](PRD-SCHEMA.ru.md)

# PRD Schema — Product Requirements Document

## When to Create a PRD

| Situation | PRD needed? | Alternative |
|-----------|-------------|-------------|
| New user-facing feature | ✅ Yes | — |
| Significant product change | ✅ Yes | — |
| Minor bug fix | ❌ No | Go straight to RFC or PR |
| Refactoring (no UI changes) | ❌ No | ADR → RFC |
| Infrastructure | ❌ No | RFC |
| API for external clients | ✅ Yes | + SPEC |
| Internal API | ❌ No | SPEC → RFC |

**Rule**: A PRD is needed when there is a **user** with a **problem** and you need to define **what** to build.

## Depth Calibration

| Complexity | Depth | Required Sections | Example |
|-----------|-------|-------------------|---------|
| **Tactical** | 1-2 hours | Problem + Goals + Requirements (3-5) | Add a filter to a table |
| **Standard** | 1-2 days | All sections | New settings module |
| **Deep** | 3-5 days | All sections + User Research + Metrics Plan | New subsystem |
| **Critical** | 1-2 weeks | Everything + Stakeholder Sign-offs + Risk Analysis | Payment system |

## Required Sections

### For all depth levels:

| # | Section | Required? | Validation |
|---|---------|-----------|------------|
| 1 | **Meta Header** | ✅ MUST | Status, Author, Created, Updated, Priority |
| 2 | **Problem Statement** | ✅ MUST | >= 2 sentences, contains "because" / "impact" |
| 3 | **Goals** | ✅ MUST | >= 1 goal, each measurable |
| 4 | **Non-Goals** | ✅ MUST | >= 1 item (scope boundary) |
| 5 | **Functional Requirements** | ✅ MUST | >= 1 REQ with Priority (Must/Should/Could) |
| 6 | **Success Metrics** | ✅ MUST | >= 1 KPI with Current + Target |
| 7 | **Related Artifacts** | ✅ MUST | Links to SPEC/RFC/ADR if exist |

### For Standard+:

| # | Section | Required? | Validation |
|---|---------|-----------|------------|
| 8 | **Target Audience** | ✅ MUST | >= 1 persona with description |
| 9 | **User Stories** | SHOULD | "As a [role], I want [X], so that [Y]" |
| 10 | **Non-Functional Requirements** | SHOULD | Performance, Security, Scalability |
| 11 | **Dependencies** | SHOULD | External/internal deps |
| 12 | **Risks** | SHOULD | >= 1 risk with mitigation |

### For Deep/Critical:

| # | Section | Required? | Validation |
|---|---------|-----------|------------|
| 13 | **Timeline** | ✅ MUST | Milestones with dates |
| 14 | **Stakeholders** | ✅ MUST | Sign-off checkboxes |
| 15 | **Acceptance Criteria** | ✅ MUST | Given/When/Then format |
| 16 | **Competitive Analysis** | COULD | If applicable |

## Validation Rules (from BMAD)

### Quality Gates

1. **Completeness** — all MUST sections are filled (not placeholders)
2. **Measurability** — each Goal has a numerical target
3. **Traceability** — each REQ has a unique ID (REQ-N)
4. **Density** — Problem Statement >= 50 words
5. **Scope Clarity** — Non-Goals >= 1 item
6. **No Implementation Leakage** — PRD describes WHAT, not HOW
7. **Consistency** — Goals and Requirements do not contradict each other

### Adversarial Review (from BMAD)

When reviewing a PRD, the reviewer **MUST** find at least 1 issue:
- Unmeasurable Goal?
- Missing edge case?
- Unrealistic timeline?
- Forgotten stakeholder?
- Security/privacy concern?

If the reviewer found zero issues — **review more carefully**.

## Numbering

| Format | Example |
|--------|---------|
| ID | `PRD-NNN` (sequential per project) |
| File | `PRD-{NNN}-{kebab-case-title}.md` |
| Path | `docs/prds/PRD-042-social-login.md` |

## Status Lifecycle

```
Draft → Review → Approved → Implementing → Implemented → Closed
                    ↓
               Rejected (with reason)
```

## Progress Bars (same format as RFC)

```
Phase 0  ████████████████████████  8/8   (100%) DONE
Phase 1  ██████████████░░░░░░░░░░  7/12  ( 58%)
─────────────────────────────────────────────────
TOTAL                              15/20 (75.0%)
```

## Links to Other Artifacts

```
PRD-001 ──creates──→ SPEC-001 (contracts)
PRD-001 ──creates──→ RFC-042 (architecture)
PRD-001 ──creates──→ ADR-007 (decisions)
PRD-001 ──belongs──→ EPIC-003 (initiative)
```
