---
title: forgeplan scan
description: "Scan source modules and print their paths, file counts, and line counts. This is a read-only report; it does not write a persistent .forgeplan/modules.json inventory."
---

Scan source modules and print their paths, file counts, and line counts. This is a read-only report; it does not write a persistent `.forgeplan/modules.json` inventory.

## Usage

```text
forgeplan scan [OPTIONS]
```

## Options

```text
      --path <PATH>  Path to project root (default: current dir)
  -h, --help         Print help
  -V, --version      Print version
```

## Examples

```bash
forgeplan scan
forgeplan scan --path crates
forgeplan coverage
forgeplan drift
```

## Path restrictions

`--path` must resolve inside the current project root. A sibling path such as `../other-project` is rejected; change to that project explicitly if you intend to scan it. Paths must exist and be accessible.

## Relationship to coverage

`coverage` scans modules itself, so running `scan` first is optional, not a prerequisite. Use it to inspect discovery results after a refactor, then relate real affected paths to decisions. `scan-import` is a different operation that imports external documents.

## See also

- [`forgeplan coverage`](/docs/cli/coverage/) - per-module decision coverage
- [`forgeplan drift`](/docs/cli/drift/) - decisions whose code moved away
- [`forgeplan reindex`](/docs/cli/reindex/) - scan workspace projections for new records, body sync, missing links, and orphan cleanup; not a complete metadata/removed-link refresh for existing rows
- [`forgeplan health`](/docs/cli/health/) - aggregates coverage into the dashboard
- [CLI overview](/docs/cli/)
