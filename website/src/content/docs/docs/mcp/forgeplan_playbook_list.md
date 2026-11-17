---
title: forgeplan_playbook_list
description: "List all discoverable playbooks (workspace + Claude plugin packs). Returns name, title, step count, and source path for each. PRD-065 AC-1. Read-only filesystem scan."
---

List all discoverable playbooks (workspace + Claude plugin packs). Returns name, title, step count, and source path for each. PRD-065 AC-1. Read-only filesystem scan.

**Category**: Other

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## What it does

1. Resolves the workspace from the server's current working directory.
2. Walks the playbook search roots (see [Discovery roots](#discovery-roots)), reading `.yaml` and `.yml` files.
3. Skips files above 1 MiB, YAML with more than 256 `{` / `[` opening characters in total (a heuristic, not recursive depth), unreadable files and loader failures.
4. Parses each surviving file with the SPEC-003 loader and keeps the first occurrence of each playbook `name`.
5. Sorts the survivors by `name` and returns the array plus a `total` count.

The scan never errors: a directory that does not exist, an unreadable file, or a malformed playbook produces an omission, not a failure.

## When an agent calls this

- Before [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) - resolve a bare playbook name to a real one instead of guessing.
- When `forgeplan_playbook_show` answered `playbook target ... not resolvable` - this is the recovery call.
- To detect whether a workspace ships its own playbooks in `.forgeplan/playbooks/` or relies on an installed pack.
- Capability discovery during routing: an empty list means no valid playbooks were found in these roots, not that path-addressed playbooks or orchestration elsewhere are unavailable.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP text content. There is no `workspace` parameter or workspace
trace; discovery uses process CWD and `$HOME` and does not require an initialized
workspace to scan plugin roots.

| Field | Type | Description |
|---|---|---|
| `playbooks` | `array` | One object per discovered playbook, sorted by `name`. |
| `playbooks[].name` | `string` | Playbook identifier from the YAML `name:` field. |
| `playbooks[].title` | `string` | Human-readable title. |
| `playbooks[].steps_count` | `number` | Number of entries in `steps:`. |
| `playbooks[].source_path` | `string` | Absolute path of the file the playbook was loaded from. |
| `total` | `number` | Length of the `playbooks` array. |
| `_next_action` | `string` | `forgeplan_playbook_show target="<first-name>"`, or `Done.` when the list is empty. |

## Examples

Illustrative empty discovery result, with terminal hint `Done.`:

```json
{
  "_next_action": "Done.",
  "playbooks": [],
  "total": 0
}
```

Illustrative workspace with one playbook in `.forgeplan/playbooks/brownfield-code.yaml`:

```json
{
  "_next_action": "forgeplan_playbook_show target=\"brownfield-code\"",
  "playbooks": [
    {
      "name": "brownfield-code",
      "source_path": "/Users/me/project/.forgeplan/playbooks/brownfield-code.yaml",
      "steps_count": 4,
      "title": "Brownfield code-first onboarding"
    }
  ],
  "total": 1
}
```

The handler emits `playbooks`, `total` and `_next_action`; each playbook row has the four fields listed above.

## Discovery roots

| Root | Notes |
|---|---|
| `.forgeplan/playbooks/` | Non-recursive: `.yaml` and `.yml` files directly in that directory. |
| `~/.claude/plugins/*/playbooks/` | One level of plugin directories, each with its own `playbooks/` folder. |

Both roots are non-recursive. A nested `playbooks/subdir/` is not scanned.

## Notes

- **The MCP scan is narrower than the CLI scan.** [`forgeplan playbook list`](/docs/cli/playbook-list/) also searches `marketplace/playbooks/`; this tool does not. A CLI-discoverable name may therefore be absent here. Pass its allowed **path** to `forgeplan_playbook_show` or [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) instead of the bare name. A returned name is not proof that its delegates can execute.
- **Silent omissions are by design.** Oversized, too-deeply-nested, and unparseable files are dropped without a warning field. Use [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) on a specific file to learn why it is absent.
- **First path wins on duplicate names.** Roots are visited in the order listed above, so a workspace playbook shadows a plugin pack playbook with the same `name`.
- **Recommendations are not in this response.** Playbook recommendation from project signals (`triggered_by`: `empty_repo`, `has_git`, `commit_count_min`, `has_docs`, `has_obsidian`, `has_cargo_toml`) is computed by the recommendation engine and emitted by [`forgeplan init`](/docs/cli/init/) on stderr. Set `FORGEPLAN_HINTS=1` to force those hints in a non-TTY pipeline. This tool returns the enumeration only.
- No LLM provider is required. No workspace artefacts are read or written.

## Related commands

- [`forgeplan playbook list`](/docs/cli/playbook-list/) - the CLI equivalent with the wider search path.
- [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) - the next call for any name this tool returns.
- [`forgeplan_plugins_list`](/docs/mcp/forgeplan_plugins_list/) - the plugin inventory that determines which pack playbooks exist on disk.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) - full structure of one playbook
- [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) - why a file was dropped by discovery
- [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) - execute a discovered playbook
- [`forgeplan playbook list`](/docs/cli/playbook-list/) - the CLI twin
