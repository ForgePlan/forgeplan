---
title: forgeplan update
description: "Update artifact metadata or body"
---

Update an artifact's metadata (title, status, depth) or replace its body content
without regenerating the file from a template. This is the low-level escape hatch
when you need to change a single field without rewriting the whole artifact.

For most lifecycle transitions, prefer the dedicated commands - they run
validation gates and enforce the state machine. Use `update` only when no
dedicated command exists or when you are scripting a bulk-edit.

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

## What it does

1. Loads the markdown file from `.forgeplan/<kind>s/<id>.md`.
2. Parses frontmatter and body.
3. Applies the supplied overrides (title, status, depth, body).
4. Rewrites the file with updated frontmatter.
5. Synchronizes derived state where supported. Migrated writes reconcile after canonical mutation and can leave publication pending.

Mutation helpers maintain their own timestamps; do not rely on a newly added `updated_at` frontmatter field as proof of publication.

## Examples

Rename an artifact:

```bash
forgeplan update PRD-001 --title "Authentication & SSO"
```

Change depth (e.g. after router re-evaluation):

```bash
forgeplan update PRD-001 --depth deep
```

Replace body from a file:

```bash
forgeplan update NOTE-042 --body @./draft.md
```

## Prefer lifecycle commands for status changes

Direct CLI status changes to `active`, `superseded` or `deprecated` are rejected.
Captured help lists those values but does not expose these handler refusals.
Use the dedicated lifecycle commands so their transition and evidence gates run:

| Goal                     | Command                                                |
|--------------------------|--------------------------------------------------------|
| draft → active           | [`forgeplan activate`](/docs/cli/activate/)            |
| active → superseded      | [`forgeplan supersede`](/docs/cli/supersede/)          |
| active → deprecated      | [`forgeplan deprecate`](/docs/cli/deprecate/)          |
| stale → active           | [`forgeplan renew`](/docs/cli/renew/)                  |
| stale → draft (new copy) | [`forgeplan reopen`](/docs/cli/reopen/)                |

## Canonical edits and index publication

Agents and scripts should use `forgeplan update`, including `--body @file`.
Human editor changes to canonical Markdown follow the project's own policy.
The derived index is then reconciled through `forgeplan index` or the appropriate
`forgeplan reindex` path, not `scan-import`. See [Constellation](/docs/reference/constellation/)
for captured versus published reads and migrated-layout limitations.

## Notes

- The `--body` flag replaces the entire body. There is no partial-edit mode.
- Use `@filepath` prefix to read body content from a file instead of passing it
  inline on the command line.
- The index is a cache, not the source of truth. If it is stale, preserve Markdown
  and config and use the appropriate `index`/`reindex` path.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan activate`](/docs/cli/activate/) - preferred way to change status
- [`forgeplan reindex`](/docs/cli/reindex/) - reconcile the derived index
- [`forgeplan validate`](/docs/cli/validate/) - check that the artifact is still valid
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan update --help`.

```text
Update artifact metadata or body

Usage: forgeplan update [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID

Options:
      --status <STATUS>  New status (draft, active, superseded, deprecated)
      --title <TITLE>    New title
      --depth <DEPTH>    New depth (tactical, standard, deep)
      --body <BODY>      New body content (use @filepath to read from file)
  -h, --help             Print help
  -V, --version          Print version
```
<!-- forgeplan-runtime:end -->
