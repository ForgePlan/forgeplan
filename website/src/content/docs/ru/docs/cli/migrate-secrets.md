---
title: "forgeplan migrate-secrets"
description: "PRD-077 FR-023 Part B — import known API-key env vars (GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY) from the process environment into `.forgeplan/secrets.env` so keys live project-local instead of in your shell rc. Default is dry-run; pass `--apply` to write"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan migrate-secrets --help`.

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

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
