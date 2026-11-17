---
title: forgeplan reason
description: "Analyze an artifact using the FPF ADI reasoning cycle (Abduction→Deduction→Induction)"
---

`forgeplan reason` asks the configured LLM to analyze an existing artifact through the FPF **ADI cycle**: **Abduction** (propose alternatives) → **Deduction** (evaluate consequences) → **Induction** (recommend an approach). The default prompt asks for at least three distinct hypotheses and justified confidence. Deep/Critical methodology calls for this review before coding, but the CLI does not enforce a hypothesis count, validate the artifact first, or block subsequent commands. A generated recommendation is not empirical verification.

## Forgeplan 0.38.0

Configure `llm` in `.forgeplan/config.yaml`. HTTP providers read the key from `llm.api_key_env`. Local profiles `claude-code`, `codex-cli`, `devin-cli`, `gemini-cli`, and `cursor-agent` reuse an existing stock CLI login without an API key in Forgeplan. They are explicit opt-ins for personal local development, not production/shared/CI use. Each call consumes the corresponding vendor plan and is subject to its terms. LLM analysis does not replace measurements or code verification. The CLI prints progress even with `--json`; `--save` adds save messages. Do not treat all stdout as one clean JSON document.

Ollama is the keyless local-HTTP exception; embedding-model weights are not required for ADI itself or its keyword KB lookup. Provider/model selection comes from `llm` configuration and supported environment overrides, not a `reason --model` flag.

## When to use

- Right after `forgeplan new prd` + `forgeplan validate` PASS, before touching code.
- Depth is **Deep** or **Critical** - ADI is non-negotiable.
- Depth is **Standard** and the solution space has real trade-offs (caching layer, rate limiter algorithm, auth flow).

## When NOT to use

- Depth is **Tactical** - a one-hour bug fix does not need three hypotheses.
- MUST sections are not yet filled - ADI on a stub produces hallucinated context.
- Artifact is a pure EvidencePack, Note, or RefreshReport (no decision to reason about).

## Usage

```text
forgeplan reason [OPTIONS] <ID>
```

## Arguments

```text
  <ID>
          Artifact ID to analyze
```

## Options

```text
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

## Examples

### Example 1: Standard ADI on a PRD

```bash
forgeplan reason PRD-001
```

Reads the complete stored body and adds metadata, outgoing relation IDs/types/titles, a detected bounded context when available, and an architecture hint. It asks the LLM for alternatives and consequences; it does not fetch every linked artifact's full body or execute evidence tests. The architecture hint comes from `.forgeplan/prompts/architecture.md` relative to the current directory, otherwise a Forgeplan-specific default; supply your project's real architecture to avoid misleading context.

### Example 2: ADI with FPF knowledge base context

```bash
forgeplan reason PRD-001 --fpf
```

`--fpf` searches significant words from the artifact title, selects up to three unique KB sections, and injects up to 300 characters from each. It does not guarantee particular sections such as B.3 or B.5, use semantic search, or verify compliance with FPF invariants. If the KB is empty or lookup fails, reasoning continues without that context.

### Example 3: persist the analysis as a Note

```bash
forgeplan reason PRD-001 --save
```

Creates a Tactical draft Note and links Note → source with `informs`. For structured responses, the Note stores an `AdiRecord` JSON representation plus a readable summary; raw-markdown fallback stores the raw analysis. Without `--save`, no Note or link is created. Neither mode activates the source or turns the analysis into an EvidencePack.

### Example 4: Machine-readable output for agents

```bash
forgeplan reason PRD-001 --json
```

JSON has `artifact_id`, `artifact_kind`, `depth`, `r_eff_score`, `_next_action`, and either `adi_output` (structured result) or `adi_analysis` (raw-markdown fallback).

## Output interpretation

Without `--json`, the CLI prints the model response as received: it may be JSON or Markdown. The structured `adi_output` contract contains:

- `hypotheses`: objects with `id`, `description`, `assumptions`, and string `confidence`.
- `deductions`: objects with `hypothesis_id`, `consequence`, `risks`, and string `feasibility`.
- `evidence_needed`: objects with `for_hypothesis`, `test`, and string `effort`.
- String `recommendation` and `confidence`.

Confidence is free text such as “High — justification”, not a validated percentage. There is no per-hypothesis `supported`/`weakened` verdict field in this DTO. Parsing supplies defaults for absent fields; a parsed result can still have zero hypotheses. If parsing fails, `--json` returns `adi_analysis` instead of `adi_output`.

Red flags:

- Alternatives are missing or merely restate the same approach - revise the artifact, prompt, or provider/model; repeated output is not independent evidence.
- The recommendation depends on unverified assumptions - collect the targeted evidence before implementation.
- Confidence has no supporting reason - assess the justification, not an arbitrary percentage threshold.

## How it fits the workflow

This command belongs in the [full artifact lifecycle](/docs/guides/first-artifact/) - see the tutorial for the end-to-end flow. `reason` runs after `validate` PASS and before code; for Critical depth, pair with `/audit` and a human review before implementation.

## See also

- [`forgeplan route`](/docs/cli/route/) - decide whether ADI is required
- [`forgeplan decompose`](/docs/cli/decompose/) - break reasoned PRD into RFC tasks
- [`forgeplan validate`](/docs/cli/validate/) - prerequisite before reasoning
- [`forgeplan generate`](/docs/cli/generate/) - draft artifact content
- [Methodology: ADI cycle](/docs/methodology/overview/)
