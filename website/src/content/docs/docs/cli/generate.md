---
title: forgeplan generate
description: "Generate an artifact using AI from a natural language description"
---

Generates an LLM-written Markdown draft from a description. The required `llm:` block in `.forgeplan/config.yaml` supplies provider/model defaults; environment variables can override them. HTTP providers include `openai`, `claude` (Anthropic), `gemini`, `ollama`, and `custom`; explicit subscription-CLI providers reuse their own local login. No provider is auto-detected. The prompt requests a kind-specific structure, but the output is not validated or checked against the code before it is saved.

## When to use

- You know what you want and would rather review an LLM draft than type the boilerplate yourself.
- **Prototype artifact for discussion**: generate a throw-away PRD to explore a half-formed idea with the team before committing to the real one.
- **Brown-field discovery**: describe an existing subsystem and ask the model to reverse-engineer a PRD / Spec / RFC for documentation.
- You have a clear one-liner ("add rate limiting to /api/v1/") and want all the sections filled consistently.
- Fast iteration during Shape phase when you want to try 2-3 framings quickly.

## When NOT to use

- The chosen provider is unavailable — configure its API key or local login, or use [`forgeplan new`](/docs/cli/new/) without an LLM.
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
  <DESCRIPTION>  Description of what to generate
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

`generate` has no provider/model flags and no JSON output flag. Missing required credentials produce a configuration error; keyless providers do not need an API key. The command does not fall back to a template when generation fails. It sends your description to the configured provider; it does not read your subsystem's source files.

Effective settings apply `FORGEPLAN_LLM_PROVIDER`, `FORGEPLAN_LLM_MODEL`,
`FORGEPLAN_LLM_BASE_URL`, `FORGEPLAN_LLM_MAX_TOKENS`, and
`FORGEPLAN_LLM_API_KEY_ENV` after the YAML defaults. The last variable changes
the **name** of the credential variable, not the key value. Maximum tokens
is overridden only when its value parses as an unsigned 32-bit integer.
These environment variables do not replace the prerequisite for an `llm:`
block, even for a keyless provider. Check inherited overrides when the
effective provider, endpoint, model, or credential lookup is unexpected.

## Examples

### Example 1: Generate a PRD from a one-liner

```bash
forgeplan generate prd "add rate limiting to /api/v1/ endpoints"
```

Creates a `draft` at `depth: standard`. The metadata title is the first description line, truncated to 80 characters; the assigned ID and actual projection path are printed as `ID:` and `Created:`. Use those values rather than predicting the filename. Review the model's content, then run `forgeplan validate <returned-id>`.

### Example 2: Reverse-engineer an RFC for an existing subsystem

```bash
forgeplan generate rfc "current embedding pipeline: tract CPU inference, BGE-M3, 1024 dimensions, shared machine-wide cache"
```

Useful as a draft based on the supplied description, not an as-built audit. The prompt asks for unchecked actionable checkboxes; completed implementation phases are not automatically established. Compare every architecture claim against source before accepting it.

### Example 3: Draft a ProblemCard for a fresh signal

```bash
forgeplan generate problem "users report search returns stale results after renaming artifacts"
```

The Problem prompt requests Signal / Context / Impact / Anti-Goodhart Indicators. The actual sections depend on the model response. Review the content before using the lightweight activation path; a model-generated assertion is not evidence.

## How it fits the workflow

```
route  →  generate  →  (review + edit)  →  validate  →  reason  →  code  →  evidence  →  review  →  activate
```

`generate` replaces manual first-draft writing, not validation or review. It does not set reasoning state, create evidence links, or activate the result. Any further reasoning/evidence requirements come from your methodology and the separate command guards, not from automatic generation behavior.

## See also

- [`forgeplan new`](/docs/cli/new/) - manual template stub, full control over wording
- [`forgeplan validate`](/docs/cli/validate/) - required after editing the draft
- [`forgeplan reason`](/docs/cli/reason/) - ADI cycle to verify hypotheses before coding
- [`forgeplan capture`](/docs/cli/capture/) - log decisions from a live conversation
- [Methodology: artifact model](/docs/methodology/overview/)
