---
title: forgeplan_playbook_show
description: "Show full details of a single playbook (resolved by name or path). Returns the parsed Playbook struct + source path. PRD-065 AC-1."
---

Show full details of a single playbook (resolved by name or path). Returns the parsed Playbook struct + source path. PRD-065 AC-1.

**Category**: Other

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "target": "brownfield-code"
}
```

## What it does

1. Resolves `target`: a path-like string (it contains `/`, `\`, or ends in `.yaml` / `.yml`) is canonicalized and confined to an allowed root; anything else is treated as a bare name and looked up through the same discovery pass that [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/) performs.
2. Reads the file under a 1 MiB size cap and a heuristic cap of 256 `{` / `[` opening characters in total, not recursive YAML depth.
3. Parses it with the SPEC-003 loader, which rejects an empty `steps` array, `requires:` entries naming steps that do not exist, cycles in the `requires:` graph, `mapping:` without `produces_at:`, and an unsupported `schema_version`. A `produces_at:` without `mapping:` is a warning, not an error, and the playbook still loads.
4. Returns the parsed playbook plus the redacted source path.

This is the read surface for playbook structure. It executes nothing.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `target` | `string` | yes | Playbook name (matches discovery results) or path to a YAML file. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

| Field | Type | Description |
|---|---|---|
| `playbook` | `object` | The parsed playbook (see below). |
| `source_path` | `string` | Path the playbook was loaded from, redacted: workspace-relative when the file sits under the working directory, otherwise the bare file name. Absolute host paths are never echoed. |
| `_next_action` | `string` | Always the dry-run form of the run call: `forgeplan_playbook_run target="<name>" yes=true dry_run=true`. |

### Playbook object

| Field | Type | Description |
|---|---|---|
| `schema_version` | `string` | SPEC-003 schema version, e.g. `"1.0.0"`. The abbreviated YAML form `"1.0"` is zero-extended. |
| `name` | `string` | Playbook identifier, kebab-case by convention. |
| `title` | `string` | Human-readable title. |
| `description` | `string` | Optional multi-line description. |
| `triggered_by` | `object` | Optional project signals used by the recommendation engine: `empty_repo`, `has_git`, `commit_count_min`, `has_docs`, `has_obsidian`, `has_cargo_toml`. |
| `requires` | `object` | Optional prerequisites: `plugins` as `name` plus optional `version` range, and `skills` as `name` plus optional `pack`. |
| `steps` | `array` | Ordered step objects, at least one required. |

### Step object

| Field | Type | Description |
|---|---|---|
| `id` | `string` | Step identifier. Use unique values; this loader does not reject duplicate IDs. |
| `delegate_to` | `object` | One of five variants, tagged on `type`: `plugin` (`name`, `target`), `agent` (`name`), `skill` (`name`, optional `pack`), `command` (`command` argv array), `forgeplan_core` (`target` of `ingest`, `new`, `validate`, `activate`, or `search`). |
| `input` | `JSON value` | Optional free-form YAML value serialized to JSON; it is not restricted to an object by the playbook type. Individual dispatchers may require a mapping. |
| `produces_at` | `string` | Optional workspace-relative output path. |
| `mapping` | `string` | Optional mapping metadata; the loader requires `produces_at` alongside it. This field alone does not make the executor ingest output: use an explicit core `ingest` step. |
| `requires` | `array` | Step IDs that must complete first. Defines the DAG. |
| `fallback_hint` | `string` | Install hint printed when the delegate is missing. |
| `on_error` | `string` | `abort` (default) or `continue`. |
| `timeout_seconds` | `number` | Optional override used by subprocess dispatchers: plugin 600s, agent 300s, command 180s when omitted. The marker-only skill dispatcher does not enforce a timeout. |
| `budget_usd` | `number` | Optional per-step USD cap, translated to `--max-budget-usd` for `claude --print` delegates. |
| `allowed_tools` | `array` | Optional tool allowlist for `claude --print` delegates, translated to `--allowedTools`. |

Absent optional fields are omitted, not emitted as `null`. `on_error` is emitted
with its default `abort`. Top-level `requires` describes prerequisites but does
not attest to installation or an active agent. In particular, the 0.38.0 skill
dispatcher does not validate or load skills. No `workspace` input or workspace
trace is available on this read endpoint.

The hint uses the parsed playbook name. If you supplied a path outside discovery
roots, retain that path for the next call rather than assuming the name is discoverable.

## Examples

Show a discovered playbook by name:

```json
{ "target": "brownfield-code" }
```

Show a file by path. A bare name is looked up through discovery, so a playbook that only lives in `marketplace/playbooks/` must be addressed this way:

```json
{ "target": "marketplace/playbooks/brownfield-code.yaml" }
```

Illustrative response excerpt with description and steps shortened. This is not a complete serialized playbook or an executed capture:

```json
{
  "_next_action": "forgeplan_playbook_run target=\"brownfield-code\" yes=true dry_run=true",
  "playbook": {
    "description": "Four-step orchestration that converts a code-heavy brownfield repo …",
    "name": "brownfield-code",
    "requires": {
      "plugins": [{ "name": "c4-architecture", "version": ">=1.0" }],
      "skills": [{ "name": "forge-history-miner", "pack": "brownfield-code-pack" }]
    },
    "schema_version": "1.0.0",
    "steps": [
      {
        "delegate_to": { "name": "c4-architecture", "target": "c4-code", "type": "plugin" },
        "fallback_hint": "claude plugin install c4-architecture",
        "id": "run-c4-architecture",
        "input": { "scope": "full-repo" },
        "on_error": "abort",
        "produces_at": "C4-Documentation/"
      },
      {
        "delegate_to": { "target": "ingest", "type": "forgeplan_core" },
        "id": "ingest-c4",
        "input": {
          "mapping_path": "marketplace/mappings/c4-to-forge.yaml",
          "source_path": "C4-Documentation/"
        },
        "on_error": "abort",
        "requires": ["run-c4-architecture"]
      }
    ],
    "title": "Brownfield code-first onboarding",
    "triggered_by": {
      "commit_count_min": 50,
      "has_docs": false,
      "has_git": true,
      "has_obsidian": false
    }
  },
  "source_path": "marketplace/playbooks/brownfield-code.yaml"
}
```

## Errors

| Situation | Message | `_next_action` |
|---|---|---|
| Name not discovered, or path outside the allowed roots | `playbook target <target> not resolvable` | `List discoverable playbooks: forgeplan_playbook_list` |
| Read failed, file over 1 MiB, or nesting over the cap | `cannot read playbook: file exceeds the 1048576-byte size limit` (or the nesting / io-error variant) | `Verify the file is well-formed and below 1 MiB.` |
| Loader rejected the YAML | `playbook parse error: <kind>` | `Validate to see structured findings: forgeplan_playbook_validate file="<path>"` |

Success is JSON in MCP text content. These failures instead return plain-text
tool errors (`isError: true`) with an `_next_action:` line. There is no common
JSON error payload shared with `forgeplan_playbook_validate`.

## Safety notes

- **Path confinement.** The target must canonicalize inside the workspace root or the `~/.claude/plugins/` tree. `/etc/passwd` and `../../etc/passwd` are rejected even when the underlying file exists, and when no allowed root can be resolved at all the tool refuses rather than falling through to "anything goes".
- **Parse errors are redacted.** The `<kind>` in a parse error is a structured label (`yaml_parse_error`, `empty_steps`, `unknown_step_ref`, `cycle`, `mapping_without_produces_at`, `unsupported_schema_version`, `internal_range`), never the raw parser message, because that message can quote excerpts of the offending file.
- Read-only, idempotent, non-destructive. No dispatcher is invoked and no step runs.

## Related commands

- [`forgeplan playbook show`](/docs/cli/playbook-show/) - the CLI equivalent, same loader and same checks.
- [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/) - the name source for `target`.
- [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) - structured error detail when parsing fails.
- [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) - execute the playbook this tool describes.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/) - discover the names this tool accepts
- [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) - validate a file before shipping it
- [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) - dry-run first, then execute
- [`forgeplan playbook show`](/docs/cli/playbook-show/) - the CLI twin
