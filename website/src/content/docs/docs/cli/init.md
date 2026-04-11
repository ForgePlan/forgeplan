---
title: forgeplan init
description: "Create a new .forgeplan/ workspace — LanceDB tables, config, and subdirectories"
---

`forgeplan init` bootstraps a new Forgeplan workspace in the current directory. It creates the `.forgeplan/` folder with all subdirectories for artifacts (adrs, prds, rfcs, epics, specs, evidence, problems, solutions, notes, refresh, memory), initializes the LanceDB index, and writes a default `config.yaml`. After this, you can start creating artifacts with `forgeplan new`.

## When to use

- Bootstrapping Forgeplan on a brand-new project for the first time.
- Fresh clone of an existing Forgeplan repo — markdown is tracked but `.forgeplan/lance/` is gitignored, so the index must be rebuilt locally.
- Recovery after catastrophic workspace corruption or a lost `.forgeplan/lance/` directory (pair with a fresh export backup).

## When NOT to use

- `.forgeplan/` already exists and is healthy — use [`forgeplan migrate`](/docs/cli/migrate/) for schema upgrades instead of reinitializing.
- Rebuilding only the LanceDB index from intact markdown — use [`forgeplan scan-import`](/docs/cli/scan-import/) (no destructive reinit needed).

## Usage

```text
forgeplan init [OPTIONS]
```

## Options

```text
      --force    Force reinitialize even if .forgeplan/ exists
  -y, --yes      Non-interactive mode (skip prompts, use defaults)
      --scan     Scan for existing documents and import them
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: First-time setup (AI agent)

```bash
forgeplan init -y
```

Creates `.forgeplan/` with defaults and no interactive prompts. AI agents must always use `-y` — the interactive wizard will block them otherwise.

### Example 2: Fresh clone of a Forgeplan repo

```bash
git clone <repo> && cd <repo>
forgeplan init -y
forgeplan scan-import
forgeplan list
```

Markdown under `.forgeplan/{adrs,prds,...}` is tracked, but `lance/`, `.fastembed_cache/`, and `config.yaml` are gitignored. `init -y` recreates the empty shell, then `scan-import` rebuilds the LanceDB index from tracked markdown.

### Example 3: Brownfield onboarding

```bash
forgeplan init -y --scan
```

Scans standard doc directories (`docs/`, `rfcs/`, etc.) and imports any markdown that looks like an artifact, classifying it by kind.

### Example 4: Reinitialize after backup

```bash
forgeplan export --output backup.json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
rm -rf .forgeplan
forgeplan init -y --force
forgeplan import backup.json
```

The only safe reinit path — export + filesystem copy + reinit + import.

## How it fits the workflow

This command belongs in the [full artifact lifecycle](/docs/guides/first-artifact/) — see the tutorial for the end-to-end flow. `init` is step zero; the next command is almost always `forgeplan health` or `forgeplan scan-import`.

## Safety notes

- **AI agents must always pass `-y`.** The interactive wizard will hang on stdin and look like a stuck process.
- **Never `rm -rf .forgeplan` without an export first.** See [`forgeplan export`](/docs/cli/export/) — it is the only backup path that captures links, evidence, and scoring state.
- **`config.yaml` is gitignored.** If you reinit, you will lose your LLM provider settings. Back it up separately: `cp .forgeplan/config.yaml ~/fp-config-backup.yaml`.
- **Markdown survives reinit only if you copy it out first.** `--force` wipes the folder. Always `cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)` as a secondary safety net.
- **Schema drift between versions** (e.g. v0.17 → v0.18 added columns) — prefer [`forgeplan migrate`](/docs/cli/migrate/) over reinit when possible.

## See also

- [`forgeplan export`](/docs/cli/export/) — mandatory backup before any destructive operation
- [`forgeplan import`](/docs/cli/import/) — restore artifacts after reinit
- [`forgeplan scan-import`](/docs/cli/scan-import/) — rebuild LanceDB from tracked markdown
- [`forgeplan migrate`](/docs/cli/migrate/) — non-destructive schema upgrades
- [`forgeplan health`](/docs/cli/health/) — session start verification
- [Configuration](/docs/getting-started/configuration/) — LLM provider setup
