---
title: forgeplan context
description: "Single-call reasoning context — artifact + graph + validation + scoring in one payload"
---

`forgeplan context` returns everything an AI agent needs to reason about an
artifact in a single call: the artifact body, its parent/child relationships,
validation results, R_eff score, and related memory. Instead of making five
round-trips (`get`, `validate`, `score`, `graph`, `memory`), the agent asks
once and receives a complete picture.

It is the primary data endpoint for the Forgeplan MCP server — every
`forgeplan_context` tool call goes through this command. CLI use is mostly
for debugging what the agent actually sees.

## When to use

- An AI agent (via MCP) needs full state of an artifact before editing or reasoning
- Debugging what context is passed to `forgeplan reason` or `forgeplan generate`
- Writing an external script that analyzes artifact health and relationships
- Auditing whether an artifact has enough linked evidence to be trustworthy
- Preparing input for a manual review — single output covers everything

## When NOT to use

- You only need the artifact body — use `forgeplan get <ID>` (lighter payload)
- You only need validation results — use `forgeplan validate <ID>`
- You only need the score — use `forgeplan score <ID>`
- You are rendering a human dashboard — use `forgeplan health` or `forgeplan status`

## Usage

```text
forgeplan context [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID (PRD, RFC, ADR, Epic, Problem, ...)
```

## Options

```text
      --json     Output as JSON for machine consumption (primary mode for AI agents)
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: Full context for a PRD

```bash
forgeplan context PRD-001
```

Prints the PRD body, validation verdict, R_eff score, upstream/downstream
links, and any related notes. Human-readable markdown by default.

### Example 2: JSON payload for an agent or script

```bash
forgeplan context PRD-001 --json
```

Emits structured JSON with top-level keys:

```json
{
  "artifact": { "id": "PRD-001", "kind": "prd", "status": "active", "body": "..." },
  "validation": { "verdict": "PASS", "must": [], "should": [] },
  "scoring": { "r_eff": 0.82, "evidence_count": 4 },
  "graph": {
    "parents": [{ "id": "EPIC-002", "relation": "child_of" }],
    "children": [{ "id": "RFC-018", "relation": "implements" }],
    "evidence": [{ "id": "EVID-042", "verdict": "supports" }]
  },
  "memory": [{ "key": "decision", "value": "..." }]
}
```

### Example 3: Debugging MCP agent behavior

```bash
forgeplan context PRD-019 --json | jq '.graph.evidence'
```

If an agent claims "no evidence linked" but you disagree, run this to see
exactly what the MCP layer is returning.

## Output interpretation

The JSON payload has five sections; each is optional if empty:

- **artifact** — frontmatter + markdown body exactly as stored on disk
- **validation** — `verdict` is `PASS` or `FAIL`; `must` and `should` list rule violations
- **scoring** — `r_eff` is the weakest-link evidence score (0.0-1.0);
  `evidence_count` is the number of linked EvidencePacks
- **graph** — `parents`, `children`, `evidence` arrays with IDs and relation type
- **memory** — decision-memory entries scoped to this artifact

Red flags:

- `r_eff: 0.0` and `evidence_count: 0` on an active artifact — blind spot, create evidence
- `validation.verdict: "FAIL"` on an active artifact — should never happen, shows ingestion drift
- Empty `graph.parents` on a PRD that claims to implement an Epic — missing link

## How it fits the workflow

```
Shape → Validate → Reason → Code → Evidence → Activate
                    ^                   ^
                context feeds here   context used in audits
```

- Called automatically by `forgeplan reason`, `forgeplan generate`, and MCP tools
- Use manually to preview what an agent will see before invoking reasoning
- Pair with `forgeplan audit` for bulk context sweeps across many artifacts

## See also

- [`forgeplan get`](/docs/cli/get/) — lighter subset (artifact only)
- [`forgeplan validate`](/docs/cli/validate/) — validation subset
- [`forgeplan score`](/docs/cli/score/) — scoring subset
- [`forgeplan reason`](/docs/cli/reason/) — consumes context for ADI analysis
- [`forgeplan health`](/docs/cli/health/) — project-wide dashboard
