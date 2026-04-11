---
title: forgeplan scan
description: "Scan the codebase for source modules — the first step of the coverage/drift pipeline."
---

`forgeplan scan` walks the project tree and builds an inventory of source modules.
It respects `.gitignore`, groups files by crate / package / directory, and writes
the resulting list into the workspace so that `coverage` and `drift` can cross-reference
it against `Affected Files` sections in artifacts.

Think of scan as the "read your own repo" step. It doesn't touch artifacts directly —
it just makes the module graph queryable by the other quality commands.

## When to use

- First time running coverage/drift in a workspace — scan must precede them.
- After a big refactor that adds/removes modules — rescan to refresh the inventory.
- In CI: rescan before any coverage check so stale module lists don't cause false positives.
- Before onboarding a teammate — rescan so `coverage` shows the real picture.

## When NOT to use

- Just to read files — use your shell. Scan is workspace-aware, not a generic file lister.
- If coverage/drift already ran and your code hasn't changed — scan is idempotent but wasteful.

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

### Scan the current project

```bash
forgeplan scan
```

Output:

```text
Scanning /Users/me/forgeplan...
  crates/forgeplan-core      14 modules
  crates/forgeplan-cli        8 modules
  crates/forgeplan-mcp        5 modules
  website/src                12 modules

39 modules indexed in .forgeplan/modules.json
```

### Scan a sibling repo

```bash
forgeplan scan --path ../other-project
```

Useful when managing multiple projects from one `.forgeplan/` workspace (advanced).

### Full coverage pipeline

```bash
forgeplan scan && forgeplan coverage && forgeplan drift
```

The canonical codebase-reconciliation sequence: refresh modules, compute coverage,
check drift. Run it monthly or after major refactors.

## Output interpretation

| Line          | Meaning                                                    |
|---------------|------------------------------------------------------------|
| per-crate row | how many source modules were found in each top-level dir  |
| total         | modules indexed — this becomes the denominator for coverage|

The scan is language-agnostic but tuned for Rust/TypeScript/Python layouts. Empty
directories and test-only modules are excluded by default.

## How it fits the workflow

```
scan → coverage (modules ↔ artifacts) → drift (artifacts ↔ git history) → remediate
```

Scan is step 0 of the quality reconciliation loop. Without it, `coverage` and `drift`
have no module graph to work against.

## See also

- [`forgeplan coverage`](/docs/cli/coverage/) — per-module decision coverage
- [`forgeplan drift`](/docs/cli/drift/) — decisions whose code moved away
- [`forgeplan scan-import`](/docs/cli/scan-import/) — rebuild LanceDB index from markdown
- [`forgeplan health`](/docs/cli/health/) — aggregates coverage into the dashboard
- [CLI overview](/docs/cli/)
