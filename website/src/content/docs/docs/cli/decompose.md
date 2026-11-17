---
title: forgeplan decompose
description: "Ask the configured LLM to suggest RFC-sized tasks for an existing PRD. The command prints titles, descriptions, scope, and dependencies. In 0.38.0 it does not create RFC artifacts or links."
---

Ask the configured LLM to suggest RFC-sized tasks for an existing PRD. The command prints titles, descriptions, scope, and dependencies. In 0.38.0 it does not create RFC artifacts or links.

## Usage

```text
forgeplan decompose <ID>
```

## Arguments

```text
  <ID>  PRD artifact ID to decompose
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## When to use

Use after shaping and validating a Standard-or-deeper PRD whose implementation needs several phases. Do not decompose a stub or avoid resolving disputed goals. If an RFC already covers the work, update it instead of creating duplicates.

The workspace and source artifact must exist, and `llm` must be configured with the selected provider's authentication or local login. This command does not require embedding weights. A non-PRD input produces a warning and is decomposed anyway; validation, depth, status, and prior ADI are workflow recommendations, not enforced prerequisites. Missing LLM configuration, missing artifacts, and failed LLM calls return errors. The result is provider-generated text, not a guaranteed structured task DTO, and there is no `--json` flag.

## Example workflow

```bash
forgeplan validate PRD-019
forgeplan reason PRD-019 --fpf
forgeplan decompose PRD-019
# Review suggestions and create chosen RFCs explicitly:
forgeplan new rfc "Authentication provider abstraction"
# Use the returned ID, for example RFC-018:
forgeplan link RFC-018 PRD-019 --relation refines
```

## Review and persist suggestions

Printed tasks are proposals, not persisted phases. Fill required sections and phase checkboxes in each chosen RFC, link it with `refines`, and validate before implementation. `progress` tracks checkboxes once they exist. Check for invented requirements, oversized tasks, missing acceptance criteria, rollback, or evidence work. Decomposition does not implement or verify a solution.

## See also

- [`forgeplan reason`](/docs/cli/reason/) - run ADI before decomposing
- [`forgeplan new`](/docs/cli/new/) - create the source PRD
- [`forgeplan validate`](/docs/cli/validate/) - gate for decomposition input
- [`forgeplan generate`](/docs/cli/generate/) - generate content for any artifact kind
- [Methodology: PRD → RFC flow](/docs/methodology/overview/)
