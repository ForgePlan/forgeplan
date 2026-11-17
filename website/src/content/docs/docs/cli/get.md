---
title: forgeplan get
description: "Read an artifact by slug or display ID. Text output presents metadata, body, and links; it is not a byte-for-byte dump of the original markdown file. JSON provides the stored fields for scripts."
---

Read an artifact by slug or display ID. Text output presents metadata, body, and links; it is not a byte-for-byte dump of the original markdown file. JSON provides the stored fields for scripts.

## Usage

```text
forgeplan get [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
forgeplan get PRD-001
forgeplan get RFC-002 | less
# Structured evidence fields are in the body, not a frontmatter object:
forgeplan get EVID-012 --json | jq -r '.body'
```

## JSON contract

Top-level keys include `id`, `slug`, `kind`, `status`, `title`, `depth`, `author`, `parent_epic`, `valid_until`, `r_eff`, `created_at`, `updated_at`, `body`, `links`, and `_next_action`. There is no nested `frontmatter` object. Use `list` or `search` to discover an ID, then `get` to inspect it. The corresponding MCP tool is `forgeplan_get`.

`links.outbound` is an array of `{target, relation}` objects; `links.inbound` contains `{source, relation}`. Both arrays are present even when empty. Nullable metadata remains `null`; `slug` comes from stored body frontmatter. Requires an initialized workspace and valid configuration. A missing ID exits through the ordinary CLI error path with a `Fix: forgeplan list` hint; `--json` does not turn that failure into the successful artifact schema.

## See also

- [`forgeplan list`](/docs/cli/list/) - discover IDs
- [`forgeplan search`](/docs/cli/search/) - find by query
- [`forgeplan validate`](/docs/cli/validate/) - quality check
- [`forgeplan score`](/docs/cli/score/) - R_eff + F-G-R metrics
- [Methodology guide](/docs/methodology/overview/)
