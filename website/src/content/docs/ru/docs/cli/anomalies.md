---
title: "forgeplan anomalies"
description: "Issue #289 — detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, etc.). Each entry carries severity + tier + suggested_resolution so orchestrators can dispatch fixes"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan anomalies --help`.

```text
Issue #289 — detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, etc.). Each entry carries severity + tier + suggested_resolution so orchestrators can dispatch fixes

Usage: forgeplan anomalies [OPTIONS]

Options:
      --kind <KIND>          Filter by anomaly kind (snake_case: stuck_draft, orphan_link, mistyped_based_on, missing_must_section, expired_evidence, weakest_link_unresolvable, phase_mismatch, circular_dependency, duplicate_artifact)
      --severity <SEVERITY>  Filter by severity (low, medium, high)
      --since <SINCE>        Only return anomalies observed at or after this RFC 3339 timestamp
      --json                 Output as JSON instead of human-readable table
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
