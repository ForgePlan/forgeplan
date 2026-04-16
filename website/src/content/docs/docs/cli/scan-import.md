---
title: forgeplan scan-import
description: "Scan the filesystem for markdown docs and import them as Forgeplan artifacts — brownfield onboarding and index rebuild"
---

`forgeplan scan-import` walks the filesystem (standard doc directories by default, or a custom `--path`), detects markdown files that look like Forgeplan artifacts, and imports them into the LanceDB index. It is the primary tool for two scenarios: onboarding a legacy project that already has docs, and rebuilding the derived index from tracked markdown after a fresh `git clone`.

## When to use

- **Brownfield onboarding.** You have existing RFCs, PRDs, or ADRs in markdown and want to bring them under Forgeplan methodology without rewriting them.
- **Fresh git clone.** `.forgeplan/{adrs,prds,rfcs,...}` is tracked in git but `.forgeplan/lance/` is not — `scan-import` rebuilds the index from the tracked markdown.
- **After bulk external edits.** If you edited markdown directly (outside `forgeplan update`), `scan-import` re-syncs the LanceDB index to match.
- **Index recovery.** `rm -rf .forgeplan/lance && forgeplan scan-import` is the safe way to nuke and rebuild the derived layer without touching artifacts.

## When NOT to use

- To restore a backup — use [`forgeplan import`](/docs/cli/import/) with a JSON file. `scan-import` only reads markdown and cannot reconstruct scoring history or decay state.
- To create new artifacts — use `forgeplan new <kind>`. `scan-import` is for discovering existing files.
- To repair schema mismatches — use [`forgeplan migrate`](/docs/cli/migrate/). `scan-import` assumes the schema is already current.

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

### Example 1: Rebuild index after git clone

```bash
git clone <repo> && cd <repo>
forgeplan init -y
forgeplan scan-import
forgeplan list
```

The standard "fresh clone" recipe. `.forgeplan/lance/` is gitignored, so every new checkout needs an `init` + `scan-import` to recreate the derived index from tracked markdown.

### Example 2: Dry run before committing to import

```bash
forgeplan scan-import --dry-run
```

Prints the list of files that would be imported, their detected artifact kind, and any parse warnings — without touching LanceDB. Use this on unfamiliar legacy repos before the real run.

### Example 3: Brownfield onboarding with custom path

```bash
forgeplan init -y
forgeplan scan-import --path docs/architecture
forgeplan list
forgeplan health
```

Points the scanner at a non-standard directory. Useful when legacy docs live under `docs/`, `architecture/`, or `decisions/` instead of `.forgeplan/`.

### Example 4: Full index rebuild without reinit

```bash
rm -rf .forgeplan/lance
forgeplan scan-import
forgeplan health
```

Safe because markdown is the source of truth (ADR-003). The LanceDB folder is derived and always rebuildable. Do NOT `rm -rf .forgeplan/` — only the `lance/` subfolder.

## How it fits the workflow

`scan-import` is the bridge between raw markdown and the Forgeplan index. Typical placements:

- **Session start after clone**: `forgeplan init -y` → `forgeplan scan-import` → `forgeplan health`
- **Brownfield setup**: `forgeplan init -y --scan` is equivalent to running `init` and `scan-import` together
- **Recovery**: `rm -rf .forgeplan/lance && forgeplan scan-import` after any index corruption

Because markdown is authoritative (ADR-003), this command is idempotent and safe to run repeatedly. It will not duplicate artifacts — re-running reconciles the index with the filesystem.

## Safety notes

- **Only `.forgeplan/lance/` is safe to delete.** Never `rm -rf .forgeplan/` without an export first. The markdown subfolders (`adrs/`, `prds/`, etc.) are the source of truth and are not rebuildable from LanceDB.
- **`--dry-run` first on unfamiliar repos.** Scanning can surface files you didn't intend to import (`README.md`, `CHANGELOG.md`, etc.). Preview before committing.
- **Direct markdown edits work, but require scan-import to sync.** The recommended path is still `forgeplan update`, which keeps both layers in sync automatically.
- **Scan respects frontmatter.** Files without valid Forgeplan YAML frontmatter (kind, id, status) may be skipped or imported as generic Notes depending on detection heuristics.
- **No scoring history is recovered.** `scan-import` rebuilds identity and relations, but evidence decay and historical R_eff values come back only via [`forgeplan import`](/docs/cli/import/) from a JSON export.

## See also

- [`forgeplan init`](/docs/cli/init/) — create the workspace shell before scanning; `init --scan` combines both steps
- [`forgeplan import`](/docs/cli/import/) — restore full state including scoring (needs JSON, not markdown)
- [`forgeplan export`](/docs/cli/export/) — safety backup before any destructive operation
- [`forgeplan migrate`](/docs/cli/migrate/) — apply schema changes without touching content
- [`forgeplan health`](/docs/cli/health/) — verify the index matches expectations after scan
- [ADR-003](/docs/methodology/overview/) — why markdown is the source of truth
