---
title: forgeplan_ingest
description: "Plan an ingest run (mapping YAML × source file). Wave 3: dry-run-only - returns drafts without writing artifacts. Set `dry_run: false` to indicate intent to write, but the MCP surface still defers actual writes to the `forgeplan ingest` CLI (Wave 4 will wire artifact::Store). PRD-066 AC-1."
---

Plan an ingest run (mapping YAML × source file). Wave 3: dry-run-only - returns drafts without writing artifacts. Set `dry_run: false` to indicate intent to write, but the MCP surface still defers actual writes to the `forgeplan ingest` CLI (Wave 4 will wire artifact::Store). PRD-066 AC-1.

**Category**: Other

This tool is a **planner, not an executor**. It validates that the two paths exist, that they sit inside the workspace, and that they are within the size budget the server is willing to touch. It then hands the agent the exact `forgeplan ingest` command line that would perform the run. Nothing is written to `.forgeplan/`, not even with `dry_run: false`.

## What it does

1. **Canonicalise the mapping path.** The file must exist, must be a regular file, and must resolve under an allowed root: the workspace root (the parent of `.forgeplan/`) or the `~/.claude/plugins/` tree where plugin packs install their bundles. Symlink escapes are resolved before the check, so a benign-looking relative path cannot point outside.
2. **Canonicalise the source path** with the same rules.
3. **Check the size budget.** The mapping is refused above 1 MiB, the source above 10 MiB. The check reads file metadata only - the server never loads either file into memory here.
4. **Echo the plan.** The response repeats both paths (redacted to workspace-relative where possible), both flags, and `wave3_status: "planned"` with `drafts`, `skipped` and `errors` as empty lists.
5. **Hint the real command.** `_next_action` carries a ready-to-paste `forgeplan ingest --mapping ... --source ...` line, with `--dry-run` and/or `--update` appended to match the flags you passed.

Because step 3 is the only thing that touches the files, a mapping that violates the SPEC-004 schema **passes** MCP validation and is rejected only by the CLI:

```text
# MCP: accepts it and returns a plan
{ "wave3_status": "planned", "mapping": "bad.yaml", ... }

# CLI: refuses to run it
Error: invalid mapping YAML: source_kind: unknown variant `markdown`, expected one of
`c4-documentation`, `autoresearch`, `git-log`, `ddd-model`, `sparc-spec` at line 2 column 14
Fix: edit bad.yaml to match SPEC-004 schema
```

## When an agent calls this

- **Pre-flight a mapping** before spending a CLI run on it: catch a bad path or an oversize source early.
- **Dry-run a planned ingest** and show the user which rule would fire, without touching the artifact store.
- **Ask "what would this command be?"** - the hinted command line is the answer, and it is safe to hand to a human.
- **Check workspace confinement** - a refusal is the signal that the path the agent was given lives outside the project.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `mapping` | `string` (path) | yes | Path to the mapping YAML describing the transform (SPEC-004 schema). Must be an existing regular file inside an allowed root. By convention these live under `.forgeplan/mappings/`. |
| `source` | `string` (path) | yes | Path to the file the mapping consumes. Must be an existing regular file - a **directory is refused**, even though the CLI accepts one. |
| `dry_run` | `boolean` | no (default: `true`) | Plan-only. Echoed back in the response and appended to the hinted command. The MCP surface never writes either way. |
| `update` | `boolean` | no (default: `false`) | Allow updating existing artifacts on an idempotent re-ingest. Echoed back and appended to the hinted command. |
| `workspace` | `string` | no | Explicit workspace directory. When set, the server resolves `.forgeplan/` from this path instead of its startup CWD. PRD-078 / ADR-015. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::IngestParams`_

## Returns

```json
{
  "_next_action": "forgeplan ingest --mapping map.yaml --source docs/comp.md --dry-run",
  "drafts": [],
  "dry_run": true,
  "errors": [],
  "mapping": "map.yaml",
  "skipped": [],
  "source": "docs/comp.md",
  "update": false,
  "wave3_note": "Wave 3 MCP surface validates inputs only; full ingest execution available via the `forgeplan ingest` CLI. Wave 4 will wire IngestEngine + artifact::Store directly.",
  "wave3_status": "planned"
}
```

`drafts`, `skipped` and `errors` are always empty on this surface - they are the fields the Wave 4 executor will fill. Treat a non-empty `_next_action` as the deliverable of the call.

## Refusals

Every refusal is a tool error whose text carries the reason plus a `_next_action:` line:

| Refusal | Message | Hint |
|---|---|---|
| Mapping missing or outside the workspace | `mapping file not found or outside workspace` | Verify the mapping path. SPEC-004 mappings live under `.forgeplan/mappings/*.yaml` by convention. |
| Source missing, outside the workspace, or a directory | `source path not found or outside workspace` | Verify the source path exists under the workspace. |
| Mapping above 1 MiB | `mapping exceeds 1048576-byte size limit` | Trim the mapping or split into multiple SPEC-004 files. |
| Source above 10 MiB | `source exceeds 10485760-byte size limit` | Split the source file before ingestion. |

Path refusals are deliberately generic: the server never echoes the absolute path back to the client, so a probe cannot be used to map the host filesystem.

## Examples

Plan a run with the defaults (`dry_run` true, `update` false):

```json
{ "mapping": "map.yaml", "source": "docs/comp.md" }
```

Indicate intent to write and allow updates - the hint gains `--update` and drops `--dry-run`:

```json
{ "mapping": "map.yaml", "source": "docs/comp.md", "dry_run": false, "update": true }
```

```json
{
  "_next_action": "forgeplan ingest --mapping map.yaml --source docs/comp.md --update",
  "dry_run": false,
  "update": true,
  "wave3_status": "planned"
}
```

Point at a directory instead of a file:

```json
{ "mapping": "map.yaml", "source": "docs" }
```

```text
source path not found or outside workspace

_next_action: Verify the source path exists under the workspace.
```

## Notes

- The tool is annotated read-only, non-destructive and idempotent. Calling it repeatedly has no effect on the workspace.
- Both paths are redacted to workspace-relative form when they sit under the server's CWD, and shell-quoted in the hint so a path containing a space still pastes cleanly.
- The size limits exist to keep a hostile mapping or source from exhausting the server: they are checked before any read, so an oversize file is refused rather than truncated.

## CLI equivalent

- [`forgeplan ingest`](/docs/cli/ingest/) - the real executor. Same `--mapping` / `--source` flags plus `--dry-run`, `--update` and `--json`, and it is the only path that parses the mapping and writes artifacts.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) - the other Phase 5 surface that plans work it will not silently write
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - create a single artifact when a mapping would be overkill
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - wire the artifacts a CLI ingest produced
- [`forgeplan ingest`](/docs/cli/ingest/) - run the plan for real
