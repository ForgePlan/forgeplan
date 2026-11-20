---
title: "forgeplan reconcile-ids"
description: "PROB-060 Phase 2.4 (W2.C) — manual cleanup tool for post-merge identity drift. Scans `.forgeplan/<kind>/*.md`, detects four drift categories (filename mismatch, missing `predicted_number`, body-links drift, duplicate `assigned_number`), and either reports (`--check-only`) or auto-fixes the safe categories. LanceDB is never touched; run `forgeplan scan-import` afterwards if the index needs to be rebuilt"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

The captured help's `scan-import` recovery advice is legacy wording.
For existing canonical artifacts, use layout-appropriate `index`/`reindex`;
`scan-import` imports external Markdown, including on migrated workspaces, rather than rebuilding the canonical index. Inspect identity changes with `--check-only` first.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan reconcile-ids --help`.

```text
PROB-060 Phase 2.4 (W2.C) — manual cleanup tool for post-merge identity drift. Scans `.forgeplan/<kind>/*.md`, detects four drift categories (filename mismatch, missing `predicted_number`, body-links drift, duplicate `assigned_number`), and either reports (`--check-only`) or auto-fixes the safe categories. LanceDB is never touched; run `forgeplan scan-import` afterwards if the index needs to be rebuilt

Usage: forgeplan reconcile-ids [OPTIONS]

Options:
      --workspace <WORKSPACE>  Workspace root containing `.forgeplan/`. Default: walk-up search from cwd. May also be a `.forgeplan/` directory directly
      --check-only             Report inconsistencies without modifying files. Default is apply mode (auto-rename + auto-fill `predicted_number`)
      --report-cross-pr        Forward-compat flag for RFC-009 §Phase 2.4 cross-PR ref-drift detection. The current implementation surfaces a no-op marker in the JSON output; cross-PR walking is deferred (see module docs)
      --json                   Emit JSON report to stdout (schema_version 1). Default is a scannable human-readable summary
      --plan-out <FILE>        Write a version-1 identity-repair plan to this file. Planning is read-only: the workspace is not changed, and only the plan file is written
      --apply-plan <FILE>      Apply a version-1 identity-repair plan. The plan is re-checked against the workspace under the workspace lock; a stale plan is refused
      --after-blocked <TXN>    Link a new plan or an explicit rollback recovery to a blocked transaction
      --recover <TXN>          Recover an identity-repair transaction by its identifier. With `--rollback` the transaction's preimages are restored
      --publish-prepared       Publish a fully prepared staged journal before explicit recovery
      --rollback               Roll the recovered transaction back to its preimages
      --reconcile-index        Retry owner-specific index reconciliation for a pending repair
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
