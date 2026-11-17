---
title: agents-sparc
description: "SPARC for a single feature in an existing active system: Specification, optional Pseudocode, Architecture, Refinement and Completion."
---

**Runtime summary:** SPARC for a single feature in an existing active system: Specification, optional Pseudocode, Architecture, Refinement and Completion. Three mandatory independent gates certify Specification, Architecture and Refinement. Pseudocode, when used, is certified at Architecture rather than at a separate blocking gate; MICRO scope skips it. Source dispatch waits for the Architecture RFC to be active, but SPARC supplies no write-time hook.

**Cost**: light | **Stability**: stable | **Version**: 1.8.0 | **Category**: workflow

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 0 command files, 5 agent files, 1 skills

## Runtime boundaries

The three mandatory independent reviews are Specification, Architecture and Refinement. Optional Pseudocode receives its absorption check at Architecture, with no separate Pseudocode gate; MICRO scope skips it. SPARC has no hook or state-file write barrier. Phase ordering delays source/test dispatch until the Architecture RFC is active; human or out-of-band edits are reviewed later rather than blocked at write time.

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `agents-sparc:architecture` | `agents/architecture.md` | Methodology: SPARC Architecture phase + CRUD-R-A Profile A (RFC creation w/ module breakdown, FPF ADI for option selection). SPARC Architecture phase specialist. Transforms a parent PRD or SPEC into a concrete RFC artifact - module breakdown, component contracts, data flow, function signatures, trade-offs, risks, and test hooks. Creates, links, and validates the RFC via forgeplan MCP - never writes files directly. Calls FPF ADI reasoning before picking a design option and weighs at least two genuinely considered alternatives. Tags every claim with its identity for the audit trail. |
| `agents-sparc:pseudocode` | `agents/pseudocode.md` | SPARC Pseudocode phase specialist. Takes a parent SPEC/PRD or RFC draft and produces algorithm sketches, data structure choices, and complexity analysis (Big-O for time + space) BEFORE any code is written. Bridges Specification → Architecture by validating that the algorithmic core fits the constraints. Not forgeplan-aware - operates on local pseudo-code files. Hand off to `architecture` for concrete RFC, then to `coder` (C-coder profile) for implementation. |
| `agents-sparc:refinement` | `agents/refinement.md` | SPARC Refinement phase specialist. Executes red-green-refactor TDD loop on a code surface produced by `coder`: writes failing test (red), implements minimal change to pass (green), refactors for clarity/performance (refactor). Also handles post-implementation polish - error handling, edge cases, performance tuning. Operates on source files only; not forgeplan-aware. Hand off to `tester` (Profile B) for coverage delta EVIDENCE recording. |
| `agents-sparc:sparc-orchestrator` | `agents/sparc-orchestrator.md` | Coordinates one feature in an active codebase using separate phase and verifier contexts. Mandatory independent gates are at Specification, Architecture and Refinement. Optional Pseudocode is absorbed and certified at Architecture, not separately gated; MICRO scope omits it. Coordinates implementation and Completion evidence, but does not write source or activate artifacts. SPARC has no write-time hook: phase ordering binds dispatches, not out-of-band edits. |
| `agents-sparc:specification` | `agents/specification.md` | Methodology: SPARC Specification phase + CRUD-R-A Profile A (PRD/SPEC creation with SMART AC). SPARC Specification phase specialist. Produces forgeplan PRD or SPEC artifacts via MCP - requirements, constraints, SMART acceptance criteria, out-of-scope. Never writes files directly. Calls forgeplan_reason before finalising acceptance criteria. Tags every claim with its identity for audit trail. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `sparc` - Run one feature through Specification, optional Pseudocode, Architecture, Refinement and Completion. Independent gates are mandatory at Specification, Architecture and Refinement; Pseudocode is certified at Architecture and skipped for MICRO scope. No SPARC write-time hook is installed; Refinement may delegate to TDD. Not for a new product or legacy modernization.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-sparc@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
