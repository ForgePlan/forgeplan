---
depth: tactical
id: EVID-053
kind: evidence
links:
- target: PRD-026
  relation: informs
status: draft
title: Release v0.15.1 shipped — tag pushed, GH release created, dev synced
---

# EVID-053: Release v0.15.1 shipped

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Congruence Level Justification

CL3 — same context (this repository), direct measurement of release state.

## Release artifacts

| Item | Value |
|---|---|
| Version | v0.15.1 |
| Type | Patch (structural cleanup, no features) |
| Base | main |
| Release branch | release/v0.15.1 |
| PR | https://github.com/ForgePlan/forgeplan/pull/115 |
| Tag | https://github.com/ForgePlan/forgeplan/releases/tag/v0.15.1 |
| Main commit | 6e721db |
| dev synced | fast-forward 40a1579..6e721db |

## CI Results (PR #115)

| Check | Result | Duration |
|---|---|---|
| Check, Lint & Format | pass | 2m56s |
| Tests | pass | 6m43s |
| plan | pass | 18s |

## Contents

- Docs reorganization (docs/methodology/, operations/, schemas/, README)
- 138 markdown artifacts tracked in .forgeplan/
- .local/ for research/planning/sessions (gitignored)
- AGENTS.md as standard AI agent entry point
- Legacy docs/{epics,prds,rfcs,adrs,specs}/ removed (15 files)
- Gitignore selective: .forgeplan/lance/, .fastembed_cache/, config.yaml
- CLAUDE.md Storage section rewritten for ADR-003
- Fixed broken README.md links

## Result

Release v0.15.1 successfully shipped. All structural cleanup from PRD-026 is now in main. Previous release v0.15.0 (website launch) + v0.15.1 (docs reorg) together close the website+docs sprint.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-026 | informs |
| EVID-052 | related |
| NOTE-035 | informs |

