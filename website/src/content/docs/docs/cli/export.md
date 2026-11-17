---
title: forgeplan export
description: "Export artifact records and typed relations to a version-1 JSON bundle. It includes bodies, metadata, tags, cached R_eff, validity dates, and relation endpoints. It is not a full workspace or change-history backup."
---

Export artifact records and typed relations to a version-1 JSON bundle. It includes bodies, metadata, tags, cached R_eff, validity dates, and relation endpoints. It is not a full workspace or change-history backup.

## Usage

```text
forgeplan export [OPTIONS]
```

## Options

```text
  -o, --output <OUTPUT>  Output file path (default: .forgeplan/export.json)
  -h, --help             Print help
  -V, --version          Print version
```

## Forgeplan 0.38.0

The bundle contains r_eff_score and created_at/updated_at, but import 0.38.0 does not replay those values: scores need a separate refresh and timestamps are newly assigned. This is a portable artifact/relation bundle, not a byte-exact database or history backup.

## What is not exported

Configuration, API-key environment values, model caches, claims, trash receipts, session state, and other workspace files are not included. Keep a separate protected filesystem backup if needed. Do not assume an export reproduces every health or provider setting on another machine.

## Examples

```bash
forgeplan export --output ../forgeplan-backup.json
cp -R .forgeplan ../forgeplan-workspace-backup
# In a separately initialized destination workspace:
forgeplan import ../forgeplan-backup.json
forgeplan health
```

## Backup and sharing

The default is `.forgeplan/export.json`; choose an external path before replacing workspace files. Exports are unencrypted and may contain sensitive artifact content. Review before sharing; do not publish a bundle automatically. For a fresh clone with tracked markdown, use `reindex` instead of export/import.

## See also

- [`forgeplan import`](/docs/cli/import/) - the restore counterpart
- [`forgeplan init`](/docs/cli/init/) - create or refresh a workspace
- [`forgeplan migrate`](/docs/cli/migrate/) - non-destructive schema upgrade (still back up first)
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild index from tracked markdown
- [`forgeplan health`](/docs/cli/health/) - verify the workspace before and after backup cycles
