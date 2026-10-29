---
title: forgeplan scan-import
description: "Discover foreign Markdown documents and import them as canonical Forgeplan artifacts."
---

`forgeplan scan-import` discovers foreign Markdown in standard documentation directories or a custom `--path`, then writes canonical Forgeplan artifacts. On Constellation it requests reconciliation after a successful batch. It does not synchronize existing canonical files or rebuild their index; use `forgeplan index` for Constellation and the corresponding legacy `forgeplan reindex`.

## When to use

- **Brownfield onboarding.** You have existing RFCs, PRDs, or ADRs in markdown and want to bring them under Forgeplan methodology without rewriting them.
- **Previewing an unfamiliar document collection.** Use `--dry-run` before importing.
- **Custom documentation directories.** Point `--path` at the foreign source tree.
- **Not canonical-index recovery.** Do not delete `.forgeplan/lance/` or its ownership markers to make this command appear to recover the index.

## When NOT to use

- To restore a full workspace backup — JSON import also has limits; retain canonical files and other workspace state.
- To create new artifacts - use `forgeplan new <kind>`. `scan-import` is for discovering existing files.
- To repair schema mismatches - use [`forgeplan migrate`](/docs/cli/migrate/). `scan-import` assumes the schema is already current.

## Usage

```text
forgeplan scan-import [OPTIONS]
```

## Options

```text
      --path <PATH>  Directory to scan (default: standard doc dirs)
      --dry-run      Preview only, don't actually import
  -h, --help         Print help
  -V, --version      Print version
```

## Examples

### Example 1: Preview documents after cloning

```bash
git clone <repo>
cd <repo>
# Initialize only if this project has no workspace yet
forgeplan init -y
forgeplan scan-import --dry-run
```

For a clone already containing canonical `.forgeplan/` Markdown, maintain its owning index instead. Do not treat external-document scanning as a canonical rebuild.

### Example 2: Dry run before committing to import

```bash
forgeplan scan-import --dry-run
```

Prints the list of files that would be imported, their detected artifact kind, and any parse warnings - without touching LanceDB. Use this on unfamiliar legacy repos before the real run.

### Example 3: Brownfield onboarding with custom path

```bash
forgeplan init -y
forgeplan scan-import --path docs/architecture
forgeplan list
forgeplan health
```

Points the scanner at a non-standard directory. Useful when legacy docs live under `docs/`, `architecture/`, or `decisions/` instead of `.forgeplan/`.

### Example 4: Maintain a Constellation index without deleting it

```bash
forgeplan index
forgeplan index --list
forgeplan health
```

This is a different operation from `scan-import`. Keep canonical Markdown, generation files and ownership markers intact; follow layout-specific recovery guidance if publication fails.

## How it fits the workflow

`scan-import` bridges foreign documents into canonical artifacts.

- **Brownfield setup**: preview with `scan-import --dry-run`, then import accepted sources.
- **Combined setup**: `init --scan` requests scanning during initialization.
- **Canonical maintenance**: `index` on Constellation, not another external import.

Existing identities may be skipped. Repeating an import is not a guarantee that external edits update an existing artifact.

## Safety notes

- **Do not delete workspace/index directories as a routine recovery step.** Preserve canonical sources and the existing snapshot while diagnosing a failure.
- **`--dry-run` first on unfamiliar repos.** Scanning can surface files you didn't intend to import (`README.md`, `CHANGELOG.md`, etc.). Preview before committing.
- **For deliberate canonical edits, use `forgeplan update`.** Reconcile with the owning index pipeline, not `scan-import`.
- **Scan respects frontmatter.** Files without valid Forgeplan YAML frontmatter (kind, id, status) may be skipped or imported as generic Notes depending on detection heuristics.
- **No full history recovery guarantee.** Neither scanning foreign documents nor importing an artifact bundle restores every workspace history field.

## See also

- [`forgeplan init`](/docs/cli/init/) - create the workspace shell before scanning; `init --scan` combines both steps
- [`forgeplan import`](/docs/cli/import/) - import supported JSON artifact/relation fields, not all workspace state
- [`forgeplan export`](/docs/cli/export/) - safety backup before any destructive operation
- [`forgeplan migrate`](/docs/cli/migrate/) - apply schema changes without touching content
- [`forgeplan health`](/docs/cli/health/) - verify the index matches expectations after scan
- [ADR-003](/docs/methodology/overview/) - why markdown is the source of truth

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan scan-import --help`.

```text
Scan for existing docs and import as artifacts

Usage: forgeplan scan-import [OPTIONS]

Options:
      --path <PATH>  Directory to scan (default: standard doc dirs)
      --dry-run      Preview only, don't actually import
  -h, --help         Print help
  -V, --version      Print version
```
<!-- forgeplan-runtime:end -->
