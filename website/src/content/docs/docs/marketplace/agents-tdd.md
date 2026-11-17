---
title: agents-tdd
description: "Test-driven development pipeline: an orchestrator coordinates planning, failing tests, independent RED validation and implementation in separate contexts."
---

**Runtime summary:** Test-driven development pipeline: an orchestrator coordinates planning, failing tests, independent RED validation and implementation in separate contexts. During an initialized active cycle, its hook restricts recognized writes to classified source/test paths; it is not a filesystem sandbox. Use when an active spec carries concrete scenarios and tests should be the implementation oracle.

**Cost**: light | **Stability**: beta | **Version**: 0.6.0 | **Category**: workflow

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 0 command files, 4 agent files, 2 skills

## Runtime boundaries

Enforcement applies only to recognized tool calls during an initialized active branch cycle and to targets classified by the configured source/test patterns. Missing state, a non-repository or detached branch, and `phase: done` allow writes. Unrecognized Bash write syntax or an unresolved target is allowed; only the first extracted Bash target is classified. RED permits source stubs carrying `STUB:TDD`; GREEN blocks classified test writes. A SPEC hash check runs only when a frozen hash is present, and an absent SPEC file is not blocked. This is not a filesystem sandbox and does not intercept arbitrary external writes.

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `agents-tdd:coder-tdd` | `agents/coder-tdd.md` | Methodology: TDD RED phase (C3 phase agent of the AD/AID-PDLC sub-cycle contract) + CRUD-R-A Profile C-coder scoped to TEST files. RFC-012 FR-2. The RED test-writer. Reads the test PLAN (from tdd-planner) plus `.forgeplan/tdd/stack.json` (test_command + file globs + the per-stack red-confirm marker) and writes FAILING tests in the stack's engine. A valid RED compiles AND executes at least one assertion AND fails ON that assertion - never a compile, import, or collection error. Writes test files only; touches source files only to emit a minimal `STUB:TDD` stub so the test can compile, never a real implementation. Denies all forgeplan mutations (new / update / link / activate) - it is a producer, not a recorder. Pinned behavioral discipline: NO deliberation, NO options menus, NO plan-before-acting - the first user-facing output is the test file path, then the tests, then the RED proof. Hands the failing tests to tdd-test-validator (C4) for independent certification before GREEN. |
| `agents-tdd:tdd-orchestrator` | `agents/tdd-orchestrator.md` | Methodology: TDD methodology master (Profile B-orchestrator) - the L2 stage-master of the TDD sub-cycle, first instance of the AD/AID-PDLC sub-cycle contract (ADR-010, contract C1-C6; RFC-012 FR-7). Peer of smith and sparc-orchestrator. Coordinates the RED→verify→GREEN flow by dispatching phase agents via Task; never writes code, tests, or artifacts itself; never activates. Master orchestrator for the enforced-TDD sub-cycle. Reads the active PRD+SPEC, then dispatches the four phases as separate isolated-context agents via the Task tool - tdd-planner (scenarios→test PLAN) → coder-tdd (plan→failing tests, RED) → tdd-test-validator (independent C4 verifier) → coder (GREEN; reuse agents-core:coder) - with a BLOCKING quality-gate between every phase. On gate FAIL, returns to the previous phase; on PASS, advances and (at the validator gate) FREEZES the oracle. Writes phase transitions to the per-branch state file via the tdd-lib.sh CLI (the PreToolUse hook only reads). Refuses to start unless PRD+SPEC are active and the SPEC carries #### Scenario blocks (precondition C1). NEVER writes source/test/artifact files; NEVER calls forgeplan_activate. Cite ADR-010 contract C1-C6 + RFC-012 FR-7. |
| `agents-tdd:tdd-planner` | `agents/tdd-planner.md` | Methodology: AD/AID-PDLC sub-cycle contract C3 (phase agent) + CRUD-R-A Profile A creator (test-PLAN artifact, language-NEUTRAL). First instance = enforced-TDD (RFC-012 FR-1 / ADR-010). Language-NEUTRAL test planner - the first C3 phase agent of the enforced-TDD sub-cycle. Reads the FROZEN `#### Scenario` blocks of an active SPEC and produces a test PLAN: per scenario → test cases + what-to-assert + edge cases + RED-first expectation + how-to-write guidance for the downstream test-author. Writes NO code and picks NO language/framework. Emits the plan as a forgeplan artifact via MCP (never via Write/Edit). Denies source+test file writes and `forgeplan_activate`. May invoke `fpf:fpf-decompose` on a complex SPEC to split scenarios into bounded test groups (C7, on-demand). Hands the plan to `coder-tdd` (RED), who turns it into failing tests in the stack's engine. |
| `agents-tdd:tdd-test-validator` | `agents/tdd-test-validator.md` | Methodology: AD/AID-PDLC C4 independent verifier (ADR-010) + CRUD-R-A Profile B-gate (binary PASS/CONCERNS/BLOCKER verdict). Implements RFC-012 FR-3. The independent verifier for the TDD sub-cycle (Profile B-gate, guardian-shaped). The DIFFERENT context that certifies the RED tests before GREEN may begin - generator≠verifier applied to TESTS, not just code (ADR-009/ADR-010 C4). Reads the failing tests plus the SPEC's frozen `#### Scenario`s and certifies five things: every scenario has ≥1 covering test, the RED is valid (fails on an assertion, not a compile/collection error), the tests are not tautological/vacuous, assertion strength is adequate, and no mock gap hides a wiring failure. Renders a binary verdict - PASS (orchestrator may freeze the oracle and dispatch GREEN) / CONCERNS (orchestrator returns to coder-tdd to strengthen named tests) / BLOCKER (orchestrator halts; tests stay unfrozen) - and emits a forgeplan EVIDENCE artifact carrying that verdict and this agent's identity (the C6 exit gate reads it). It did NOT write the tests (coder-tdd did) - that separation is the whole point. Read-only: the `disallowedTools` denylist forbids ALL writes (Write/Edit/NotebookEdit) plus forgeplan_activate/reason. May invoke FPF `fpf-evaluate` (Trust Calculus) on contested coverage (C7). Never freezes the oracle itself and never activates - the orchestrator does both on reading a PASS verdict. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `tdd` - Launch the TDD orchestrator for an active PRD/SPEC with scenarios: plan tests, produce RED, independently validate it, then implement GREEN. The initialized branch-state hook constrains recognized writes to classified targets; frozen-test protection is subject to those same tool/path limits, not filesystem-wide immutability.
- `tdd-init` - One-time per-project setup for the enforced-TDD methodology - generates the `.forgeplan/tdd/stack.json` language binding that the `tdd-gate.sh` hook reads (test command, test/source file globs, the assertion-failure marker, lint command). Without it the gate cannot classify files and the TDD cycle cannot start. Auto-detects the stack from repo markers (pyproject/go.mod/Cargo.toml/package.json/ composer.json/Gemfile/pom.xml), optionally reconciles with a stack-ADR if one exists, shows the result for human review, then writes it. Run once before the first `/tdd` cycle on a project. Detects the test stack and writes `.forgeplan/tdd/stack.json`. Re-run to update after a stack change.

## Hooks

Registered events in `hooks/hooks.json` (1): `PreToolUse`.

Registration does not guarantee activation in every client. Client support, setup and defaults-off settings still apply.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-tdd@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
