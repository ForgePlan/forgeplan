---
title: forgeplan_plugins_list
description: "List installed plugins detected on disk (Claude plugins + agentskills + Cursor) + missing entries from the extended registry. PRD-067 AC-1. Read-only filesystem scan."
---

List installed plugins detected on disk (Claude plugins + agentskills + Cursor) + missing entries from the extended registry. PRD-067 AC-1. Read-only filesystem scan.

**Category**: Other

The tool inventories **compiled registry entries detected on disk** and **registered entries not detected**. It is not a catalogue of every installed plugin or an active-agent roster. A `missing` entry is an optional capability, not install work owed to the user unless the selected workflow needs it. Neither registration nor a matching path proves that a host has loaded or can run that agent.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## What it does

1. **Loads the extended registry** - the static list of plugins, sub-agents and skill packs forgeplan knows how to detect or recommend, each with a version requirement, expected paths, an install command and a description.
2. **Scans the filesystem** through that registry: for each entry, every expected path under each of that source's search roots is probed until one exists. The first match wins, so a plugin present in two locations is still reported once.
3. **Tries to read a version string** from `manifest.json`, `plugin.json`, then `SKILL.md` YAML frontmatter. Missing/unreadable version metadata is absent; a malformed semver string can still be returned as `detected_version`. Compatibility is checked by the doctor/info tools, not this scan.
4. **Splits the two lists.** Entries with a filesystem match go into `installed` with their detected path and version; everything else goes into `missing` as the static registry entry only.
5. **Hints the next call**: `forgeplan_plugins_info` for one of the detected plugins (the first one the scan produced), or `forgeplan_plugins_doctor` when nothing was detected while the registry still has gaps.

Nothing is written and nothing is installed. This is a scan, not an installer.

## Where it looks

| Source | Value in the response | Search roots |
|---|---|---|
| Claude Code plugin | `claude-plugin` | `~/.claude/plugins/cache/`, `.claude/plugins/` |
| agentskills.io skill | `agent-skills` | `.agentskills/`, `~/.agentskills/` |
| Cursor skill | `cursor` | `.cursor/skills/` |
| Built into forgeplan | `forgeplan` | not scanned |
| User-provided | `manual` | not scanned |

Relative roots resolve against the server's working directory, so a project-local `.claude/plugins/` is found as well as the user-level cache. The `forgeplan` entry itself is synthetic: it is always reported installed, with the path `(built-in)` and the version of the running binary.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP text content, with `installed`, `missing`,
`installed_count`, `missing_count` and `_next_action`. There is no `workspace`
input or workspace trace. Missing roots/manifests are omissions, not tool errors.

Historical 0.37.0 response capture, trimmed for display. These machine-specific counts are not the public marketplace plugin count or the current release version:

```json
{
  "_next_action": "forgeplan_plugins_info name=\"forgeplan\"",
  "installed": [
    {
      "detected_path": "(built-in)",
      "detected_version": "0.37.0",
      "info": {
        "description": "Forgeplan core methodology engine (always present)",
        "expected_paths": [],
        "install_command": "(built-in)",
        "name": "forgeplan",
        "source": "forgeplan",
        "version_req": "*"
      }
    }
  ],
  "installed_count": 1,
  "missing": [
    {
      "description": "C4 model diagrams (context/container/component/code) generator",
      "expected_paths": ["c4-architecture"],
      "install_command": "/plugin install c4-architecture@claude-code-workflows",
      "name": "c4-architecture",
      "source": "claude-plugin",
      "version_req": ">=1.0"
    }
  ],
  "missing_count": 11
}
```

The `missing` array is trimmed here to one entry; the counts are from a machine where only the built-in plugin was detected. An `installed` entry always nests the full static `info` record alongside the runtime evidence (`detected_path`, `detected_version`), so the caller can compare what was found against what was required.

`detected_version` is omitted entirely when no manifest published a version.

## When an agent calls this

- **Before recommending an install** - never tell the user to add a plugin that is already present.
- **After an install**, to confirm the detection actually picked it up (and at which version).
- **When a task needs an optional capability** — inspect the matching registry entry with `forgeplan_plugins_info` and confirm current public installation instructions before recommending an install.
- **As a cheap environment probe** at the start of a session, to record which plugin packs are available on this machine.

## Examples

Enumerate everything:

```json
{}
```

Typical follow-up **with `forgeplan_plugins_info`**, reading a registry entry (not arguments to `forgeplan_plugins_list`):

```json
{ "name": "c4-architecture" }
```

## Notes

- Read-only, non-destructive and idempotent. Filesystem changes can change the result, and HashMap registry order is not stable; sort entries by `info.name` / `name` before comparing snapshots.
- The scanner never panics on a missing or unreadable directory - an absent search root simply contributes nothing. A plugin that cannot be proven present is reported as missing.
- Detection is by path, not by a lockfile: a plugin directory that exists but carries no parseable manifest still counts as installed, with the version left out.
- The registry is fixed at build time for a given forgeplan version, so a newly published plugin pack does not appear in `missing` until forgeplan is upgraded.
- Registry install hints are compiled strings, not a live marketplace lookup. They do not prove current publication, host activation or implementation of a core MCP endpoint.

## CLI equivalent

- [`forgeplan plugins list`](/docs/cli/plugins-list/) - the same scan with a human-readable table, plus `--json` for the raw payload.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_plugins_info`](/docs/mcp/forgeplan_plugins_info/) - drill into one registry entry
- [`forgeplan_plugins_doctor`](/docs/mcp/forgeplan_plugins_doctor/) - the ok / outdated / missing split with install hints
- [`forgeplan plugins list`](/docs/cli/plugins-list/) - terminal listing
- [`forgeplan plugins`](/docs/cli/plugins/) - the CLI command family
