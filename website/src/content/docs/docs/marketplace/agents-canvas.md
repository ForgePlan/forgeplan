---
title: agents-canvas
description: "CANVAS design-system-to-code methodology: a coordinator walks a Pencil design through capture, audit, norm-check, vectorize and assemble, dispatching every phase and every verifier as a separate isolated context with a blocking generator-is-not-verifier gate at each handoff, plus a fail-closed hook that blocks source writes until the token contract is active."
---

CANVAS design-system-to-code methodology: a coordinator walks a Pencil design through capture, audit, norm-check, vectorize and assemble, dispatching every phase and every verifier as a separate isolated context with a blocking generator-is-not-verifier gate at each handoff, plus a fail-closed hook that blocks source writes until the token contract is active. Use when a design system has to become working components in the project's own framework, with visual parity and token fidelity proven rather than asserted. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: beta | **Version**: 1.6.0 | **Category**: methodology

**Targets**: claude-code

**Requires**: MCP: pencil; Plugins: laws-of-ux

## Commands

| Command | What it does |
|---|---|
| `/canvas-audit` | One-shot DS-convention audit (canvas-guardian as a command). |
| `/canvas-init` | One-time per-branch setup for the CANVAS tokens-gate. |
| `/canvas-review` | CANVAS post-export code + UX gate. |
| `/canvas-rule` | Look up a CANVAS design-system convention or a UX law by name - refs/slots, token naming, atomic layering, the token contract, the six Pencil HARD RULES, or any of the 30 Laws of UX. |

## Agents

| Agent | What it does |
|---|---|
| `canvas-coder` | Builds the Storybook component implementations + stories + visual tests from the canvas-port manifest. |
| `canvas-coordinator` | Master orchestrator for the Pencil -> design-system -> Storybook -> framework pipeline. |
| `canvas-designer` | The Capture phase of CANVAS. |
| `canvas-guardian` | The DS-build conscience for the CANVAS pipeline. |
| `canvas-porter-framework` | OPTIONAL - runs only on an explicit multi-framework request; the default native single-framework build never invokes it. |
| `canvas-porter-storybook` | Carefully transfers the approved Pencil design system into the tokens RFC + a Storybook port manifest - the token contract + per-component story specs + reference screenshots the canvas-coder builds from. |
| `canvas-storybook-validator` | The Storybook-gate conscience for the CANVAS pipeline (RFC-021 FR-4, ADR-010 C4). |
| `canvas-tester` | The CANVAS Norm-check gate. |

## Skills

- `canvas` - Entry point for the CANVAS design-system -> code pipeline - the master-coordinated walk from a Pencil design to **native code in the consuming project's own framework** (framework resolved via Step 0 - Capture -> Audit -> Norm-check -> Vectorize -> Assemble), an instance of the AD/AID-PDLC sub-cycle contract (ADR-010 / RFC-021), hook-gate=YES.
- `canvas-conventions` - CANVAS DS-build conventions knowledge base - the rule set the canvas-guardian audits a Pencil Design System against (HOW it was built, not whether it matches requirements).
- `canvas-design` - Pencil designer knowledge base for the CANVAS methodology - how to design and extend a .pen Design System with refs, slots, descendants, Layout B, atomic organization, themed tokens, style presets, and UX-law-driven node constraints.
- `canvas-port` - CANVAS porting playbook - Pencil design system -> Storybook -> native framework code.
- `canvas-storybook-test` - CANVAS Storybook-gate validation toolkit (RFC-021 FR-4) - the knowledge base read by canvas-storybook-validator (the C4 Storybook-gate, a SUB Task agent) when it certifies the BUILT Storybook against the Pencil source ONLY.
- `canvas-truth-map` - Map a Pencil design system to the ForgePlan source of truth (PRD/ADR/EVID) - the CANVAS Norm-check (N) knowledge base.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-canvas@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
