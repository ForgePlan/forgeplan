---
title: forgeplan reason
description: "Analyze an artifact using the FPF ADI reasoning cycle (Abduction→Deduction→Induction)"
---

`forgeplan reason` runs a structured AI-driven analysis over an existing artifact using the FPF **ADI cycle**: **Abduction** (generate 3+ hypotheses) → **Deduction** (predict consequences of each) → **Induction** (check predictions against existing evidence). It's the gate between "PRD looks reasonable" and "I actually know which approach to take" - forcing the agent to enumerate alternatives instead of anchoring on the first plausible answer. For Deep and Critical depth it is **mandatory**: no code until `reason` has produced at least three competing hypotheses and a justified winner.

This is the project's local ADI workflow, not proof of upstream FPF conformance. The LLM prompt requests 3+ hypotheses; the parser does not enforce that count. Review the result against the mandatory Deep/Critical requirement rather than treating a successful command as its fulfillment. A configured LLM provider is required: HTTP uses `llm.api_key_env`; opted-in local CLI providers reuse their own login only for personal local development.

## When to use

- Right after `forgeplan new prd` + `forgeplan validate` PASS, before touching code.
- Depth is **Deep** or **Critical** - ADI is non-negotiable.
- Depth is **Standard** and the solution space has real trade-offs (caching layer, rate limiter algorithm, auth flow).

## When NOT to use

- Tactical work without a decision that needs alternatives; elapsed time alone does not establish depth.
- MUST sections are not yet filled - ADI on a stub produces hallucinated context.
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

Reads the artifact body and asks the configured LLM for hypotheses, assumptions, predictions, risks, feasibility, evidence needs and a recommendation. It does not limit its input to named Problem/Goals/Related sections.

### Example 2: ADI with FPF knowledge base context

```bash
forgeplan reason PRD-001 --fpf
```

`--fpf` requests knowledge-base context when that source is available. In the selected migrated CLI path the FPF enrichment is omitted; legacy lookup can enrich the prompt. Neither path guarantees particular upstream sections or mechanically scores FPF invariants. Check the supplied context and verify claims independently.

### Example 3: Persist the analysis as a Note

```bash
forgeplan reason PRD-001 --save
```

Creates a Note containing the ADI output and links it to the source. Read it by its returned ID or use `forgeplan graph` for relationships; `forgeplan get` has no `--graph` option.

### Example 4: Machine-readable output for agents

```bash
forgeplan reason PRD-001 --json
```

The result contains artifact metadata and `adi_output` when structured parsing succeeds, otherwise raw `adi_analysis`. Structured hypotheses have `id`, `description`, `assumptions`, and string `confidence`; deductions contain `hypothesis_id`, `consequence`, `risks`, and `feasibility`. `evidence_needed` and `recommendation` are separate fields, not per-hypothesis evidence/verdict arrays.

## Output interpretation

Review the requested ADI stages:

- **Abduction**: require at least three genuinely competing hypotheses for Deep/Critical; a successful parse does not enforce this.
- **Deduction**: inspect predicted consequences, risks and feasibility.
- **Induction**: check real evidence needs and the recommendation. The command does not run experiments or validate the recommendation.

Confidence strings are model judgments, not calibrated percentages or empirical trust. Agreement among hypotheses is not independent evidence. Missing alternatives or evidence require further shaping or targeted evidence before the relevant gate can pass. EvidencePack `supports` / `weakens` / `refutes` labels are distinct from generated prose judgments.

## How it fits the workflow

This command belongs in the [full artifact lifecycle](/docs/guides/first-artifact/) - see the tutorial for the end-to-end flow. `reason` runs after `validate` PASS and before code; for Critical depth, pair with `/audit` and a human review before implementation.

## See also

- [`forgeplan route`](/docs/cli/route/) - decide whether ADI is required
- [`forgeplan decompose`](/docs/cli/decompose/) - break reasoned PRD into RFC tasks
- [`forgeplan validate`](/docs/cli/validate/) - prerequisite before reasoning
- [`forgeplan generate`](/docs/cli/generate/) - draft artifact content
- [Methodology: ADI cycle](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan reason --help`.

```text
Analyze an artifact using FPF ADI reasoning cycle (Abduction→Deduction→Induction)

REQUIRES LLM: generates structured three-hypothesis ADI output through the provider configured in `.forgeplan/config.yaml::llm`.

AUTHENTICATION: HTTP providers use the API-key environment variable named by `llm.api_key_env`. Local subscription providers reuse an existing stock CLI login and need no API key in ForgePlan:

llm: provider: codex-cli model: gpt-5.1-codex

Supported local profiles are `claude-code`, `codex-cli`, `devin-cli`, `gemini-cli`, and `cursor-agent`. They are explicit opt-ins for personal local development, not production/shared/CI use. Each invocation consumes the corresponding vendor plan and is subject to that vendor's terms.

Usage: forgeplan reason [OPTIONS] <ID>

Arguments:
  <ID>
          Artifact ID to analyze

Options:
      --json
          Output structured JSON instead of markdown

      --save
          Save ADI analysis as a Note artifact linked to the source

      --fpf
          Inject relevant FPF patterns into the ADI prompt

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
