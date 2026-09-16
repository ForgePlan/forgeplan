---
title: agents-tdd
description: "Enforced test-driven development: an orchestrator drives plan -> failing tests -> independent validation -> implementation, with a fail-closed hook that blocks source writes until the tests exist, and a separate context that certifies the RED is real before GREEN may start."
---

Enforced test-driven development: an orchestrator drives plan -> failing tests -> independent validation -> implementation, with a fail-closed hook that blocks source writes until the tests exist, and a separate context that certifies the RED is real before GREEN may start. Use when a spec carries concrete scenarios and the tests should be the oracle rather than an afterthought - the validator exists because a test written by the same context that writes the code is not a check. Full history in CHANGELOG.md.

**Cost**: light | **Stability**: beta | **Version**: 0.6.0 | **Category**: workflow

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Agents

| Agent | What it does |
|---|---|
| `coder-tdd` | The RED test-writer. |
| `tdd-orchestrator` | Master orchestrator for the enforced-TDD sub-cycle. |
| `tdd-planner` | Language-NEUTRAL test planner - the first C3 phase agent of the enforced-TDD sub-cycle. |
| `tdd-test-validator` | The independent verifier for the TDD sub-cycle (Profile B-gate, guardian-shaped). |

## Skills

- `tdd` - Enforced-TDD entry point.
- `tdd-init` - One-time per-project setup for the enforced-TDD methodology - generates the `.forgeplan/tdd/stack.json` language binding that the `tdd-gate.sh` hook reads (test command, test/source file globs, the assertion-failure marker, lint command).

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-tdd@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
