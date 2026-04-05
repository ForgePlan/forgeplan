---
title: Artifact Lifecycle
description: States, transitions, and terminal statuses
---

## State Machine

```
draft → active → superseded (terminal)
               → deprecated (terminal)
               → stale → active (renew)
                       → deprecated + NEW draft (reopen)
```

## States

| State | Meaning | Can transition to |
|-------|---------|-------------------|
| **draft** | Work in progress, not validated | active |
| **active** | Validated, in use | superseded, deprecated, stale |
| **stale** | expired `valid_until` | active (renew), deprecated (reopen) |
| **superseded** | Replaced by newer artifact | *(terminal)* |
| **deprecated** | No longer relevant | *(terminal)* |

## Lifecycle Commands

```bash
# Validate before activating
forgeplan review PRD-001
# → Review PASSED — ready to activate

# Activate (draft → active)
forgeplan activate PRD-001
# → Validation gate checks MUST rules

# Supersede (active → superseded)
forgeplan supersede PRD-001 --by PRD-002
# → Creates link: PRD-002 supersedes PRD-001

# Deprecate (active/stale → deprecated)
forgeplan deprecate PRD-001 --reason "No longer needed"

# Renew (stale → active)
forgeplan renew PRD-001 --reason "Re-validated" --until 2026-12-31

# Reopen (stale → deprecated + NEW draft)
forgeplan reopen PRD-001 --reason "Needs major revision"
# → PRD-001 deprecated, PRD-002 created as draft
```

## Terminal States

**Superseded** and **deprecated** are terminal — no transitions out.

- Superseded: the replacement artifact should be used instead
- Deprecated: use `forgeplan reopen` to create a new draft if needed

## Validation Gates

PRD, RFC, ADR, Epic, Spec require validation before activation:

```bash
forgeplan validate PRD-001
# Checks 30+ rules per artifact type:
# - MUST sections present (Problem, Goals, FR...)
# - No implementation leakage in requirements
# - Information density (no filler)
# - Measurability (SMART criteria)
```

Notes and Problems can be activated without validation gate.
