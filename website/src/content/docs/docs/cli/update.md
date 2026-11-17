---
title: forgeplan update
description: "Update artifact metadata or replace the complete body without recreating the artifact from a template. The command writes the markdown projection and synchronizes the LanceDB index."
---

Update artifact metadata or replace the complete body without recreating the artifact from a template. The command writes the markdown projection and synchronizes the LanceDB index.

## Usage

```text
forgeplan update [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID
```

## Options

```text
      --status <STATUS>  New status (draft, active, superseded, deprecated)
      --title <TITLE>    New title
      --depth <DEPTH>    New depth (tactical, standard, deep)
      --body <BODY>      New body content (use @filepath to read from file)
  -h, --help             Print help
  -V, --version          Print version
```

## Lifecycle restrictions

`update --status` rejects `active`, `superseded`, and `deprecated`, despite those values appearing in 0.38.0 help. Use `activate`, `supersede`, or `deprecate`; use `renew` or `reopen` for stale decisions. The remaining status writes do not run the lifecycle transition validator in 0.38.0: this is a limitation, not permission to resurrect terminal artifacts or invent status values. `--depth critical` is accepted by the runtime even though the help omits it.

## Examples

```bash
forgeplan update PRD-001 --title "Authentication and SSO"
forgeplan update PRD-001 --depth deep
forgeplan update NOTE-042 --body @./draft.md
forgeplan activate PRD-001
```

## Body and direct markdown edits

`--body` replaces the entire body, not a section. `@filepath` reads a file; its YAML frontmatter is stripped rather than copied into the body. Review the content before replacing an existing document. After direct edits to authoritative markdown, run `reindex`. `scan-import` is for new external documents, not refreshing the index.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan activate`](/docs/cli/activate/) - preferred way to change status
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild index after direct edits
- [`forgeplan validate`](/docs/cli/validate/) - check that the artifact is still valid
- [Methodology guide](/docs/methodology/overview/)
