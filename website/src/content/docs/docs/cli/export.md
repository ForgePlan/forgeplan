---
title: forgeplan export
description: "Dump all artifacts, links, and evidence to a JSON file - mandatory backup before any destructive operation"
---

`forgeplan export` writes artifact records and typed relations from the selected workspace into a JSON bundle with format version `1`. It is not a whole-workspace backup: configuration, secrets, logs, claims, session files, model caches, and published index generations are not included. Preserve canonical files and the required private filesystem backup rather than recreating retired tables.

## When to use

- **Always before `rm -rf .forgeplan`, `forgeplan init --force`, or any other destructive workspace operation.** No exceptions.
- Before upgrading the Forgeplan binary, as a safety net in case `forgeplan migrate` fails.
- Periodically as a disaster-recovery backup, especially before major methodology restructurings.
- When moving artifact records and links between workspaces; preserve other files separately.
- For debugging - sharing a workspace snapshot with teammates or attaching it to an issue.

## When NOT to use

- As a daily git commit target - the tracked markdown under `.forgeplan/{adrs,prds,rfcs,...}` is already the canonical source of truth (ADR-003). Export is for LanceDB-resident state.
- For publishing or sharing with non-Forgeplan tools - the JSON schema is internal and unstable across versions.
- As a substitute for git - it is not a version history tool, just a point-in-time snapshot.

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

## Examples

### Example 1: Default backup

```bash
forgeplan export
```

Writes to `.forgeplan/export.json`. Convenient but lives inside the very folder you might be about to delete - prefer an explicit external path for real backups.

### Example 2: Pre-reinit safety backup

```bash
backup="$HOME/backups/forgeplan-$(date +%Y%m%d-%H%M).json"
forgeplan export --output "$backup"
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
# Keep the original workspace; try the JSON import in a separate empty directory.
mkdir forgeplan-restore-demo && cd forgeplan-restore-demo
forgeplan init -y
forgeplan import "$backup"
```

This keeps the original workspace and a filesystem copy while demonstrating JSON import separately. Check the imported records and links; this is not a verified full-state recovery procedure.

### Example 3: Pre-upgrade snapshot

```bash
forgeplan export --output pre-upgrade.json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
# Upgrade to the intended release using its documented installation method.
forgeplan migrate
forgeplan health
# If a problem appears, preserve both snapshots and investigate before restoring.
```

Back up before upgrading. Do not assume a selected Constellation build and the installed release use the same index layout.

### Example 4: Share workspace state with a teammate

```bash
forgeplan export --output snapshot.json
# Review snapshot.json for private content before sharing through an approved channel.
```

The bundle can share artifact records and links, but cannot by itself reproduce all health, configuration, freshness, or runtime state.

## How it fits the workflow

`export` is a safety operation, not part of the artifact lifecycle. It should be muscle memory before any of the following:

- `rm -rf .forgeplan`
- `forgeplan init --force`
- `forgeplan migrate` on a meaningful workspace
- Upgrading the binary across a major version
- Experimental bulk edits to markdown

The rule is simple: **if you are about to touch the workspace in a way you can't undo with `git`, export first.**

## Safety notes

- **Default path lives inside `.forgeplan/`.** `forgeplan export` with no arguments writes to `.forgeplan/export.json`, which is lost if you then `rm -rf .forgeplan`. Always pass `-o` with a path outside the workspace for real backups.
- **Exports are not encrypted.** They may contain LLM prompts, internal notes, and other sensitive content. Don't commit them to public repos.
- **Keep the filesystem backup.** Export plus a copy preserves more than JSON alone; neither example establishes a tested disaster-recovery result.
- **Configuration and secrets are separate.** Preserve `.forgeplan/config.yaml` and `.forgeplan/secrets.env` privately when present; do not publish credentials.
- **JSON format has its own version.** The current exporter writes `"version": 1`; that is not the binary release number.

## See also

- [`forgeplan import`](/docs/cli/import/) - the restore counterpart
- [`forgeplan init`](/docs/cli/init/) - initialize a workspace; `--force` rewrites configuration but preserves existing artifact bodies
- [`forgeplan migrate`](/docs/cli/migrate/) - non-destructive schema upgrade (still back up first)
- [`forgeplan scan-import`](/docs/cli/scan-import/) - import new external Markdown, not rebuild the canonical index
- [`forgeplan health`](/docs/cli/health/) - verify the workspace before and after backup cycles

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan export --help`.

```text
Export all artifacts to JSON file

Usage: forgeplan export [OPTIONS]

Options:
  -o, --output <OUTPUT>  Output file path (default: .forgeplan/export.json)
  -h, --help             Print help
  -V, --version          Print version
```
<!-- forgeplan-runtime:end -->
