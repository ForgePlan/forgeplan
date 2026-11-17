---
title: forgeplan init
description: "forgeplan init creates .forgeplan/, artifact directories, the LanceDB index, and default configuration. Use -y for unattended setup. For an existing checkout with intact artifact markdown, use forgeplan reindex to rebuild the local index instead."
---

`forgeplan init` creates `.forgeplan/`, artifact directories, the LanceDB index, and default configuration. Use `-y` for unattended setup. For an existing checkout with intact artifact markdown, use `forgeplan reindex` to rebuild the local index instead.

## Usage

```text
forgeplan init [OPTIONS]
```

## Options

```text
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

## When to use

Initialize a new project with `forgeplan init -y`. Add `--scan` to discover and import documents outside the workspace. `scan-import` imports previously unindexed foreign documents; it does not refresh existing artifacts.

## Existing workspaces and backups

`--force` refreshes configuration and indices and creates missing directories while preserving artifact bodies. It does not wipe `.forgeplan/`. Existing workspaces are backed up by default; `--no-backup` explicitly disables that backup. Back up configuration before replacing it. For schema upgrades use `migrate`; for rebuilding the index from markdown use `reindex`.

## Examples

```bash
forgeplan init -y
# In an existing Forgeplan checkout:
forgeplan reindex
forgeplan list
# Import documents when onboarding a new project:
forgeplan init -y --scan
```

## Model download and safety

Interactive initialization offers the BGE-M3 download and defaults to no. `init -y` does not download it unless you explicitly add `--with-model`. The model is about 2.1 GB, cached per machine; `setup` can prepare it separately. An export contains artifacts and relations, not the whole workspace or configuration. Keep a filesystem backup for local state. Do not delete `.forgeplan/` just to rebuild an index.

## See also

- [`forgeplan export`](/docs/cli/export/) - mandatory backup before any destructive operation
- [`forgeplan import`](/docs/cli/import/) - restore artifacts after reinit
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild LanceDB from tracked markdown
- [`forgeplan migrate`](/docs/cli/migrate/) - non-destructive schema upgrades
- [`forgeplan health`](/docs/cli/health/) - session start verification
- [Configuration](/docs/getting-started/configuration/) - LLM provider setup
