---
title: forgeplan get
description: "Read full artifact content by ID — AI-friendly markdown fetch"
---

Read the full markdown content of an artifact by ID. This is how AI agents
and humans pull a single decision document into context. The command reads
from the LanceDB-projected view, which stays in sync with the source markdown
in `.forgeplan/`.

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

Read a PRD in full — the standard AI-agent fetch:

```bash
forgeplan get PRD-001
```

Read an RFC and pipe through a pager:

```bash
forgeplan get RFC-002 | less
```

JSON output — body plus all frontmatter, ready for `jq`:

```bash
forgeplan get EVID-012 --json | jq '.frontmatter.verdict, .frontmatter.congruence_level'
```

## Output interpretation

Default output is the raw markdown file: YAML frontmatter fenced by `---`,
followed by the body sections (Problem, Goals, FR, etc.). This is the same
text a human would see opening the file in an editor.

With `--json`, the envelope is:

```json
{
  "id": "PRD-001",
  "kind": "prd",
  "status": "active",
  "frontmatter": { "title": "...", "tags": [...], "created": "...", ... },
  "body": "## Problem\n..."
}
```

If the ID does not exist, the command exits with status 1 and prints
`Error: artifact not found: <ID>`.

## How it fits

`get` is the "detail view" to `list`'s "index view":

```
list (find it) → get (read it) → validate / reason / link / score (act on it)
```

For AI agents using MCP, `get` maps 1:1 to the `read_artifact` tool.

## See also

- [`forgeplan list`](/docs/cli/list/) — discover IDs
- [`forgeplan search`](/docs/cli/search/) — find by query
- [`forgeplan validate`](/docs/cli/validate/) — quality check
- [`forgeplan score`](/docs/cli/score/) — R_eff + F-G-R metrics
- [Methodology guide](/docs/methodology/overview/)
