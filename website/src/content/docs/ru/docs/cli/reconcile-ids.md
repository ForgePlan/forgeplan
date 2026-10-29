---
title: "forgeplan reconcile-ids"
description: "PROB-060 Phase 2.4 (W2.C) — manual cleanup tool for post-merge identity drift. Scans `.forgeplan/<kind>/*.md`, detects four drift categories (filename mismatch, missing `predicted_number`, body-links drift, duplicate `assigned_number`), and either reports (`--check-only`) or auto-fixes the safe categories. LanceDB is never touched; run `forgeplan scan-import` afterwards if the index needs to be rebuilt"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

Сохранённая справка ниже советует `scan-import` для перестройки индекса, но эта команда импортирует внешний Markdown, в том числе через migrated file-first путь. Для перестройки индекса из канонических источников используйте `forgeplan index` или `forgeplan reindex`. Режим `--check-only` проверяет изменения идентичности без переписывания источников.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan reconcile-ids --help`.

```text
PROB-060 Phase 2.4 (W2.C) — manual cleanup tool for post-merge identity drift. Scans `.forgeplan/<kind>/*.md`, detects four drift categories (filename mismatch, missing `predicted_number`, body-links drift, duplicate `assigned_number`), and either reports (`--check-only`) or auto-fixes the safe categories. LanceDB is never touched; run `forgeplan scan-import` afterwards if the index needs to be rebuilt

Usage: forgeplan reconcile-ids [OPTIONS]

Options:
      --workspace <WORKSPACE>  Workspace root containing `.forgeplan/`. Default: walk-up search from cwd. May also be a `.forgeplan/` directory directly
      --check-only             Report inconsistencies without modifying files. Default is apply mode (auto-rename + auto-fill `predicted_number`)
      --report-cross-pr        Forward-compat flag for RFC-009 §Phase 2.4 cross-PR ref-drift detection. The current implementation surfaces a no-op marker in the JSON output; cross-PR walking is deferred (see module docs)
      --json                   Emit JSON report to stdout (schema_version 1). Default is a scannable human-readable summary
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
