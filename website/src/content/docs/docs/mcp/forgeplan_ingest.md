---
title: forgeplan_ingest
description: "Check ingest paths and sizes and prepare a CLI command. Core 0.38 does not generate draft previews or apply ingestion, even with dry_run false."
---

Checks paths and file sizes for a proposed ingest run and returns a CLI command hint. In 0.38.0 this MCP endpoint does not parse the mapping or source, run the ingest engine, produce draft previews or apply changes, even with `dry_run: false`.

**Category**: Other

This tool is a **planner, not an executor**. It requires existing regular files under an allowed root: the project containing `.forgeplan/` found from the server process CWD, or `~/.claude/plugins/`. It has no `workspace` input. Nothing is written to the artifact store; the caller must separately decide whether to execute the hinted CLI command.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "mapping": "map.yaml",
  "source": "docs/components.md",
  "dry_run": true
}
```

## What it does

1. **Canonicalise the mapping path.** The file must exist, must be a regular file, and must resolve under an allowed root: the workspace root (the parent of `.forgeplan/`) or the `~/.claude/plugins/` tree where plugin packs install their bundles. Symlink escapes are resolved before the check, so a benign-looking relative path cannot point outside.
2. **Canonicalise the source path** with the same rules.
3. **Check the size budget.** The mapping is refused above 1 MiB, the source above 10 MiB. The check reads file metadata only - the server never loads either file into memory here.
4. **Echo the plan.** The response repeats both paths (redacted to workspace-relative where possible), both flags, and `wave3_status: "planned"` with `drafts`, `skipped` and `errors` as empty lists.
5. **Hint the real command.** `_next_action` carries a ready-to-paste `forgeplan ingest --mapping ... --source ...` line, with `--dry-run` and/or `--update` appended to match the flags you passed.

Steps 1–3 inspect paths and metadata, not file contents. Even an invalid mapping
can therefore receive a plan. For example, a mapping with an unsupported
`source_kind` may pass this preflight but fail when the CLI parses it. A plan is
not schema validation, a preview of matching rules, or evidence that ingestion will succeed.

## When an agent calls this

- **Pre-flight a mapping** before spending a CLI run on it: catch a bad path or an oversize source early.
- **Prepare the CLI invocation** without touching the artifact store. To preview actual rules and drafts, run the CLI with `--dry-run` separately.
- **Ask "what would this command be?"** — inspect the hint and confirm its paths before execution.
- **Check workspace confinement** - a refusal is the signal that the path the agent was given lives outside the project.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `dry_run` | `boolean` | no | Plan-only - engine runs but no artifacts written. Wave 3 default. Schema default: `true`. |
| `mapping` | `string` | yes | Path to mapping YAML file (SPEC-004 schema). |
| `source` | `string` | yes | Path to the source file (or directory) the mapping consumes. |
| `update` | `boolean` | no | Allow updating existing artifacts (idempotent re-ingest). Schema default: `false`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP text content. Values and paths below are illustrative.

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

`drafts`, `skipped` and `errors` are always empty on this endpoint. They do not report parsed drafts, successful skips or the absence of mapping errors. `wave3_note` describes planned future wiring, not functionality present in 0.38.0.

## Refusals

Every refusal is a tool error whose text carries the reason plus a `_next_action:` line:

| Refusal | Message | Hint |
|---|---|---|
| Mapping missing or outside the workspace | `mapping file not found or outside workspace` | Verify the mapping path. SPEC-004 mappings live under `.forgeplan/mappings/*.yaml` by convention. |
| Source missing, outside the workspace, or a directory | `source path not found or outside workspace` | Verify the source path exists under the workspace. |
| Mapping above 1 MiB | `mapping exceeds 1048576-byte size limit` | Trim the mapping or split into multiple SPEC-004 files. |
| Source above 10 MiB | `source exceeds 10485760-byte size limit` | Split the source file before ingestion. |

Path refusals are deliberately generic and do not echo absolute host paths. They cover both workspace and plugin-root checks, despite the shorter wording in the message.

## Examples

Plan a run with the defaults (`dry_run` true, `update` false):

```json
{ "mapping": "map.yaml", "source": "docs/comp.md" }
```

Indicate intent to write and allow updates - the hint gains `--update` and drops `--dry-run`:

```json
{ "mapping": "map.yaml", "source": "docs/comp.md", "dry_run": false, "update": true }
```

Illustrative response excerpt (the full response still contains the other fields above):

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

- The tool is annotated read-only, non-destructive and idempotent; it does not apply ingest changes.
- Paths under process CWD or its detected workspace root are made relative; other accepted paths become a bare filename. The command hint shell-quotes them, but a plugin-root filename may no longer resolve from your shell CWD. Substitute the real allowed path before executing it.
- Oversized files are refused from metadata, not truncated. No content validation or YAML nesting check runs in this handler.

## CLI equivalent

- [`forgeplan ingest`](/docs/cli/ingest/) — executes the pipeline using `--mapping` / `--source`, with `--dry-run`, `--update` and `--json`. Its dry run is the way to inspect actual drafts. A playbook's `forgeplan_core:ingest` delegate is another execution path; it is not this plan-only MCP endpoint.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) — executes delegates with consent; unlike this endpoint, its core ingest delegate can write artifacts
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - create a single artifact when a mapping would be overkill
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - wire the artifacts a CLI ingest produced
- [`forgeplan ingest`](/docs/cli/ingest/) - run the plan for real
