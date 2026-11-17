---
title: forgeplan new
description: "Create a new artifact from a template stub"
---

Create a draft template in the appropriate `.forgeplan/` artifact directory. This starts the Shape phase: fill the body, then validate it. PRD/Epic/Spec/RFC/ADR activation normally rejects incomplete MUST sections and other failed gates; `--force` is an explicit exceptional override, and lightweight kinds have different activation rules.

## Forgeplan 0.38.0

Every new artifact starts as draft without an automatic valid_until. Note/evidence/problem/solution/refresh default to tactical depth; other kinds default to standard. Beyond the ten kinds listed in help, the parser accepts memory, use_case, glossary, invariant, scenario, hypothesis, and domain_model. A similar title of the same kind prompts in a terminal and refuses in a noninteractive invocation. --allow-duplicate (new’s --force alias) explicitly bypasses that check; it is not activation --force. Creation does not call an LLM.

## When to use

- Starting the **Shape** phase of any Standard+ task: create a PRD / RFC / ADR before writing code so the reasoning is captured.
- Recording a tactical Note without needing the LLM - `new note "..."`.
- Explicit template control - when you want the exact stub layout rather than an LLM-drafted body (use [`forgeplan generate`](/docs/cli/generate/) for the LLM path).

## When NOT to use

- For a tactical one-file fix with no design choices, whether to create a Note depends on project policy; the CLI does not require a PRD for every edit.
- The decision came out of a live conversation and you only want to log it - use [`forgeplan capture`](/docs/cli/capture/).
- You already have a saved memory you want to turn into an artifact - use [`forgeplan promote`](/docs/cli/promote/).

## Usage

```text
forgeplan new [OPTIONS] <KIND> <TITLE>
```

## Arguments

```text
  <KIND>   Artifact kind: prd, epic, spec, rfc, adr, problem, solution, evidence, note, refresh
  <TITLE>  Artifact title
```

## Options

```text
      --allow-duplicate  Skip duplicate-detection prompt and create anyway [alias: --force]
  -h, --help             Print help
  -V, --version          Print version
```

Forgeplan runs a similarity check against existing artifacts and warns if the title looks like a duplicate. Pass `--allow-duplicate` / `--force` to bypass when you intentionally want a parallel artifact (e.g., a second PRD exploring the same domain from a different angle).

## Examples

### Example 1: Create a PRD for a new feature

```bash
forgeplan new prd "OAuth2 login"
```

Prints the actual created path and ID. Use those returned values rather than guessing a pre-merge slug or display-ID filename. Immediately open it and fill in Problem, Goals, Non-Goals, Target Users, Related, and Functional Requirements - then run `forgeplan validate PRD-NNN` to confirm 0 MUST errors before moving on.

### Example 2: Record a problem signal

```bash
forgeplan new problem "Slow search on 10k+ artifacts"
```

Creates a ProblemCard with Signal / Context / Goals / Anti-Goodhart indicators sections. ProblemCards don't need a design - fill Signal and Context, then link to any Evidence you already have. Problems don't require a validation gate for activation, so you can activate them as soon as the card is coherent.

### Example 3: Attach a benchmark as evidence

```bash
forgeplan new evidence "Benchmark auth handler under 500 concurrent users"
# Fill the real evidence body first; substitute its returned ID below.
forgeplan link EVID-001 PRD-012 --relation informs
forgeplan score PRD-012
```

Evidence bodies MUST contain structured fields (`verdict`, `congruence_level`, `evidence_type`) - without them the R_eff parser falls back to CL0 and penalises the score. Link to the artifact the evidence supports, then inspect the recursive score; a new link does not guarantee a higher R_eff.

## How it fits the workflow

This command belongs in the [full artifact lifecycle](/docs/guides/first-artifact/) - see the tutorial for the end-to-end flow. `new` only produces a stub: **work is not done until MUST sections are filled and `validate` passes.** Unfilled drafts can appear as stale-draft or orphan findings; blind spots specifically concern active decisions without evidence support. Ordinary guarded activation rejects incomplete work.

## See also

- [`forgeplan generate`](/docs/cli/generate/) - let the LLM draft the artifact body instead of filling it manually
- [`forgeplan validate`](/docs/cli/validate/) - run MUST/SHOULD rules; required before `activate`
- [`forgeplan route`](/docs/cli/route/) - decide depth + pipeline before creating the artifact
- [`forgeplan health`](/docs/cli/health/) - surface stubs and blind spots after creation
- [Methodology: artifact model](/docs/methodology/overview/)
