---
title: forgeplan update
description: "Update artifact metadata or body"
---

Update an artifact's metadata (title, status, depth) or replace its body content
without regenerating the file from a template. This is the low-level escape hatch
when you need to change a single field without rewriting the whole artifact.

For most lifecycle transitions, prefer the dedicated commands — they run
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
5. Re-indexes the artifact in LanceDB so search and graph stay in sync.

The `updated_at` timestamp in frontmatter is refreshed automatically.

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

Force-set status (skipping the lifecycle state machine — use with care):

```bash
forgeplan update PRD-001 --status active
```

## Prefer lifecycle commands for status changes

Direct `--status` edits bypass the validation gate. For PRD / RFC / ADR / Epic /
Spec, this can create "active" artifacts that fail `forgeplan validate`. Use the
dedicated commands instead:

| Goal                     | Command                                                |
|--------------------------|--------------------------------------------------------|
| draft → active           | [`forgeplan activate`](/docs/cli/activate/)            |
| active → superseded      | [`forgeplan supersede`](/docs/cli/supersede/)          |
| active → deprecated      | [`forgeplan deprecate`](/docs/cli/deprecate/)          |
| stale → active           | [`forgeplan renew`](/docs/cli/renew/)                  |
| stale → draft (new copy) | [`forgeplan reopen`](/docs/cli/reopen/)                |

## Direct markdown edits are often faster

Because the markdown files are the source of truth (ADR-003), editing
`.forgeplan/<kind>s/<id>.md` in your editor and then running
[`forgeplan scan-import`](/docs/cli/scan-import/) is often the most ergonomic
workflow — especially for body edits, structured field updates, or batch
changes. `forgeplan update` shines when you are scripting or want to avoid
opening the file at all.

## Notes

- The `--body` flag replaces the entire body. There is no partial-edit mode.
- Use `@filepath` prefix to read body content from a file instead of passing it
  inline on the command line.
- After any direct edit (via CLI or editor), the LanceDB index is the cache, not
  the truth. If you suspect drift, run `forgeplan scan-import` to rebuild.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan activate`](/docs/cli/activate/) — preferred way to change status
- [`forgeplan scan-import`](/docs/cli/scan-import/) — rebuild index after direct edits
- [`forgeplan validate`](/docs/cli/validate/) — check that the artifact is still valid
- [Methodology guide](/docs/methodology/overview/)
