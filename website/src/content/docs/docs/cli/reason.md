---
title: forgeplan reason
description: "Analyze an artifact using the FPF ADI reasoning cycle (Abduction→Deduction→Induction)"
---

`forgeplan reason` runs a structured AI-driven analysis over an existing artifact using the FPF **ADI cycle**: **Abduction** (generate 3+ hypotheses) → **Deduction** (predict consequences of each) → **Induction** (check predictions against existing evidence). It's the gate between "PRD looks reasonable" and "I actually know which approach to take" — forcing the agent to enumerate alternatives instead of anchoring on the first plausible answer. For Deep and Critical depth it is **mandatory**: no code until `reason` has produced at least three competing hypotheses and a justified winner.

## When to use

- Right after `forgeplan new prd` + `forgeplan validate` PASS, before touching code.
- Depth is **Deep** or **Critical** — ADI is non-negotiable.
- Depth is **Standard** and the solution space has real trade-offs (caching layer, rate limiter algorithm, auth flow).

## When NOT to use

- Depth is **Tactical** — a one-hour bug fix does not need three hypotheses.
- MUST sections are not yet filled — ADI on a stub produces hallucinated context.
- Artifact is a pure EvidencePack, Note, or RefreshReport (no decision to reason about).

## Usage

```text
forgeplan reason [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID to analyze (PRD, RFC, ADR, Epic, Problem, ...)
```

## Options

```text
      --json     Output structured JSON instead of markdown
      --save     Save ADI analysis as a Note artifact linked to the source
      --fpf      Inject relevant FPF patterns into the ADI prompt
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: Standard ADI on a PRD

```bash
forgeplan reason PRD-001
```

Reads `PRD-001`, pulls its Problem/Goals/Related sections, and asks the LLM to
generate 3+ hypotheses for how to meet the goals. For each hypothesis it lists
predicted consequences and which existing evidence supports or weakens it.

### Example 2: ADI with FPF knowledge base context

```bash
forgeplan reason PRD-001 --fpf
```

`--fpf` injects relevant sections from the FPF knowledge base (B.3 Trust
Calculus, B.5 Reasoning loops) into the prompt. Use this when the decision
involves trust boundaries, reversibility, or reasoning quality — the LLM will
score hypotheses against FPF invariants instead of just engineering intuition.

### Example 3: Persist the analysis as a Note

```bash
forgeplan reason PRD-001 --save
```

Creates a `note-*` artifact containing the full ADI output and links it to the
source PRD (`informs` relation). Useful when you want the reasoning to survive
past the current terminal session and show up in `forgeplan get PRD-001 --graph`.

### Example 4: Machine-readable output for agents

```bash
forgeplan reason PRD-001 --json
```

Emits structured JSON with `hypotheses[]`, each containing `summary`,
`confidence`, `supporting_evidence[]`, `weakening_evidence[]`, and `verdict`.
Consumed by MCP clients and audit scripts.

## Output interpretation

A typical markdown run prints three sections:

- **Abduction** — 3+ hypotheses, each with a one-line claim and a confidence
  score (0-100%). If all three converge on the same approach, you can proceed
  with high trust. If they diverge, treat this as a signal to discuss with a
  human before coding.
- **Deduction** — predicted consequences per hypothesis (performance, rollback
  cost, blast radius, user impact).
- **Induction** — verdict per hypothesis: `supported`, `weakened`, or
  `insufficient evidence`. The final recommendation lists the winning
  hypothesis and the evidence gaps that should become follow-up EvidencePacks.

Red flags:

- All hypotheses have confidence < 50% — the PRD is underspecified, go back to Shape
- The winner is `insufficient evidence` — create targeted evidence before implementing
- Only one hypothesis was generated — LLM anchoring, re-run with `--fpf` or a different model

## How it fits the workflow

This command belongs in the [full artifact lifecycle](/docs/guides/first-artifact/) — see the tutorial for the end-to-end flow. `reason` runs after `validate` PASS and before code; for Critical depth, pair with `/audit` and a human review before implementation.

## See also

- [`forgeplan route`](/docs/cli/route/) — decide whether ADI is required
- [`forgeplan decompose`](/docs/cli/decompose/) — break reasoned PRD into RFC tasks
- [`forgeplan validate`](/docs/cli/validate/) — prerequisite before reasoning
- [`forgeplan generate`](/docs/cli/generate/) — draft artifact content
- [Methodology: ADI cycle](/docs/methodology/overview/)
