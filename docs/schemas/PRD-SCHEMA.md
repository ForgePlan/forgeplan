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

This page describes the Forgeplan **0.38.0** validator, not every recommendation in the authoring template. `critical` is accepted as an alias of `deep`; it is not a separate runtime gate.

| Depth | Additional runtime requirements |
|-------|---------------------------------|
| `note`, `tactical` | Base metadata and the five PRD sections below |
| `standard` | Target audience MUST; problem density and implementation leakage SHOULD |
| `deep` / `critical` | Standard requirements, density and leakage become MUST; deep sections below |

Depth measures decision rigor, not a fixed number of hours or days.

## Required Sections

### For all depth levels:

The common MUST checks require non-empty frontmatter `id` and `status`. Author, dates, priority and other template metadata are useful authoring fields, not additional PRD MUST checks.

| Section | Accepted headings / runtime check |
|---------|-----------------------------------|
| **Problem Statement** | `Problem`, `Problem Statement`, `Motivation`, `Background` |
| **Goals** | `Goals`, `Success Criteria`, `Objectives` |
| **Non-Goals** | `Non-Goals`, `Out of Scope`, `Product Scope` |
| **Functional Requirements** | `Functional Requirements` or `Requirements` |
| **Related Artifacts** | `Related Artifacts`, `Related`, `Dependencies` |

Section detection is case-insensitive prefix matching at any Markdown heading level. Prefer the template's English headings even when the prose is Russian; arbitrary translated headings are not aliases. Most checks establish section presence, not item counts, sentence counts or stakeholder approval.

### For Standard+:

| Check | Standard | Deep / Critical |
|-------|----------|-----------------|
| Target audience (`Target`, `Audience`, `Users` and their expanded forms) | MUST | MUST |
| Problem section has at least 50 words | SHOULD | MUST |
| No implementation technology names in FR/NFR prose | SHOULD | MUST |

### For Deep/Critical:

| Section / check | Severity |
|-----------------|----------|
| `Timeline` | MUST |
| `Stakeholders` | MUST |
| `Acceptance Criteria` | MUST (heading presence; Gherkin is template guidance) |
| `Risk…` (e.g. `Risks & Mitigations`) | MUST |
| `Success Metrics` or `Success Criteria`, containing a digit, `%`, `< ` or `> ` | MUST |
| `Rollback…` or `Revert…` | SHOULD |
| `Dependencies` or `Depends…` | SHOULD |

## Validation Rules (from BMAD)

### Quality Gates

- At every depth, missing NFRs (`Non-Functional Requirements`, `NFR`, `Quality Attributes`) and unmeasured subjective NFR wording are SHOULD findings. State measurable budgets and how they are measured, or explicitly state that the change adds none.
- Other SHOULD checks flag vague FR wording, filler, orphan FRs/goals and project-type omissions. `FR-001` with actor/capability wording is the template convention; the FR-format rule is COULD, not a MUST unique-ID check.
- Domain-specific sections are MUST when `domain` selects a supported regulated domain. Template completeness does not establish semantic correctness.
- Unfilled templates and placeholders produce SHOULD findings during validation. Activation separately blocks detected stubs; `validate` PASS is not permission to ship an empty form.

The PRD states user needs and outcomes. A SPEC supplies the contract or observable scenarios used to check an implementation; it is not a second PRD.

### Adversarial Review

Apply the core protocol in [`QUALITY-GATES.md` §2 — Adversarial Review](../operations/QUALITY-GATES.md#2-adversarial-review-protocol). When reviewing a PRD, actively search for counterexamples against, among others:
- Unmeasurable Goal?
- Missing edge case?
- Unrealistic timeline?
- Forgotten stakeholder?
- Security/privacy concern?

The reviewer never invents findings to manufacture a non-zero count and never accepts risk on the owner's behalf; unresolved HIGH / Critical means BLOCKER. A zero-defect PASS requires the seven D1 conditions (generator ≠ verifier, current version pinned, coverage, evidence sufficiency, preserved gates, finite report) on the current complete candidate.

## Numbering

| Format | Example |
|--------|---------|
| ID | `PRD-NNN` (sequential per project) |
| File | `PRD-{NNN}-{kebab-case-title}.md` |
| Path | `.forgeplan/prds/PRD-042-social-login.md` |

## Status Lifecycle

```
draft → active → superseded | deprecated | stale
stale → active (renew) | deprecated
```

These are shared artifact statuses, not implementation milestones. Use `activate`, `supersede --by`, `deprecate --reason`, `renew` and `reopen` as appropriate. `update --status active|superseded|deprecated` is rejected. Track implementation with checkboxes / phases, not `Approved`, `Implemented` or `Closed` frontmatter values.

## Progress Bars (same format as RFC)

```
Phase 0  ████████████████████████  8/8   (100%) DONE
Phase 1  ██████████████░░░░░░░░░░  7/12  ( 58%)
─────────────────────────────────────────────────
TOTAL                              15/20 (75.0%)
```

## Links to Other Artifacts

```
SPEC-001 ──refines──→ PRD-001
RFC-042  ──based_on─→ PRD-001
ADR-007  ──based_on─→ RFC-042
PRD-001.parent_epic = EPIC-003
```

Create actual graph edges with `forgeplan link`; a prose table alone does not create links. New artifact numbers are allocated per kind, with cross-worktree coordination in the same Git repository; they are not a global cross-repository sequence.
