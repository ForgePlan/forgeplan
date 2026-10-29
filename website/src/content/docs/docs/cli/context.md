---
title: forgeplan context
description: "Single-call reasoning context - artifact + graph + validation + scoring in one payload"
---

`forgeplan context` combines artifact metadata, relationships, validation counts,
R_eff, F-G-R and suggestions. The CLI payload does **not** include the artifact
body or related memory; use `forgeplan get <ID> --json` for the body.

The captured MCP catalog for this build exposes no `forgeplan_context` tool, so
this CLI projection is not an MCP response. MCP-side reads use the tools listed in
[MCP overview](/docs/mcp/) (`forgeplan_get`, `forgeplan_search`, the Constellation
tools).

## When to use

- Inspecting validation, scoring and relationships of one artifact before editing or reasoning
- Writing an external script that analyzes artifact health and relationships
- Auditing whether an artifact has enough linked evidence to be trustworthy
- Preparing metadata, validation and relationship inputs for a manual review; read the body separately.

## When NOT to use

- You only need the artifact body - use `forgeplan get <ID>` (lighter payload)
- You only need validation results - use `forgeplan validate <ID>`
- You only need the score - use `forgeplan score <ID>`
- You are rendering a human dashboard - use `forgeplan health` or `forgeplan status`

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

Prints metadata, validation status, R_eff, F-G-R, graph links and suggestions.
The human-readable output does not print the artifact body.

### Example 2: JSON payload for an agent or script

```bash
forgeplan context PRD-001 --json
```

Illustrative JSON excerpt showing selected fields, not a captured result:

```json
{
  "artifact": { "id": "PRD-001", "kind": "prd", "status": "active", "r_eff": 0.82 },
  "graph": {
    "parent": "EPIC-002",
    "evidence": ["EVID-042"],
    "depends_on": [], "dependents": [], "related": []
  },
  "validation": { "passed": true, "must_errors": 0, "should_warnings": 0 },
  "fgr": { "formality": 1.0, "granularity": 0.8, "reliability": 0.7, "grade": "A" }
}
```

### Example 3: Debugging MCP agent behavior

```bash
forgeplan context PRD-019 --json | jq '.graph.evidence'
```

This CLI projection shows evidence IDs in its graph. Inspect the MCP tool's own
response when investigating what an MCP agent received.

## Output interpretation

The CLI JSON contains:

- **artifact** — metadata including `r_eff`, title, depth and timestamps, not the body
- **derived_status** — computed status label
- **graph** — `parent` plus arrays of evidence, dependency, dependent and related IDs
- **validation** — `passed`, `must_errors` and `should_warnings`
- **fgr** — `formality`, `granularity`, `reliability` and `grade`
- **suggestions**, **hints**, **_next_action** — guidance for the next step

Inspect zero R_eff, failed validation or a missing expected parent separately.
An active artifact can fail later validation after its content changes; that
observation alone does not establish the cause.

## How it fits the workflow

```
Shape → Validate → Reason → Code → Evidence → Activate
                    ^                   ^
                optional manual context review
```

- Use manually to inspect this CLI projection before preparing a reasoning request.
- Read the artifact body separately with `get` when the task requires it.
- For multiple artifacts, call `context` for each selected ID; there is no `forgeplan audit` CLI command.

## See also

- [`forgeplan get`](/docs/cli/get/) - lighter subset (artifact only)
- [`forgeplan validate`](/docs/cli/validate/) - validation subset
- [`forgeplan score`](/docs/cli/score/) - scoring subset
- [`forgeplan reason`](/docs/cli/reason/) - ADI analysis with its own input preparation
- [`forgeplan health`](/docs/cli/health/) - project-wide dashboard

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan context --help`.

```text
Single-call reasoning context — artifact + graph + validation + scoring

Usage: forgeplan context [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID

Options:
      --json     Output as JSON for machine consumption (primary mode for AI agents)
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
