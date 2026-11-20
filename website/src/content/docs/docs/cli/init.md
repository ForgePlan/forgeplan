---
title: forgeplan init
description: "Create a new .forgeplan/ workspace - LanceDB tables, config, and subdirectories"
---

`forgeplan init` bootstraps a new Forgeplan workspace in the current directory. It creates the `.forgeplan/` folder with all subdirectories for artifacts (adrs, prds, rfcs, epics, specs, evidence, problems, solutions, notes, refresh, memory), initializes the LanceDB index, and writes a default `config.yaml`. After this, you can start creating artifacts with `forgeplan new`.

## When to use

- Bootstrapping Forgeplan on a brand-new project for the first time.
- Fresh clone of an existing Forgeplan repo - markdown is tracked but `.forgeplan/lance/` is gitignored, so the index must be rebuilt locally.
- Refreshing local configuration deliberately; this is not an index-recovery command.

## When NOT to use

- `.forgeplan/` is healthy - do not reinitialize just to update the index. Legacy schema upgrades are described in [`migrate`](/docs/cli/migrate/); Constellation uses a separate layout.
- Rebuilding the derived index from intact Markdown - use [`forgeplan reindex`](/docs/cli/reindex/), or the Constellation indexing flow described in the [guide](/docs/guides/constellation/).

## Usage

```text
forgeplan init [OPTIONS]
```

## Options

```text
      --force       Refresh config and indices; preserve existing artifact Markdown
  -y, --yes         Non-interactive mode (skip prompts, use defaults)
      --scan        Scan for existing documents and import them
      --no-backup   Skip the automatic --force backup (not recommended for recovery)
      --with-model  Explicitly download the embedding model
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: First-time setup (AI agent)

```bash
forgeplan init -y
```

Creates `.forgeplan/` with defaults and no interactive prompts. AI agents must always use `-y` - the interactive wizard will block them otherwise.

### Example 2: Fresh clone of a Forgeplan repo

```bash
git clone <repo> && cd <repo>
forgeplan init -y
forgeplan reindex
forgeplan list
```

If `.forgeplan/` already exists, `init -y` reports it and returns without
recreating missing configuration. This includes an ancestor workspace:
discovery walks up from the current directory. Check the reported path.
Use `reindex` to rebuild from tracked Markdown. If you deliberately need
fresh config defaults, run `init -y --force` from the intended project root,
then restore custom settings from the backup; see below.

### Example 3: Brownfield onboarding

```bash
forgeplan init -y --scan
```

Scans standard doc directories (`docs/`, `rfcs/`, etc.) and imports any markdown that looks like an artifact, classifying it by kind.

### Example 4: Reinitialize after backup

```bash
forgeplan init -y --force
# Review the reported .forgeplan-backup-<timestamp>/ directory.
# Merge any required custom settings back into .forgeplan/config.yaml.
forgeplan reindex
```

`--force` preserves existing artifact `.md` bodies and creates a backup by
default. The active `config.yaml` is regenerated with defaults; custom LLM,
embedding, and other settings remain only in the backup until you restore
them. Do not delete `.forgeplan/` or import the same artifacts again merely
to rebuild the index.

`init -y` does not download a model. For a new workspace only, an intentional
scripted download can use `init -y --with-model`. An existing-workspace early
return can bypass that step; use `forgeplan setup` to fetch deliberately.

## How it fits the workflow

`init` is the bootstrap step. See [Configuration](/docs/getting-started/configuration/)
for local settings and [Constellation](/docs/guides/constellation/) for the
local build's migrated-workspace workflow.

## Safety notes

- **AI agents must always pass `-y`.** The interactive wizard will hang on stdin and look like a stuck process.
- **Do not delete `.forgeplan/` to diagnose an index problem.** Preserve Markdown and configuration; use the indexing command for your layout.
- **`config.yaml` is local.** Repeated `init -y` is a no-op, while `--force` activates defaults, not your previous custom settings.
- **Markdown is preserved by `--force`.** Keep the automatic backup; it is not necessary to copy sources out and import them again.
- **Parent discovery matters.** Run initialization from the intended root and check the reported workspace before continuing.

## See also

- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild the derived index
- [`forgeplan export`](/docs/cli/export/) - portable artifact export
- [`forgeplan scan-import`](/docs/cli/scan-import/) - import external documents
- [Constellation reference](/docs/reference/constellation/) - layout and read contracts
- [Configuration](/docs/getting-started/configuration/) - LLM provider setup

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan init --help`.

```text
Initialize a new .forgeplan/ workspace

Usage: forgeplan init [OPTIONS]

Options:
      --force
          Reinitialize config + indices in an existing .forgeplan/. PROB-068: --force is now strictly additive — it never overwrites existing artifact .md bodies. Only `config.yaml` is rewritten and missing directories / the LanceDB index are recreated. An auto-backup is taken unless `--no-backup` is also passed

  -y, --yes
          Non-interactive mode (skip prompts, use defaults)

      --scan
          Scan for existing documents and import them

      --no-backup
          Skip the safety auto-backup that `--force` would otherwise create (`.forgeplan-backup-<timestamp>/`). PROB-068: backups are on by default to protect against the historical data loss vector; opt out only when you've already exported

      --with-model
          Download the embedding model without asking.
          
          Interactive runs are asked (default no); `--yes` never downloads, so agents and CI cannot pull ~2.1 GB by accident. Use this when a scripted install genuinely wants semantic search ready up front.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
