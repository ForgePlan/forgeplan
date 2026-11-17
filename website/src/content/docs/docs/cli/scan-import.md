---
title: forgeplan scan-import
description: "Import existing external Markdown documents as Forgeplan artifacts for brownfield onboarding. This is not the index-rebuild command: default project discovery excludes .forgeplan/, and existing artifact bodies are skipped rather than refreshed. Use reindex for a fresh checkout or edits to workspace Markdown."
---

Import existing external Markdown documents as Forgeplan artifacts for brownfield onboarding. This is not the index-rebuild command: default project discovery excludes `.forgeplan/`, and existing artifact bodies are skipped rather than refreshed. Use `reindex` for a fresh checkout or edits to workspace Markdown.

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

## Discovery and preview

The default scan reads standard documentation directories and selected root-level Markdown files, not every directory in the repository. `--path` changes the scan root and must resolve inside the project; the same standard-directory/root-file discovery rules still apply. Symlinks, oversized files, and excluded directories are skipped. Detection uses frontmatter, filename, or content; undetected files are reported unknown, not guaranteed to become Notes.

## Examples

```bash
# In a workspace initialized for an existing project:
forgeplan scan-import --dry-run
forgeplan scan-import --path docs/architecture --dry-run
# Review the detected sources before a real import:
forgeplan scan-import --path docs/architecture
forgeplan list
forgeplan health
# Different task: refresh tracked .forgeplan Markdown after a clone/edit:
forgeplan reindex
```

## Existing artifacts and projections

A real import creates a workspace Markdown projection and a LanceDB record, preserving supported source frontmatter such as author, tags, and links. When an ID already exists, the record is reported skipped and its body is not updated. A missing projection can be repaired on retry; that exception is not a general refresh of existing artifacts. Repeated import therefore does not reconcile manual body edits. `init --scan` runs this onboarding import after initialization.

## Limits of dry-run and success

`--dry-run` previews detected entries without writing imported artifacts; it returns before the existing-ID skip check, so preview counts are not a promise that all entries will actually be imported. Read the real run’s imported/skipped/unknown/failed counts and warnings: per-file failures can be reported without a failing process exit. Import does not prove completeness or evidence quality. Validate the chosen artifacts and inspect health afterward.

## Recovery is reindex, not deletion

For `.forgeplan/` Markdown already tracked in the repository, run `forgeplan reindex`; do not delete the index and then run scan-import. Neither scanner nor an export/import pair recreates full change history. Use `import` only for a supported export bundle and keep separate backups of configuration and other local state.

## See also

- [`forgeplan init`](/docs/cli/init/) - create the workspace shell before scanning; `init --scan` combines both steps
- [`forgeplan import`](/docs/cli/import/) - read a supported export bundle, not full workspace history
- [`forgeplan export`](/docs/cli/export/) - safety backup before any destructive operation
- [`forgeplan migrate`](/docs/cli/migrate/) - apply schema changes without touching content
- [`forgeplan health`](/docs/cli/health/) - verify the index matches expectations after scan
- [ADR-003](/docs/methodology/overview/) - why markdown is the source of truth
- [`forgeplan reindex`](/docs/cli/reindex/) — recover the index from workspace Markdown
