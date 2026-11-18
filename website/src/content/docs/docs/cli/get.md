---
title: forgeplan get
description: "Read full artifact content by ID - AI-friendly markdown fetch"
---

Read a full artifact by ID or resolvable slug. On a migrated workspace, the selected build uses a captured canonical-Markdown view and reports whether it is persisted. On a legacy workspace it reads the projected store. Canonical Markdown, its derived index, and federation snapshots are distinct sources.

## When to use

- An AI agent needs full context of a PRD / RFC / ADR before coding
- You want to review a specific decision in the terminal without opening the file
- You want JSON output for piping into other Forgeplan tools

## Not to use when

- You want a list of candidates → use [`forgeplan list`](/docs/cli/list/)
- You do not know the exact ID → use [`forgeplan search`](/docs/cli/search/)
- You want linked artifacts too → use [`forgeplan graph`](/docs/cli/graph/) or
  [`forgeplan tree`](/docs/cli/tree/)

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

Read a PRD in full - the standard AI-agent fetch:

```bash
forgeplan get PRD-001
```

Read an RFC and pipe through a pager:

```bash
forgeplan get RFC-002 | less
```

Retrieve the complete `body` field without assuming a `frontmatter` JSON object:

```bash
forgeplan get EVID-012 --json | jq -r '.body'
```

## Output interpretation

Human-readable output includes a metadata header, links, the body, and hints; it is not a raw-file-only stream. On the migrated CLI path, JSON `body` contains the entire captured Markdown, including frontmatter. The full-profile `forgeplan_constellation_get` has a different, parsed-body contract.

With `--json`, metadata such as `title`, `depth`, `author`, `tags`, timestamps and `r_eff` is flattened at the top level, alongside `body`, `links.outbound`, `links.inbound`, and `_next_action`. There is no nested `frontmatter` object. The migrated branch also adds `index`, `generation_id`, `snapshot_id`, `source_set_digest`, and `persisted`. `persisted: false` means the fresh captured view is not the published generation; reading it does not publish a new generation.

An unresolved ID fails with an `Artifact '<ID>' not found` error and a `forgeplan list` hint.

## How it fits

`get` is the "detail view" to `list`'s "index view":

```
list (find it) → get (read it) → validate / reason / link / score (act on it)
```

The artifact-reading MCP tool is [`forgeplan_get`](/docs/mcp/forgeplan_get/). It is not named `read_artifact`; profile-specific pinned retrieval is a separate interface.

## See also

- [`forgeplan list`](/docs/cli/list/) - discover IDs
- [`forgeplan search`](/docs/cli/search/) - find by query
- [`forgeplan validate`](/docs/cli/validate/) - quality check
- [`forgeplan score`](/docs/cli/score/) - R_eff + F-G-R metrics
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan get --help`.

```text
Read a full artifact by ID

Usage: forgeplan get [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
