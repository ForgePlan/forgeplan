---
title: "forgeplan migrate-secrets"
description: "PRD-077 FR-023 Part B — import known API-key env vars (GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY) from the process environment into `.forgeplan/secrets.env` so keys live project-local instead of in your shell rc. Default is dry-run; pass `--apply` to write"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan migrate-secrets --help`.

```text
PRD-077 FR-023 Part B — import known API-key env vars (GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY) from the process environment into `.forgeplan/secrets.env` so keys live project-local instead of in your shell rc. Default is dry-run; pass `--apply` to write

Usage: forgeplan migrate-secrets [OPTIONS]

Options:
      --workspace <WORKSPACE>  Workspace root containing `.forgeplan/`. Default: current working dir (walk-up search like other commands)
      --apply                  Apply the migration (write `.forgeplan/secrets.env`). Default is dry-run — only lists what would be migrated
      --json                   Emit a JSON summary to stdout instead of human-readable text. Useful for scripting / CI integration
  -h, --help                   Print help
  -V, --version                Print version
```
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
