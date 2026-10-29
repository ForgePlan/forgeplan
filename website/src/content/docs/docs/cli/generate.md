---
title: forgeplan generate
description: "Generate an artifact using AI from a natural language description"
---

Generates an **LLM-drafted artifact** from a natural-language description using the provider configured in `.forgeplan/config.yaml`. Unlike [`forgeplan new`](/docs/cli/new/), it asks the model to fill the template's sections. Completeness, accuracy, and time saved are not guaranteed; review and validate the result.

## When to use

- You know what you want and would rather review an LLM draft than type the boilerplate yourself.
- **Prototype artifact for discussion**: generate a throw-away PRD to explore a half-formed idea with the team before committing to the real one.
- **Brown-field discovery**: describe an existing subsystem and ask the model to reverse-engineer a PRD / Spec / RFC for documentation.
- You have a clear one-liner ("add rate limiting to /api/v1/") and want all the sections filled consistently.
- Fast iteration during Shape phase when you want to try 2–3 framings quickly.

## When NOT to use

- No LLM provider is configured - follow [configuration](/docs/getting-started/configuration/) or use [`forgeplan new`](/docs/cli/new/). There is no `forgeplan config set` command.
- The decision is tactical and doesn't need an artifact at all - just commit.
- You want full manual control over wording - use [`forgeplan new`](/docs/cli/new/) and fill sections yourself.
- You're logging a decision from a conversation - use [`forgeplan capture`](/docs/cli/capture/).
- The content already exists in a saved memory - use [`forgeplan promote`](/docs/cli/promote/).

## Usage

```text
forgeplan generate <KIND> <DESCRIPTION>
```

## Arguments

```text
  <KIND>         Artifact kind: prd, epic, spec, rfc, adr, problem, solution, evidence
  <DESCRIPTION>  Natural language description of what to generate
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

`generate` reads project `llm` configuration. HTTP providers need the configured API-key environment variable; explicitly opted-in local subscription providers use an existing vendor CLI login. Missing configuration or provider failure is an error, not a successful empty draft.

## Examples

### Example 1: Generate a PRD from a one-liner

```bash
forgeplan generate prd "add rate limiting to /api/v1/ endpoints"
```

Reports the created path and ID. The model is asked for the PRD sections, but their actual content must be checked. Review the returned artifact and run `forgeplan validate` with its actual ID; do not infer the filename or ID from this illustration.

### Example 2: Reverse-engineer an RFC for an existing subsystem

```bash
forgeplan generate rfc "selected Constellation embedding pipeline: tract BGE-M3, 1024 dimensions, model cache root resolved from FORGEPLAN_MODEL_CACHE or ~/.forgeplan/models"
```

Use this as a brownfield writing prompt, not a code-inspection result. Confirm the as-built details against the target source and mark implementation phases complete only when actual evidence supports them.

### Example 3: Draft a ProblemCard for a fresh signal

```bash
forgeplan generate problem "users report search returns stale results after renaming artifacts"
```

The model is asked to draft the ProblemCard template. Review Signal, Context, Goals, and Anti-Goodhart content, attach real Evidence or Solutions where applicable, and satisfy validation and lifecycle gates before activation.

## How it fits the workflow

```
route  →  generate  →  (review + edit)  →  validate  →  reason  →  code  →  evidence  →  review  →  activate
```

`generate` slots into the same Shape phase as `new`, but collapses "create stub" and "fill MUST sections" into one LLM call. The rest of the pipeline is unchanged: you still need `validate` to pass, `reason` for Deep/Critical depth, and Evidence before `activate`. Treat the generated text as a first draft - the model will not catch domain-specific constraints the way a human operator will.

## See also

- [`forgeplan new`](/docs/cli/new/) - manual template stub, full control over wording
- [`forgeplan validate`](/docs/cli/validate/) - required after editing the draft
- [`forgeplan reason`](/docs/cli/reason/) - ADI cycle to verify hypotheses before coding
- [`forgeplan capture`](/docs/cli/capture/) - log decisions from a live conversation
- [Methodology: artifact model](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan generate --help`.

```text
Generate an artifact using AI from a natural language description

Usage: forgeplan generate <KIND> <DESCRIPTION>

Arguments:
  <KIND>         Artifact kind: prd, epic, spec, rfc, adr, problem, solution, evidence
  <DESCRIPTION>  Description of what to generate

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
