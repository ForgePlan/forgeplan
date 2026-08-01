# Issue Governance

## Labels recommended

```text
area:protocol
area:core
area:agent-api
area:policy
area:docs
area:marketplace
area:web
area:integration
kind:epic
kind:feature
priority:p0
priority:p1
status:blocked
needs:decision
needs:verification
```

## Required issue sections

- Objective
- Product boundary
- Scope
- Non-goals
- Dependencies
- Acceptance criteria
- Evidence
- Rollback/migration where applicable

## Completion rule

Issue closes only when:

- linked PR merged;
- acceptance criteria checked;
- independent verification recorded;
- Evidence artifact or equivalent receipt linked;
- docs and schemas updated;
- related old issues closed/superseded;
- release note added for user-visible change.

## Existing issues

Do not recreate existing defects. New umbrella implementation issues must explicitly coordinate and close/supersede referenced issues such as #304, #325, #328, #329, #353, #360, #374 and #397.
