[English](EPIC-SCHEMA.md) · [Русский](EPIC-SCHEMA.ru.md)

# EPIC Schema — Strategic Initiative

## When to Create an Epic

- Initiative spanning > 2 RFCs
- Cross-team work
- Quarterly roadmap item
- Monolith decomposition (multiple services)
- Migration (framework, DB, cloud)

**Rule**: Epic = container. If you have 1 PRD and 1 RFC — an Epic is not needed.

## Required Sections

Forgeplan **0.38.0** checks the following at every depth. The shared metadata MUST checks require non-empty frontmatter `id` and `status`; owner, dates and target quarter are template guidance.

| Section | Runtime MUST: accepted headings |
|---------|---------------------------------|
| Vision | `Vision`, `Goals`, `Executive Summary` |
| Outcomes | `Outcomes`, `Success Criteria`, `Goals` |
| Children | `Children`, `Artifacts`, `PRDs`, `Components` |
| Phases | `Phases`, `Phase`, `Implementation Phases`, `Timeline` |
| Progress | `Progress`, `Status` |

Matching is case-insensitive by prefix at any heading level. These are presence checks: they do not count measurable outcomes, verify child membership, require a phase item or recompute progress bars. Dependency graph and risk sections are useful template guidance, not registered Epic SHOULD rules. Shared placeholder/stub and body/link-drift checks still apply.

## Aggregated Progress

The template illustrates a manually maintained sum of child checkbox counts. `forgeplan progress EPIC-003` reads checkboxes in the Epic body; it does not recursively aggregate child PRD/RFC progress. A textual Children table also does not set `parent_epic` or create graph edges.

```
PRD-001  ████████████████████████  8/8   (100%) DONE
RFC-042  ██████████████░░░░░░░░░░  7/12  ( 58%)
RFC-043  ░░░░░░░░░░░░░░░░░░░░░░░░  0/6   (  0%)
─────────────────────────────────────────────────
TOTAL                              15/26 (57.7%)
```

## Status Lifecycle

```
draft → active → superseded | deprecated | stale
stale → active (renew) | deprecated
```

`Done`, `Archived` and `Cancelled` may describe project milestones in prose; they are not shared artifact lifecycle states. Use the lifecycle commands rather than bypassing them through `update --status`.

## Numbering

| Format | Example |
|--------|---------|
| ID | `EPIC-NNN` |
| File | `EPIC-{NNN}-{kebab-case}.md` |
| Path | `.forgeplan/epics/EPIC-003-auth-system.md` |
