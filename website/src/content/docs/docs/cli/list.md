---
title: forgeplan list
description: "List artifacts with filters — primary inventory command"
---

List artifacts in the current workspace, optionally filtered by kind, status,
tag, or creation date. This is the primary inventory command — use it to
answer "what do we have in this project?" before planning, routing, or
searching.

## When to use

- At session start, after `forgeplan health`, to scan what exists
- Before `forgeplan new` — avoid creating duplicates
- When drafting sprint scope ("all active PRDs tagged `auth`")
- In CI scripts to enumerate artifacts by status for reporting

## Not to use when

- You need full content → use [`forgeplan get`](/docs/cli/get/)
- You need fuzzy/semantic matching → use [`forgeplan search`](/docs/cli/search/)
- You want hierarchy (Epic → PRD → RFC) → use [`forgeplan tree`](/docs/cli/tree/)
- You want dependency order → use [`forgeplan order`](/docs/cli/order/)

## Usage

```text
forgeplan list [OPTIONS]
```

## Options

```text
  -t, --type <TYPE>      Filter by kind (prd, epic, spec, rfc, adr, etc.)
  -s, --status <STATUS>  Filter by status (draft, active, etc.)
      --tag <TAG>        Filter by tag. Supports "key=value" or bare "key" (matches any value). Examples: --tag source=code, --tag legacy
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```

## Examples

List everything (default view on a fresh workspace):

```bash
forgeplan list
```

Only active PRDs — current in-flight product scope:

```bash
forgeplan list --type prd --status active
```

Everything tagged `security`, as JSON for scripting:

```bash
forgeplan list --tag security --json | jq '.[] | .id'
```

## Output interpretation

Default output is a table with four columns:

| Column   | Meaning                                                   |
|----------|-----------------------------------------------------------|
| `ID`     | Stable artifact ID (e.g. `PRD-001`, `RFC-002`, `EVID-012`) |
| `KIND`   | Artifact type — prd, rfc, adr, epic, note, problem, evidence |
| `STATUS` | Lifecycle: draft / active / superseded / deprecated / stale |
| `TITLE`  | Human-readable title from frontmatter                     |

Rows are sorted by `created` descending (newest first). With `--json`, each
record includes full frontmatter plus `score` and `R_eff` where available —
useful piped into `jq` for dashboards.

## How it fits

`list` is the entry point of the read-only exploration loop:

```
list → get → tree → graph   (understand what exists and how it links)
```

For planning, chain with `order` and `blocked` to see dependency state. For
quality triage, pair with [`blindspots`](/docs/cli/blindspots/) to find
artifacts without evidence.

## See also

- [`forgeplan get`](/docs/cli/get/) — read full markdown of one artifact
- [`forgeplan tree`](/docs/cli/tree/) — hierarchy view
- [`forgeplan search`](/docs/cli/search/) — smart keyword + semantic search
- [`forgeplan health`](/docs/cli/health/) — project-level dashboard
- [Methodology guide](/docs/methodology/overview/)
