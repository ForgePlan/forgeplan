---
title: forgeplan_playbook_validate
description: "Validate a playbook YAML file (parse + structural checks: cycles, unknown step refs, mapping/produces_at consistency). Returns `passed` flag + list of errors. PRD-065 AC-2."
---

Validate a playbook YAML file (parse + structural checks: cycles, unknown step refs, mapping/produces_at consistency). Returns `passed` flag + list of errors. PRD-065 AC-2.

**Category**: Other

## What it does

1. Canonicalizes `file` and checks it is a regular file inside an allowed root.
2. Reads it under the 1 MiB size cap and the 256-character YAML nesting cap.
3. Runs the SPEC-003 loader. The loader parses the YAML with `deny_unknown_fields`, then checks the structural invariants below, in this order. It stops at the first failure.
4. Returns `passed: true` with counts, or `passed: false` with a single structured error object.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `file` | `string` | yes | Path to the playbook YAML to validate. A bare playbook name is **not** looked up through discovery here - pass a path. The path must canonicalize inside the workspace root or the `~/.claude/plugins/` tree. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::PlaybookValidateParams`_

## Checks performed

| Check | Severity | Result |
|---|---|---|
| YAML parses, all required fields present, no unknown top-level field, known `delegate_to.type` | ERROR | `kind: yaml_parse_error` with `line` and `column` |
| `steps:` is not empty | ERROR | `kind: empty_steps` |
| Every `requires:` entry names a step that exists in this playbook | ERROR | `kind: unknown_step_ref` with `count` |
| The `requires:` graph is acyclic | ERROR | `kind: cycle` with `path_len` |
| A step with `mapping:` also sets `produces_at:` | ERROR | `kind: mapping_without_produces_at` with `step_id` |
| `schema_version` is inside the supported range | ERROR | `kind: unsupported_schema_version` with `version` and `supported` |
| A step with `produces_at:` but no `mapping:` | WARN | Loads fine; the loader logs a warning that the output will be captured but not ingested |
| A `delegate_to: command` step (shell exec) | WARN | Loads fine; execution is separately gated - see [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) |

## Returns

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

A passing run (captured against the shipped `brownfield-code` playbook):

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

The same broken file through the CLI, which is not redacted and therefore names the offending step:

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
- **A `passed: true` does not mean the playbook will run.** It means the YAML is structurally sound. Missing plugins, missing skills, and missing credentials only surface during [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/).
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
