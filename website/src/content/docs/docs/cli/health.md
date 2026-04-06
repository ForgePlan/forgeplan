---
title: forgeplan health
description: Project health dashboard — gaps, risks, blind spots, next actions
---

## Usage

```bash
forgeplan health
```

## What It Shows

```
Forgeplan Health — MyProject
══════════════════════════════════════════

  Artifacts:  20 total
  By kind:    prd 5, rfc 3, adr 2, evidence 8, note 2
  By status:  active 12, draft 6, deprecated 2

  ○ Blind spots (1):
    ADR-001 — no linked evidence

  ○ Orphans (2):
    PRD-024 — no links
    PRD-025 — no links

  → Next actions:
    1. Link 2 orphan artifact(s)
    2. Add evidence for 1 blind spot(s)
```

## What It Detects

| Check | Description |
|-------|-------------|
| **Blind spots** | Active decisions without any linked evidence |
| **Orphans** | Artifacts with no links to any other artifact |
| **Stale** | Artifacts past `valid_until` date |
| **Gaps** | Pipeline compliance gaps by depth |

## When to Use

- **Session start** — first command to run in any new chat
- **Before PR** — ensure no blind spots or orphans
- **Sprint review** — overall project health check

## Related Commands

```bash
forgeplan blindspots    # Just blind spots
forgeplan stale         # Just stale artifacts
forgeplan gaps          # Pipeline compliance
forgeplan journal       # Decision timeline
```
