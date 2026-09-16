---
title: forgeplan_plugins_list
description: "List installed plugins detected on disk (Claude plugins + agentskills + Cursor) + missing entries from the extended registry. PRD-067 AC-1. Read-only filesystem scan."
---

List installed plugins detected on disk (Claude plugins + agentskills + Cursor) + missing entries from the extended registry. PRD-067 AC-1. Read-only filesystem scan.

**Category**: Other

The tool answers one question in two halves: **what is on this machine**, and **what is in the registry but not on this machine**. The registry is a static list compiled into forgeplan-core; the installed half comes from walking the filesystem. The gap between them is the install work still owed to the user.

## What it does

1. **Loads the extended registry** - the static list of plugins, sub-agents and skill packs forgeplan knows how to detect or recommend, each with a version requirement, expected paths, an install command and a description.
2. **Scans the filesystem** through that registry: for each entry, every expected path under each of that source's search roots is probed until one exists. The first match wins, so a plugin present in two locations is still reported once.
3. **Tries to read a version** from the matched directory, in order: `manifest.json`, `plugin.json`, then the YAML frontmatter of `SKILL.md`. A version that cannot be parsed is not an error - it is reported as absent.
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

This tool takes no parameters. The server declares an explicit empty parameter object so the JSON Schema advertises a correct empty object rather than an ambiguous one.

| Name | Type | Required | Description |
|---|---|---|---|
| (none) | - | - | No parameters. Call with an empty argument object. |

_Schema source: `crates/forgeplan-core/src/plugins/types.rs::EmptyParams`_

## Returns

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
- **When a task needs a capability the agent does not have** - a non-empty `missing` list names the exact install command.
- **As a cheap environment probe** at the start of a session, to record which plugin packs are available on this machine.

## Examples

Enumerate everything:

```json
{}
```

Typical follow-up, reading the entry that the hint points at:

```json
{ "name": "c4-architecture" }
```

## Notes

- Read-only, non-destructive and idempotent: repeated calls return the same lists.
- The scanner never panics on a missing or unreadable directory - an absent search root simply contributes nothing. A plugin that cannot be proven present is reported as missing.
- Detection is by path, not by a lockfile: a plugin directory that exists but carries no parseable manifest still counts as installed, with the version left out.
- The registry is fixed at build time for a given forgeplan version, so a newly published plugin pack does not appear in `missing` until forgeplan is upgraded.

## CLI equivalent

- [`forgeplan plugins list`](/docs/cli/plugins-list/) - the same scan with a human-readable table, plus `--json` for the raw payload.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_plugins_info`](/docs/mcp/forgeplan_plugins_info/) - drill into one registry entry
- [`forgeplan_plugins_doctor`](/docs/mcp/forgeplan_plugins_doctor/) - the ok / outdated / missing split with install hints
- [`forgeplan plugins list`](/docs/cli/plugins-list/) - terminal listing
- [`forgeplan plugins`](/docs/cli/plugins/) - the CLI command family
