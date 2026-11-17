---
title: forgeplan context
description: "Read a combined CLI summary of an artifact, its relationships, validation counts, F-G-R quality, and computed R_eff. Use get separately for the complete body. Forgeplan 0.38.0 has no forgeplan_context MCP tool."
---

Read a combined CLI summary of an artifact, its relationships, validation counts, F-G-R quality, and computed R_eff. Use `get` separately for the complete body. Forgeplan 0.38.0 has no `forgeplan_context` MCP tool.

## Usage

```text
forgeplan context [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID
```

## Options

```text
      --json     Output as JSON for machine consumption (primary mode for AI agents)
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
forgeplan context PRD-001
forgeplan context PRD-001 --json | jq '.artifact.r_eff, .validation, .graph.evidence'
```

## JSON contract

Keys are `artifact` (id, kind, status, title, depth, r_eff, created_at, updated_at), `derived_status`, `graph` (parent, evidence, depends_on, dependents, related), `validation` (passed, must_errors, should_warnings), `fgr` (formality, granularity, reliability, grade), `suggestions`, `_next_action`, and `hints`. There is no artifact body, `scoring` object, or `memory` array.

## Using the result

Inspect `validation.passed` and error counts, follow evidence IDs with `get`, and check parent or dependency links. Use `health` for the workspace and `score --json` for evidence contributions. This CLI summary does not establish what another tool or agent actually read.

## See also

- [`forgeplan get`](/docs/cli/get/) - lighter subset (artifact only)
- [`forgeplan validate`](/docs/cli/validate/) - validation subset
- [`forgeplan score`](/docs/cli/score/) - scoring subset
- [`forgeplan reason`](/docs/cli/reason/) - consumes context for ADI analysis
- [`forgeplan health`](/docs/cli/health/) - project-wide dashboard
