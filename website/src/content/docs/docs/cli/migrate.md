---
title: forgeplan migrate
description: "Open an existing workspace store and apply pending LanceDB schema migrations. This does not migrate slug filenames, assign display IDs, rebuild content from markdown, or repair arbitrary corruption."
---

Open an existing workspace store and apply pending LanceDB schema migrations. This does not migrate slug filenames, assign display IDs, rebuild content from markdown, or repair arbitrary corruption.

## Usage

```text
forgeplan migrate
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Upgrade workflow

```bash
forgeplan export --output ../pre-upgrade.json
cp -R .forgeplan ../pre-upgrade-workspace
# Upgrade the binary using your installation method, then:
forgeplan migrate
forgeplan health
```

## Choosing the right operation

Use `reindex` for a fresh checkout or stale index with intact markdown. Use `migrate-dry-run` to inspect slug migration collisions, `ci-assign-id` for ID assignment, and `reconcile-ids` for identity drift. Do not remove the workspace merely because the index needs rebuilding. Keep a full backup when configuration and local state matter; export alone is not a full filesystem backup.

## See also

- [`forgeplan export`](/docs/cli/export/) - mandatory safety backup before upgrading
- [`forgeplan import`](/docs/cli/import/) - restore exported artifacts and relations; not a database-schema rollback
- [`forgeplan init`](/docs/cli/init/) - initialize a new workspace, not repair an existing one
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild the derived index from markdown
- [`forgeplan health`](/docs/cli/health/) - post-migration verification
