---
title: forgeplan generate
description: "Generate an artifact using AI from a natural language description"
---

Generates a **fully-drafted artifact** from a natural language description using an LLM (Gemini / OpenAI / Anthropic, configured in `.forgeplan/config.yaml`). Unlike [`forgeplan new`](/docs/cli/new/), which produces an empty template you fill manually, `generate` asks the model to write Problem, Goals, Non-Goals, Functional Requirements, and the rest of the MUST sections in one pass. The result is a draft you still review — but the 30-minute cold-start of staring at an empty template disappears.

## When to use

- You know what you want and would rather review an LLM draft than type the boilerplate yourself.
- **Prototype artifact for discussion**: generate a throw-away PRD to explore a half-formed idea with the team before committing to the real one.
- **Brown-field discovery**: describe an existing subsystem and ask the model to reverse-engineer a PRD / Spec / RFC for documentation.
- You have a clear one-liner ("add rate limiting to /api/v1/") and want all the sections filled consistently.
- Fast iteration during Shape phase when you want to try 2–3 framings quickly.

## When NOT to use

- You don't have LLM credentials configured — run `forgeplan config set llm.provider ...` first, or fall back to [`forgeplan new`](/docs/cli/new/).
- The decision is tactical and doesn't need an artifact at all — just commit.
- You want full manual control over wording — use [`forgeplan new`](/docs/cli/new/) and fill sections yourself.
- You're logging a decision from a conversation — use [`forgeplan capture`](/docs/cli/capture/).
- The content already exists in a saved memory — use [`forgeplan promote`](/docs/cli/promote/).

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

`generate` reads your active LLM provider from `.forgeplan/config.yaml`. If no provider is set or the API key is missing, the command fails early with a config hint instead of falling back to an empty stub.

## Examples

### Example 1: Generate a PRD from a one-liner

```bash
forgeplan generate prd "add rate limiting to /api/v1/ endpoints"
```

Produces `.forgeplan/prds/prd-NNN-add-rate-limiting.md` with Problem ("public endpoints are exposed to abuse..."), Goals, Non-Goals, Target Users, and an FR list drafted by the model. Open the file, read critically, tighten the wording, then run `forgeplan validate PRD-NNN` to confirm MUST rules pass.

### Example 2: Reverse-engineer an RFC for an existing subsystem

```bash
forgeplan generate rfc "current embedding pipeline: fastembed BGE-M3, 1024 dims, batch 32, cached in .forgeplan/.fastembed_cache/"
```

Good for brown-field documentation — the LLM drafts an RFC describing the as-built architecture with Implementation Phases already checked off. Use this as a starting point, then correct any hallucinated details against the real code.

### Example 3: Draft a ProblemCard for a fresh signal

```bash
forgeplan generate problem "users report search returns stale results after renaming artifacts"
```

Generates Signal / Context / Goals / Anti-Goodhart indicators sections. ProblemCards don't require a validation gate, so you can `activate` as soon as the card is coherent and link follow-up Evidence or Solutions.

## How it fits the workflow

```
route  →  generate  →  (review + edit)  →  validate  →  reason  →  code  →  evidence  →  review  →  activate
```

`generate` slots into the same Shape phase as `new`, but collapses "create stub" and "fill MUST sections" into one LLM call. The rest of the pipeline is unchanged: you still need `validate` to pass, `reason` for Deep/Critical depth, and Evidence before `activate`. Treat the generated text as a first draft — the model will not catch domain-specific constraints the way a human operator will.

## See also

- [`forgeplan new`](/docs/cli/new/) — manual template stub, full control over wording
- [`forgeplan validate`](/docs/cli/validate/) — required after editing the draft
- [`forgeplan reason`](/docs/cli/reason/) — ADI cycle to verify hypotheses before coding
- [`forgeplan capture`](/docs/cli/capture/) — log decisions from a live conversation
- [Methodology: artifact model](/docs/methodology/overview/)
