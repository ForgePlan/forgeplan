---
title: forgeplan coverage
description: "Scan the codebase and report which source modules are covered by artifact Affected Files sections. This measures documented decision coverage, not test coverage or proof that the implementation matches a decision."
---

Scan the codebase and report which source modules are covered by artifact Affected Files sections. This measures documented decision coverage, not test coverage or proof that the implementation matches a decision.

## Usage

```text
forgeplan coverage [OPTIONS]
```

## Options

```text
      --backfill  Backfill "Affected Files" section into artifacts missing it
  -h, --help      Print help
  -V, --version   Print version
```

## Examples

```bash
forgeplan coverage
forgeplan coverage --backfill
forgeplan drift
```

## Backfill and interpretation

Only active PRD/RFC/ADR bodies contribute decision paths. `--backfill` updates
stored bodies missing both `## Affected Files` and `## Affected Scope`; it appends
the fixed example paths `crates/forgeplan-core/src/**` and
`crates/forgeplan-cli/src/**`, not inferred project paths. Those examples can
inflate coverage in a matching repository, so replace them with real scope.
In 0.38.0 this backfill does not render Markdown projections; do not assume
the body update is durable across a later file-to-store sync.

The normal report lists uncovered modules, covered modules with decision IDs,
and a percentage. Matching strips limited trailing patterns (`/*`, `/**`,
`/*.rs`), then checks whether either path string contains the other. This is
a symmetric substring heuristic, not prefix/boundary matching or a general
glob engine, and does not prove a decision governs every file. No healthy percentage is enforced.

## CLI and MCP differences

`coverage` scans modules itself; `scan` is useful for inspection but is not required first. CLI 0.38.0 has no `--json` option for coverage. For a structured integration, use the separately documented `forgeplan_coverage` MCP tool and its own schema.

## See also

- [`forgeplan scan`](/docs/cli/scan/) - populate the module inventory
- [`forgeplan drift`](/docs/cli/drift/) - the other half of codebase reconciliation
- [`forgeplan new adr`](/docs/cli/new/) - create the missing documentation
- [`forgeplan health`](/docs/cli/health/) - includes coverage in project-level summary
- [CLI overview](/docs/cli/)
