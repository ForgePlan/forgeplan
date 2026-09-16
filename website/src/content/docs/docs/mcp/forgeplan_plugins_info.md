---
title: forgeplan_plugins_info
description: "Show details for a single plugin from the extended registry. Returns the static PluginInfo plus, if installed, the InstalledPlugin runtime record (detected path + version). PRD-067."
---

Show details for a single plugin from the extended registry. Returns the static PluginInfo plus, if installed, the InstalledPlugin runtime record (detected path + version). PRD-067.

**Category**: Other

One registry entry, fully resolved. The response merges two sources: the **static** record, which is identical on every machine, and the **runtime** record, which is what this machine actually has. A plugin that is registered but not detected comes back with a full static record and a null runtime half - that is a normal answer, not an error.

## What it does

1. **Looks the name up in the extended registry.** The lookup is exact and case-sensitive against the registry key, so it is a kebab-case plugin name. A name that is not registered is refused before any filesystem work happens.
2. **Returns the static `info` record**: name, distribution source, version requirement, expected paths, install command and description.
3. **Scans for that plugin on disk** and, when a path matches, attaches the `installed` record with the path that resolved and the version read from its manifest.
4. **Checks compatibility** to pick the hint: a detected version satisfying the requirement ends the call with `Done.`, anything else asks for an install or an update using the registry's own install command.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `name` | `string` | yes | Plugin name in kebab-case, exactly as registered. For example `c4-architecture`, `agents-pro`, `forgeplan`. |

_Schema source: `crates/forgeplan-core/src/plugins/types.rs::PluginsInfoParams`_

## Returns

Registered and detected on this machine:

```json
{
  "_next_action": "Done.",
  "info": {
    "description": "Forgeplan core methodology engine (always present)",
    "expected_paths": [],
    "install_command": "(built-in)",
    "name": "forgeplan",
    "source": "forgeplan",
    "version_req": "*"
  },
  "installed": {
    "detected_path": "(built-in)",
    "detected_version": "0.37.0",
    "info": { "name": "forgeplan", "source": "forgeplan", "version_req": "*" }
  }
}
```

Registered but not detected - the static record is complete and `installed` is null:

```json
{
  "_next_action": "Install: `/plugin install c4-architecture@claude-code-workflows`.",
  "info": {
    "description": "C4 model diagrams (context/container/component/code) generator",
    "expected_paths": ["c4-architecture"],
    "install_command": "/plugin install c4-architecture@claude-code-workflows",
    "name": "c4-architecture",
    "source": "claude-plugin",
    "version_req": ">=1.0"
  },
  "installed": null
}
```

The static `info` is nested inside `installed` as well, so a consumer holding only the runtime half still sees the requirement it was measured against.

## How the hint is chosen

| State | `_next_action` |
|---|---|
| Detected and the version satisfies the requirement | `Done.` |
| Detected but the version fails the requirement, or no version could be read | `Update plugin: <install command>.` |
| Not detected | `Install: <install command>.` |

Because a missing version counts as "cannot prove compatibility", a plugin directory with no parseable manifest lands in the update branch even when it is freshly installed. Read `installed.detected_version` before acting on that hint.

## Refusals

A name that is not in the registry is a tool error with a pointer to the full list:

```text
plugin `nope-not-real` not in registry

_next_action: List all known plugins: `forgeplan_plugins_list`.
```

The refusal happens before the filesystem scan, so a typo costs nothing and never reports a partial answer.

## When an agent calls this

- **Confirming the install command** before showing it to the user, rather than guessing a marketplace name.
- **Checking a version requirement** before depending on a plugin capability.
- **After an install**, to verify the detected version satisfies what forgeplan expects.

## Examples

Read one registered plugin:

```json
{ "name": "c4-architecture" }
```

Read the built-in entry to learn the running version:

```json
{ "name": "forgeplan" }
```

## Notes

- Read-only, non-destructive and idempotent.
- Detection is a path probe, not an execution: the plugin is never loaded, imported or run to answer this call.
- The registry is compiled into the binary, so an entry can be registered while the plugin itself is published elsewhere - which is exactly what `missing` in [`forgeplan_plugins_list`](/docs/mcp/forgeplan_plugins_list/) reports.

## CLI equivalent

- [`forgeplan plugins info`](/docs/cli/plugins-info/) - the same lookup printed as a field block, with an explicit `Installed: yes` / `no` line.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_plugins_list`](/docs/mcp/forgeplan_plugins_list/) - the installed / missing split across the whole registry
- [`forgeplan_plugins_doctor`](/docs/mcp/forgeplan_plugins_doctor/) - ok / outdated / missing with install hints in one pass
- [`forgeplan plugins info`](/docs/cli/plugins-info/) - terminal lookup
- [`forgeplan plugins`](/docs/cli/plugins/) - the CLI command family
