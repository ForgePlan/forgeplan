---
title: forgeplan_plugins_doctor
description: "Health-check across the full plugin registry: separates ok / outdated / missing entries and surfaces install hints for the missing ones. PRD-067 AC-2."
---

Health-check across the full plugin registry: separates ok / outdated / missing entries and surfaces install hints for the missing ones. PRD-067 AC-2.

**Category**: Other

The triage tool for a broken environment. Where [`forgeplan_plugins_list`](/docs/mcp/forgeplan_plugins_list/) answers "what is here", this tool answers "what is wrong, and what do I paste to fix it". Every entry in the extended registry is classified into exactly one of three buckets, and every non-`ok` entry contributes a runnable install command to `install_hints`.

## What it does

1. Builds the extended registry: the built-in plugin defaults plus the curated extras the registry carries.
2. Detects what is actually installed on disk by scanning the Claude plugin directories, agent-skill directories, and Cursor locations.
3. Classifies each registry entry:
   - **ok** - installed, and the detected version satisfies the registry's version requirement.
   - **outdated** - installed, but the detected version fails the requirement.
   - **missing** - not detected at all.
4. Collects the `install_command` of every `outdated` and `missing` entry into `install_hints`, in registry order. The same command can appear more than once when several registry entries ship in one pack - four C4 agent entries all point at `/plugin install c4-architecture@claude-code-workflows`.
5. Returns the three buckets, the hint list, and the three counts.

## When an agent calls this

- Start of a session in an unfamiliar environment: one call tells you which capabilities are installed before you plan around them.
- After a step failed with a "delegate not found" or missing-skill error - the playbook's `fallback_hint` names the plugin, this tool confirms whether it is genuinely absent.
- Before recommending a playbook from the recommendation engine: the recommended playbook's prerequisites are plugins, and `install_hints` is what the user has to run.
- After an install, to confirm the entry moved from `missing` to `ok`.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| (none) | - | - | The tool takes no parameters. It is read-only, idempotent, non-destructive, and closed-world. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::forgeplan_plugins_doctor` (signature `Parameters(_p): Parameters<EmptyParams>`)_

## Returns

| Field | Type | Description |
|---|---|---|
| `ok` | `array` | Entries that are installed and version-compatible. |
| `outdated` | `array` | Entries that are installed but fail their version requirement. |
| `missing` | `array` | Registry entries not detected on disk. |
| `install_hints` | `array` | Install command for every `outdated` and `missing` entry, in registry order. May contain duplicates. |
| `ok_count` | `number` | Length of `ok`. |
| `outdated_count` | `number` | Length of `outdated`. |
| `missing_count` | `number` | Length of `missing`. |
| `_next_action` | `string` | With at least one hint: `Run install hint: <first hint>. Then re-check: forgeplan_plugins_doctor`. With none: `Done.` |

### Entry shapes

The three buckets carry different key sets - `outdated` uses `required` where the other two use `version_req`.

| Bucket | Keys |
|---|---|
| `ok` | `name`, `detected_version`, `version_req`, `path` |
| `outdated` | `name`, `detected_version`, `required`, `path` |
| `missing` | `name`, `version_req`, `description` |

## Examples

Triage a fresh machine - one entry is healthy, eleven are absent:

```json
{}
```

Response (captured live, with the `missing` array trimmed to three entries):

```json
{
  "_next_action": "Run install hint: `/plugin install c4-architecture@claude-code-workflows`. Then re-check: `forgeplan_plugins_doctor`.",
  "install_hints": [
    "/plugin install c4-architecture@claude-code-workflows",
    "/plugin install c4-architecture@claude-code-workflows",
    "/plugin install autoresearch@claude-code-workflows",
    "/plugin install agents-pro@ForgePlan-marketplace",
    "/plugin install agents-sparc@ForgePlan-marketplace",
    "/plugin install c4-architecture@claude-code-workflows",
    "/plugin install sparc-specification@ForgePlan-marketplace",
    "/plugin install agents-pro@ForgePlan-marketplace",
    "forgeplan skill install brownfield-docs-pack",
    "/plugin install c4-architecture@claude-code-workflows",
    "/plugin install agent-orchestration@claude-code-workflows"
  ],
  "missing": [
    {
      "description": "C4 context-level diagram agent (via c4-architecture)",
      "name": "c4-context",
      "version_req": ">=1.0"
    },
    {
      "description": "Automated research workflows for greenfield kickoff",
      "name": "autoresearch",
      "version_req": ">=1.0"
    },
    {
      "description": "Brownfield Obsidian/markdown vault migration skill pack",
      "name": "brownfield-docs-pack",
      "version_req": ">=0.1"
    }
  ],
  "missing_count": 11,
  "ok": [
    {
      "detected_version": "<detected on disk>",
      "name": "forgeplan",
      "path": "(built-in)",
      "version_req": "*"
    }
  ],
  "ok_count": 1,
  "outdated": [],
  "outdated_count": 0
}
```

`detected_version` is read from the installed manifest during the scan, so it reflects the plugin on disk rather than the binary serving the request - which is why it is shown as a placeholder here. The built-in `forgeplan` entry resolves to `(built-in)` with the requirement `*`.

The same check from the terminal, which renders the buckets as a one-line summary plus one line per problem:

```text
$ forgeplan plugins doctor
  ✓ 1 ok, 11 missing, 0 outdated

    ✓ forgeplan                        0.37.0 (req *)

    ✗ agents-sparc-specification       SPARC specification agent (via agents-sparc pack)
        install: /plugin install agents-sparc@ForgePlan-marketplace
    ✗ brownfield-docs-pack             Brownfield Obsidian/markdown vault migration skill pack
        install: forgeplan skill install brownfield-docs-pack

Fix: /plugin install agents-sparc@ForgePlan-marketplace
```

## Notes

- **Offline and deterministic.** Detection is a filesystem scan against a static registry. No network call, no package manager, no LLM.
- **An unparseable version is treated as ok.** Compatibility is checked with the registry's version requirement; when either side of that comparison cannot be parsed as semver the entry lands in `ok` rather than `outdated`. An entry whose manifest publishes no version at all is `outdated`, not `ok`.
- **`install_hints` is the actionable part.** It is a flat list, deduplicated only by accident of the registry - filter it yourself before showing a user eleven lines that are really four distinct commands.
- **Bucket and hint order is not stable.** The registry is iterated as an unordered map, so `missing`, `outdated`, and `install_hints` come back in a different order on successive calls - and `_next_action` names whichever hint happened to land first. Sort by `name` before rendering, and never treat `install_hints[0]` as a fixed answer.
- **A `missing` entry is not an error.** The registry is a catalogue of what ForgePlan *can* use; most workspaces legitimately have a subset installed. Only treat a missing plugin as a problem when a playbook or workflow you intend to run requires it.
- Install commands carry an explicit `@marketplace` suffix where applicable. The suffix is required - without it the install is ambiguous when two marketplaces publish a plugin with the same name.

## Related commands

- [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/) - the CLI equivalent, with the human-rendered summary.
- [`forgeplan_plugins_list`](/docs/mcp/forgeplan_plugins_list/) - the full inventory, installed and missing.
- [`forgeplan_plugins_info`](/docs/mcp/forgeplan_plugins_info/) - details for a single registry entry.
- [`forgeplan_playbook_run`](/docs/mcp/forgeplan_playbook_run/) - the consumer of the plugins this tool audits.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_plugins_list`](/docs/mcp/forgeplan_plugins_list/) - inventory before triage
- [`forgeplan_plugins_info`](/docs/mcp/forgeplan_plugins_info/) - one entry in detail
- [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/) - which playbooks the installed packs provide
- [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/) - the CLI twin
