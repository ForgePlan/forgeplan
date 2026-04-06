---
depth: standard
id: NOTE-002
kind: note
links:
- target: EPIC-001
  relation: informs
status: deprecated
title: Integration vision
---

# NOTE-002: Task Tracker Sync Vision

Forgeplan = ЧТО и ПОЧЕМУ (methodology), tracker = КТО и КОГДА (execution). Sync = bridge.

## Mapping

| Forgeplan | Tracker |
|-----------|---------|
| PRD | Epic |
| FR | Task |
| RFC | Story |
| ADR | Decision tag |

## Architecture

- Artifact IDs mappable (PRD-001 ↔ Linear issue ID)
- Sync = separate crate (forgeplan-sync)
- Orchestra already connected through MCP


