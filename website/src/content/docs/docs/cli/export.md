---
title: forgeplan export
description: "Dump all artifacts, links, and evidence to a JSON file — mandatory backup before any destructive operation"
---

`forgeplan export` serializes the entire workspace — artifacts, links, evidence records, scoring state, and metadata — into a single JSON file. It is the mandatory backup step before any destructive operation on `.forgeplan/` and the only way to produce a restorable snapshot that captures state beyond the tracked markdown.

## When to use

- **Always before `rm -rf .forgeplan`, `forgeplan init --force`, or any other destructive workspace operation.** No exceptions.
- Before upgrading the Forgeplan binary, as a safety net in case `forgeplan migrate` fails.
- Periodically as a disaster-recovery backup, especially before major methodology restructurings.
- When moving a workspace between machines and you need full state (scoring, evidence decay) rather than just markdown.
- For debugging — sharing a workspace snapshot with teammates or attaching it to an issue.

## When NOT to use

- As a daily git commit target — the tracked markdown under `.forgeplan/{adrs,prds,rfcs,...}` is already the canonical source of truth (ADR-003). Export is for LanceDB-resident state.
- For publishing or sharing with non-Forgeplan tools — the JSON schema is internal and unstable across versions.
- As a substitute for git — it is not a version history tool, just a point-in-time snapshot.

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

Writes to `.forgeplan/export.json`. Convenient but lives inside the very folder you might be about to delete — prefer an explicit external path for real backups.

### Example 2: Pre-reinit safety backup

```bash
forgeplan export --output ~/backups/forgeplan-$(date +%Y%m%d-%H%M).json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
rm -rf .forgeplan
forgeplan init -y
forgeplan import ~/backups/forgeplan-*.json
```

The full "safe reinit" ritual. The export lands outside `.forgeplan/`, and a filesystem copy of the whole folder provides a secondary safety net.

### Example 3: Pre-upgrade snapshot

```bash
forgeplan export --output pre-v0.18-upgrade.json
cargo install forgeplan
forgeplan migrate
forgeplan health
# if anything looks wrong:
rm -rf .forgeplan && forgeplan init -y
forgeplan import pre-v0.18-upgrade.json
```

Cheap insurance before a version bump.

### Example 4: Share workspace state with a teammate

```bash
forgeplan export --output snapshot.json
gh gist create snapshot.json   # or attach to an issue
```

Gives a teammate everything they need to reproduce scoring, health output, and decay state — markdown alone can't do this.

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
- **Export + filesystem copy is the gold standard.** `forgeplan export -o backup.json && cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)` gives you both a structured restore path and a raw bit-for-bit snapshot.
- **`config.yaml` is NOT in the export.** LLM API key settings live in `.forgeplan/config.yaml` and must be backed up separately.
- **Exports are versioned to the binary.** A backup made under v0.17 may require running `forgeplan migrate` after importing into v0.18.

## See also

- [`forgeplan import`](/docs/cli/import/) — the restore counterpart
- [`forgeplan init`](/docs/cli/init/) — the destructive step export usually precedes
- [`forgeplan migrate`](/docs/cli/migrate/) — non-destructive schema upgrade (still back up first)
- [`forgeplan scan-import`](/docs/cli/scan-import/) — rebuild index from tracked markdown
- [`forgeplan health`](/docs/cli/health/) — verify the workspace before and after backup cycles
