---
title: "forgeplan migrate-dry-run"
description: "PROB-060 Phase 0b — EVID-C migration dry-run. Scans all artifacts in `.forgeplan/`, computes the slug each would receive under SPEC-005 rules, and detects per-kind collisions before Phase 4 migration. Read-only — never mutates `.md` files"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan migrate-dry-run --help`.

```text
PROB-060 Phase 0b — EVID-C migration dry-run. Scans all artifacts in `.forgeplan/`, computes the slug each would receive under SPEC-005 rules, and detects per-kind collisions before Phase 4 migration. Read-only — never mutates `.md` files.

Hybrid resolution: default = fail-and-list (exit 1 on collisions); `--auto-suffix` adds `suggested_resolution` per collision in JSON.

Usage: forgeplan migrate-dry-run [OPTIONS]

Options:
      --workspace <WORKSPACE>
          Workspace root containing `.forgeplan/`. Default: current working dir (walk-up search like other commands)

      --auto-suffix
          Include `suggested_resolution` per collision in the JSON output. Without this flag the run defaults to fail-and-list (exit 1)

      --output <OUTPUT>
          Also write the JSON report to this path (the same JSON as `--json`). Optional — caller chooses where to persist evidence

      --json
          Emit JSON to stdout. Default is a human-readable summary table

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
