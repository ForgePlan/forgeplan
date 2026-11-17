---
title: forgeplan_playbook_validate
description: "Validate a playbook YAML file (parse + structural checks: cycles, unknown step refs, mapping/produces_at consistency). Returns `passed` flag + list of errors. PRD-065 AC-2."
---

Validate a playbook YAML file (parse + structural checks: cycles, unknown step refs, mapping/produces_at consistency). Returns `passed` flag + list of errors. PRD-065 AC-2.

**Category**: Other

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "file": ".forgeplan/playbooks/brownfield-code.yaml"
}
```

## What it does

1. Canonicalizes `file` and checks it is a regular file inside an allowed root.
2. Reads it under the 1 MiB cap and a heuristic cap of 256 `{` / `[` opening characters in total, not recursive YAML depth.
3. Runs the SPEC-003 loader: parse (including unknown-field checks on strict types), schema-version check, non-empty steps, unknown references, cycles, then mapping/output consistency. It stops at the first error; unknown step fields are not rejected by the `Step` type.
4. Returns `passed: true` with counts, or `passed: false` with a single structured error object.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `file` | `string` | yes | Path to the playbook YAML file to validate. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Checks performed

| Check | Severity | Result |
|---|---|---|
| YAML parses, required fields present, strict types have no unknown fields, known `delegate_to.type` | ERROR | `kind: yaml_parse_error` with `line` and `column` |
| `schema_version` is inside the supported range | ERROR | `kind: unsupported_schema_version` with `version` and `supported` |
| `steps:` is not empty | ERROR | `kind: empty_steps` |
| Every `requires:` entry names a step that exists | ERROR | `kind: unknown_step_ref` with `count` |
| The `requires:` graph is acyclic | ERROR | `kind: cycle` with `path_len` |
| A step with `mapping:` also sets `produces_at:` | ERROR | `kind: mapping_without_produces_at` with `step_id` |
| `produces_at:` without `mapping:` | WARN | Loads; warning is logged, not returned in a `warnings` field |
| Command delegate or undeclared plugin/skill prerequisite | WARN | Loads; execution and consent remain separate |

## Returns

The table describes JSON returned in MCP text content after a successful file read.
A loader failure with `passed: false` is still an MCP success, not `isError: true`.
Path and bounded-read failures instead return a plain-text tool error with an
`_next_action:` line and no `passed` field. No `workspace` parameter or trace exists.

| Field | Type | Description |
|---|---|---|
| `passed` | `boolean` | `true` when the playbook loaded cleanly. |
| `name` | `string` | Playbook name. Present only when `passed` is `true`. |
| `title` | `string` | Playbook title. Present only when `passed` is `true`. |
| `steps_count` | `number` | Number of steps. Present only when `passed` is `true`. |
| `errors` | `array` | Empty on success; on failure exactly one structured object (the loader stops at the first error). |
| `source_path` | `string` | Redacted path the validation ran against. |
| `_next_action` | `string` | On success the dry-run form of the run call; on failure the re-validate call. |

## Examples

Validate a path:

```json
{ "file": ".forgeplan/playbooks/brownfield-code.yaml" }
```

Illustrative passing result for a four-step playbook:

```json
{
  "_next_action": "forgeplan_playbook_run target=\"brownfield-code\" yes=true dry_run=true",
  "errors": [],
  "name": "brownfield-code",
  "passed": true,
  "source_path": "marketplace/playbooks/brownfield-code.yaml",
  "steps_count": 4,
  "title": "Brownfield code-first onboarding"
}
```

A failing run. The `errors` entry carries the error class and the metadata that survives redaction - here `count` says how many `requires:` references are broken, but not which:

```json
{
  "_next_action": "Fix the YAML, then re-validate: forgeplan_playbook_validate file=\"<path>\"",
  "errors": [{ "kind": "unknown_step_ref", "count": 1 }],
  "passed": false,
  "source_path": ".forgeplan/playbooks/badref.yaml"
}
```

Illustrative CLI comparison. Unlike MCP's redacted error metadata, CLI names the offending reference:

```text
$ forgeplan playbook validate .forgeplan/playbooks/badref.yaml --json
{
  "_next_action": "forgeplan playbook validate .forgeplan/playbooks/badref.yaml",
  "details": [
    "step `a` requires unknown step `nope`"
  ],
  "error": "1 step(s) reference unknown step IDs in `requires:`",
  "passed": false,
  "source_path": ".forgeplan/playbooks/badref.yaml"
}
```

## Error kinds

| `kind` | Extra fields | Meaning |
|---|---|---|
| `yaml_parse_error` | `line`, `column` | Parse failure. `line` and `column` are `null` when the parser reported no location. |
| `empty_steps` | - | `steps:` is an empty array. |
| `unknown_step_ref` | `count` | At least one `requires:` entry names a step that does not exist. Individual step IDs are redacted. |
| `cycle` | `path_len` | The `requires:` graph contains a cycle. The cycle members are redacted. |
| `mapping_without_produces_at` | `step_id` | A step sets `mapping:` with nothing for it to ingest. |
| `unsupported_schema_version` | `version`, `supported` | The declared `schema_version` is outside the supported range. |
| `internal_range` | `range` | Internal loader range error. Report it as a bug. |

Error content is redacted by design: the raw parser message can quote lines of the file, and the file may live anywhere the caller pointed the tool at, so only structured metadata is returned. When you need the offending step IDs, run [`forgeplan playbook validate`](/docs/cli/playbook-validate/) on the same path.

## Notes

- **This is a static check.** Nothing is executed, no dispatcher is constructed, and no journal entry is written. Read-only, idempotent, non-destructive.
- **One error at a time.** The loader returns on the first failure, so `errors` has length 1 whenever `passed` is `false`. Fix, re-run, repeat.
- **`passed: true` does not mean the playbook will run.** It checks structure, not installed prerequisites, credentials or output files. Runtime dispatch can still fail; skill steps in 0.38.0 only print a marker and do not detect missing skills.
- A path outside the allowed roots fails with `cannot read playbook (path not in workspace)`, with `_next_action` pointing at [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/).

## Related commands

- [`forgeplan playbook validate`](/docs/cli/playbook-validate/) - the CLI equivalent, with unredacted error detail.
- [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) - the parsed playbook on success.
- [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) - the dry run that enumerates steps before execution.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) - inspect the playbook after a clean validation
- [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) - run it, dry first
- [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/) - recover when the path does not resolve
- [`forgeplan playbook validate`](/docs/cli/playbook-validate/) - the CLI twin with full error detail
