---
title: forgeplan capture
description: "Capture a decision from a conversation using the configured LLM. Simple decisions become Notes; architectural decisions can become ADRs. The result is a draft artifact at Tactical depth, not an automatically validated or activated decision."
---

Capture a decision from a conversation using the configured LLM. Simple decisions become Notes; architectural decisions can become ADRs. The result is a draft artifact at Tactical depth, not an automatically validated or activated decision.

## Usage

```text
forgeplan capture [OPTIONS] <DECISION>
```

## Arguments

```text
  <DECISION>  The decision statement
```

## Options

```text
      --context <CONTEXT>  Additional context (optional)
  -h, --help               Print help
  -V, --version            Print version
```

## Prerequisites and boundaries

Configure an LLM provider in `.forgeplan/config.yaml`. Without one, capture fails rather than creating a deterministic Note. `--context` supplies background. Capture does not set `valid_until`, so it does not itself assign a 90-day expiry. Use `new note` or `new adr` to choose the kind and fill a template manually.

API-backed providers need their configured credentials; Ollama is keyless local HTTP, and explicit stock-CLI profiles use their existing login. These are provider prerequisites, not an embedding-model requirement. Capture prints progress, the projection path, assigned ID, detected kind, title, and a follow-up hint; there is no `--json` flag. A malformed kind prefix in the model response falls back to Note, not an error.

## Examples

```bash
forgeplan capture "Retry server failures, not malformed requests" --context "4xx responses need corrected input"
forgeplan capture "Markdown is authoritative; LanceDB is a derived index"
```

## Review the result

Read the returned artifact, check the selected kind and reasoning, and refine it with `update --body @file` if necessary. Link it to a relevant PRD or RFC with `informs`. Validate and review before activation; generated prose is not proof that the decision is correct.

## See also

- [`forgeplan new note`](/docs/cli/new/) - manual Note creation with template
- [`forgeplan new adr`](/docs/cli/new/) - manual ADR creation for full workflow
- [`forgeplan promote`](/docs/cli/promote/) - turn a saved memory into an artifact
- [`forgeplan link`](/docs/cli/link/) - attach the captured decision to a parent PRD / RFC
- [Methodology: depth calibration](/docs/methodology/overview/)
