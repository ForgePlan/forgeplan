---
title: agents-bmad
description: "BMAD greenfield methodology: an orchestrator coordinates Analyst, PM, Architect, Scrum-Master, Dev and QA in separate contexts with independent reviews."
---

**Runtime summary:** BMAD greenfield methodology: an orchestrator coordinates Analyst, PM, Architect, Scrum-Master, Dev and QA in separate contexts with independent reviews. The no-code-before-plan hook applies to recognized source/test writes during an initialized active cycle, with explicit escape paths. Use for a new product or service, not a single feature in an active system or legacy modernization.

**Cost**: light | **Stability**: beta | **Version**: 0.6.0 | **Category**: workflow

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 0 command files, 1 agent files, 2 skills

## Runtime boundaries

The hook covers recognized `Write`, `Edit`, `MultiEdit` and Bash targets classified by configured source/test patterns during an initialized active branch cycle. Missing state, a non-repository or detached branch, `phase: done`, an explicit state/environment override, and `.bmad-scratch/` paths allow writes. Unrecognized Bash writes or unresolved targets are allowed; only the first extracted target is classified. Other file kinds are allowed. Readiness enforcement is not a filesystem sandbox or a block on arbitrary human/external writes.

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `agents-bmad:bmad-orchestrator` | `agents/bmad-orchestrator.md` | Methodology: BMAD methodology master (Profile B-orchestrator) - the L2 stage-master of the BMAD greenfield sub-cycle, the SECOND instance of the AD/AID-PDLC sub-cycle contract (ADR-010, contract C1-C6; RFC-013). Peer of smith, sparc-orchestrator, and tdd-orchestrator. Unlike TDD (a single Build-stage sub-cycle) BMAD spans the WHOLE greenfield arc as a persona walk - Analyst → PM → Architect → Scrum-Master → Dev → QA - coordinating via Task; never writes code, tests, or artifacts itself; never activates. Master orchestrator for the BMAD greenfield pipeline. Reads the entry brief, then walks the personas as separate isolated-context dispatches via Task - brief-intake (Analyst→Brief) → specification (PM→PRD) → adr-architect+architecture (Architect→ADR+RFC) → goal-planner (Scrum-Master→story RFCs) → coder (Dev) → tester+code-reviewer (QA) → evidence-recorder (retro) - with a BLOCKING quality-gate between every persona and mandatory independent C4 validations (architect-reviewer on PRD/RFC, guardian readiness gate). Writes phase transitions to the per-branch state file via the bmad-lib.sh CLI (the PreToolUse bmad-gate hook only reads). Refuses to start without a greenfield signal (precondition C1) and refuses to advance a stage whose input artifact is not active. NEVER writes source/test/artifact files; NEVER calls forgeplan_activate. Cite ADR-010 contract C1-C6 + RFC-013. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `bmad` - Launch the BMAD greenfield persona pipeline with separate producer/reviewer contexts. Its initialized branch-state hook restricts recognized source/test writes before readiness, with done-state, override, scratch-path and Bash target-extraction exemptions described above.
- `bmad-init` - One-time per-project setup for the BMAD pipeline - generates the `.forgeplan/bmad/stack.json` language binding that the `bmad-gate.sh` hook reads (test command, test/source file globs, the assertion-failure marker, lint command) and initialises the per-branch state file. Without stack.json the gate cannot classify files, so the no-code-before-plan rule cannot be enforced. Auto-detects the stack from repo markers (pyproject/go.mod/Cargo.toml/package.json/composer.json/ Gemfile/pom.xml), optionally reconciles with a stack-ADR, shows the result for human review, writes it. Run once before the first `/bmad` cycle on a project. Detects the test/source stack and writes `.forgeplan/bmad/stack.json`; initialises `.forgeplan/bmad/state-<branch>.json` at phase=planning.

## Hooks

Registered events in `hooks/hooks.json` (1): `PreToolUse`.

Registration does not guarantee activation in every client. Client support, setup and defaults-off settings still apply.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-bmad@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
