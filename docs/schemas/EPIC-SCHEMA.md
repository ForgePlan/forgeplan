[English](EPIC-SCHEMA.md) · [Русский](EPIC-SCHEMA.ru.md)

# EPIC Schema — Strategic Initiative

## When to Create an Epic

- Initiative spanning > 2 RFCs
- Cross-team work
- Quarterly roadmap item
- Monolith decomposition (multiple services)
- Migration (framework, DB, cloud)

**Rule**: Epic = container. If you have 1 PRD and 1 RFC — an Epic is not needed.

## Authoring Checklist

These are content conventions, not promises of automatic quantitative or
semantic enforcement. The validator checks the presence of Vision, Outcomes,
Children/Artifacts, Phases, and Progress; it does not prove a one-sentence
vision, two measurable outcomes, or that progress bars aggregate correctly.
Base metadata rules check `id` and `status` presence, not valid lifecycle
values. Other checklist details need review; validation and activation
readiness are separate.

| # | Section | Authoring expectation | Content guidance |
|---|---------|-----------|------------|
| 1 | **Meta Header** | ✅ MUST | Status, Owner, Created, Target (quarter) |
| 2 | **Vision** | ✅ MUST | 1 sentence — what we want to achieve |
| 3 | **Outcomes** | ✅ MUST | >= 2 measurable outcomes |
| 4 | **Children Table** | ✅ MUST | PRD/RFC/ADR list with status |
| 5 | **Phases** | ✅ MUST | >= 1 phase with artifacts |
| 6 | **Progress** | ✅ MUST | Aggregated progress bars |
| 7 | **Dependency Graph** | SHOULD | Mermaid diagram |
| 8 | **Risks** | SHOULD | >= 1 risk |

## Aggregated Progress

Epic progress = sum of progress from children (PRD, RFC):

```
PRD-001  ████████████████████████  8/8   (100%) DONE
RFC-042  ██████████████░░░░░░░░░░  7/12  ( 58%)
RFC-043  ░░░░░░░░░░░░░░░░░░░░░░░░  0/6   (  0%)
─────────────────────────────────────────────────
TOTAL                              15/26 (57.7%)
```

## Status Lifecycle

```text
draft --activate--> active
active --> superseded | deprecated | stale
stale --renew--> active
stale --reopen--> deprecated + new draft
stale --deprecate--> deprecated
```

`superseded` and `deprecated` are terminal. Done, Archived and Cancelled may be
process labels in prose, not values of persisted `status`; `review` is a check,
not a status transition.

## Numbering

| Format | Example |
|--------|---------|
| ID | `EPIC-NNN` |
| File | `EPIC-{NNN}-{kebab-case}.md` |
| Path | `.forgeplan/epics/EPIC-003-auth-system.md` |
