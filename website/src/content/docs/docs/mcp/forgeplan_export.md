---
title: forgeplan_export
description: "Export all artifacts and relations to a JSON bundle. Returns the exported data directly for programmatic use, or writes it to a file path. Safe, read-only snapshot suitable for backup, migration, and cross-machine transfer."
---

Produces a complete JSON snapshot of the workspace: every artifact (PRD, RFC, ADR, Epic, Spec, Note, Problem, Solution, Evidence, Refresh), their frontmatter, their markdown bodies, and the full relation graph. The result can be written to a file or returned inline so the agent can embed it in a response, pipe it into migration tooling, or attach it to an audit trail.

**Category**: Workspace & Data

## When an agent calls this

- **Backup** before a risky operation such as `forgeplan init` re-initialisation, a schema upgrade, or bulk deletes — the #1 guardrail against losing artifacts.
- **Cross-machine transfer** — hand an encrypted JSON blob to another dev's machine and restore via `forgeplan_import`.
- **CI seeding** — snapshot a reference workspace once and load it into each pipeline run deterministically.
- **Audit / compliance** — freeze the state of decisions at a release boundary for traceability.
- **Debugging ForgePlan itself** — attach the export to a bug report so maintainers can reproduce the exact workspace state.

## Safety notes

- The export is **read-only**. It never mutates the workspace.
- **API keys and local config are NOT exported.** `.forgeplan/config.yaml` is gitignored and lives outside the artifact domain; the export contains only artifacts + relations. You still need to reconfigure LLM providers after `forgeplan_import`.
- Derived indexes (`.forgeplan/lance/`, `.fastembed_cache/`) are **not** included — they rebuild on demand via `forgeplan scan-import`.
- The export is the canonical backup format. Never `rm -rf .forgeplan` without calling `forgeplan_export` first.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `output` | `string` | no | Destination file path. If omitted, the tool returns the JSON bundle inline in the response. If provided, the tool writes to disk and returns a small success summary. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ExportParams`_

## Returns

**With `output`:**

```json
{
  "written": "/Users/me/forgeplan-backup-2026-04-11.json",
  "artifacts": 187,
  "relations": 312,
  "bytes": 648321
}
```

**Without `output`** (inline mode):

```json
{
  "version": 1,
  "generated_at": "2026-04-11T12:04:18Z",
  "artifacts": [
    {
      "id": "PRD-001",
      "kind": "prd",
      "status": "active",
      "depth": "standard",
      "title": "Auth system",
      "body": "# PRD-001: Auth system\n\n## Problem\n…",
      "tags": ["auth"],
      "valid_until": null,
      "updated_at": "2026-04-08T10:00:00Z"
    }
  ],
  "relations": [
    { "from": "EVID-012", "to": "PRD-001", "type": "informs" }
  ]
}
```

Inline mode is convenient for small workspaces; for large ones prefer writing to a file to avoid bloating MCP response payloads.

## Example invocation

Write to file (recommended for backups):

```json
{ "output": "/tmp/forgeplan-backup-2026-04-11.json" }
```

Return inline (for programmatic consumption):

```json
{}
```

## Typical sequence

```
forgeplan_export (output=backup.json)       ← safety snapshot
…perform risky operation…
forgeplan_import (data=<contents of backup.json>)   ← restore if things go wrong
```

For migration:

```
[machine A] forgeplan_export → bundle.json
[transfer bundle.json]
[machine B] forgeplan init → forgeplan_import (data=bundle.json)
```

## CLI equivalent

- [`forgeplan export`](/docs/cli/export/) — identical behaviour from the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_import`](/docs/mcp/forgeplan_import/) — restore from the bundle
- [`forgeplan_init`](/docs/mcp/forgeplan_init/) — initialise workspace before import
