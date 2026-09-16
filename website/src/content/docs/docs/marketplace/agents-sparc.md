---
title: agents-sparc
description: "SPARC methodology agents - specification, pseudocode, architecture, refinement, and the orchestrator that walks a single feature through all five phases with a blocking quality gate between each."
---

SPARC methodology agents - specification, pseudocode, architecture, refinement, and the orchestrator that walks a single feature through all five phases with a blocking quality gate between each. Use when adding one feature to an existing, active system and the design should be settled before code: the middle path between greenfield BMAD and a tactical fix. Full history in CHANGELOG.md.

**Cost**: light | **Stability**: stable | **Version**: 1.8.0 | **Category**: workflow

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Agents

| Agent | What it does |
|---|---|
| `architecture` | SPARC Architecture phase specialist. |
| `pseudocode` | SPARC Pseudocode phase specialist. |
| `refinement` | SPARC Refinement phase specialist. |
| `sparc-orchestrator` | Master orchestrator for the SPARC feature pipeline. |
| `specification` | SPARC Specification phase specialist. |

## Skills

- `sparc` - Entry point for the SPARC feature pipeline - the master-coordinated five-phase walk that drives a SINGLE feature in an EXISTING active system from specification to a completed, independently-verified feature (Specification → Pseudocode → Architecture → Refinement → Completion), the THIRD instance of the AD/AID-PDLC sub-cycle contract (ADR-010 / RFC-016).

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-sparc@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
