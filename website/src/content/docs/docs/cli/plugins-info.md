---
title: forgeplan plugins info
description: "Show details for a specific plugin (path, version, description)"
---

Looks up one plugin by name in the compiled-in registry and reports everything
Forgeplan knows about it: the source it is detected through, the version
requirement it is checked against, the description, the install command, and
whether it is present on this machine right now.

It is the per-entry view that [`forgeplan plugins list`](/docs/cli/plugins-list/)
gives you as a table and [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/)
gives you as a verdict. `info` answers "what is this plugin, and what would I
type to get it" for a name you already have - the registry holds 12 entries in
0.37.0.

## Forgeplan 0.38.0

The detection registry compiled into CLI 0.38.0 has 12 entries and is not the public marketplace catalog. Public catalog 1.187.1 at b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7 has 23 plugins. Output below containing 0.37.0 is retained as previously captured examples, not a current installation inventory. The bundled hint `forgeplan skill install brownfield-docs-pack` is not a working CLI 0.38.0 command: no skill install command exists. Other install strings are static recommendations, not proof that a pack exists in the public catalog; consult that catalog and the selected plugin’s instructions.

## What it does

1. **Looks the name up in the registry.** This is an exact, case-sensitive match
   against the compiled-in entry names. An unknown name exits 2 with
   `Or: forgeplan plugins list` and no further work.
2. **Scans the filesystem** with the same detection as `list` - source roots
   joined with the entry's expected paths, first match wins, version read
   best-effort from `manifest.json`, `plugin.json`, or `SKILL.md` frontmatter.
3. **Prints the registry block**: name, source, version requirement,
   description, install command, and the expected paths when the entry declares
   them.
4. **Prints the detection block**: `Installed: yes` with the resolved path and
   version, or `Installed: no`.
5. **Prints the hint**: `Done.` when the plugin is installed, otherwise
   `Next: <install command>`. This is a compiled-in hint, not a verified working installation recipe; check the current catalog before using it.

## Fields

| Field | Meaning |
|---|---|
| `Name` | the registry key you must pass as `NAME` |
| `Source` | how it is detected - `ClaudePlugin`, `AgentSkills`, `Cursor`, `Forgeplan`, or `Manual` in text mode |
| `Version req` | the semver requirement the doctor compares against, for example `>=1.0` or `*` |
| `Description` | the registry's one-line summary |
| `Install` | static installation recommendation (`(built-in)` for Forgeplan itself), not execution or availability proof |
| `Expected paths` | printed only when the entry declares paths; each is resolved under the source's roots |
| `Installed` | the detection result for this machine |
| `Path` | where detection found it, relative when the match came from a project-local root |
| `Version` | printed only when a manifest supplied one |

`info` reports install state, not compatibility. A plugin installed at `0.5.0`
against a `>=1.0` requirement still prints `Installed: yes` and `Done.` - the
version verdict belongs to [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/).

## Usage

```text
forgeplan plugins info [OPTIONS] <NAME>
```

## Arguments

```text
  <NAME>  Plugin name (e.g. c4-architecture)
```

## Options

```text
      --json     Output as JSON
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: the built-in capability

```bash
forgeplan plugins info forgeplan
```

```
  Name:        forgeplan
  Source:      Forgeplan
  Version req: *
  Description: Forgeplan core methodology engine (always present)
  Install:     (built-in)

  Installed:   yes
  Path:        (built-in)
  Version:     0.37.0

Done.
```

No `Expected paths` block, because the built-in entry is not scanned on disk -
it is always present, and the version is the version of the running binary.

### Example 2: a registry entry that is not installed

```bash
forgeplan plugins info c4-architecture
```

```
  Name:        c4-architecture
  Source:      ClaudePlugin
  Version req: >=1.0
  Description: C4 model diagrams (context/container/component/code) generator
  Install:     /plugin install c4-architecture@claude-code-workflows
  Expected paths:
    - c4-architecture

  Installed:   no

Next: /plugin install c4-architecture@claude-code-workflows
```

Exit code 0 - "not installed" is information, not an error. The `Next:` line is
the same command [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/) repeats
in its `Fix:` line and in each missing entry's `install:` row.

### Example 3: a registry entry that is installed

```bash
forgeplan plugins info c4-architecture
```

```
  Name:        c4-architecture
  Source:      ClaudePlugin
  Version req: >=1.0
  Description: C4 model diagrams (context/container/component/code) generator
  Install:     /plugin install c4-architecture@claude-code-workflows
  Expected paths:
    - c4-architecture

  Installed:   yes
  Path:        .claude/plugins/c4-architecture
  Version:     0.5.0

Done.
```

The desktop-skill variant of the same question shows the other detection source
and the other install channel:

```bash
forgeplan plugins info brownfield-docs-pack
```

```
  Name:        brownfield-docs-pack
  Source:      AgentSkills
  Version req: >=0.1
  Description: Brownfield Obsidian/markdown vault migration skill pack
  Install:     forgeplan skill install brownfield-docs-pack
  Expected paths:
    - brownfield-docs-pack

  Installed:   yes
  Path:        .agentskills/brownfield-docs-pack
  Version:     0.2.0

Done.
```

The sample path is project-local. Detection also searches user-global AgentSkills roots, so another installation can remain visible outside that project.

### Example 4: machine-readable

```bash
forgeplan plugins info c4-architecture --json
```

```json
{
  "_next_action": "Done.",
  "info": {
    "description": "C4 model diagrams (context/container/component/code) generator",
    "expected_paths": [
      "c4-architecture"
    ],
    "install_command": "/plugin install c4-architecture@claude-code-workflows",
    "name": "c4-architecture",
    "source": "claude-plugin",
    "version_req": ">=1.0"
  },
  "installed": {
    "detected_path": ".claude/plugins/c4-architecture",
    "detected_version": "0.5.0",
    "info": {
      "description": "C4 model diagrams (context/container/component/code) generator",
      "expected_paths": [
        "c4-architecture"
      ],
      "install_command": "/plugin install c4-architecture@claude-code-workflows",
      "name": "c4-architecture",
      "source": "claude-plugin",
      "version_req": ">=1.0"
    }
  }
}
```

Two differences from the text form matter when scripting: `source` is the
kebab-case string (`claude-plugin`, `agent-skills`) instead of the CamelCase
Rust variant name, and `installed` is `null` rather than an object when nothing
was detected. The registry record is repeated inside `installed` because the
detection result embeds the entry it matched.

### Example 5: an unknown name

```bash
forgeplan plugins info no-such-plugin
```

```
Error: plugin 'no-such-plugin' not in registry
Or: forgeplan plugins list
```

Exit code 2. The match is case-sensitive, so a mis-cased name fails the same
way:

```bash
forgeplan plugins info C4-Architecture
```

```
Error: plugin 'C4-Architecture' not in registry
Or: forgeplan plugins list
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | the name is in the registry, whether or not it is installed |
| `2` | the name is not in the registry |

## Notes

- No workspace is required; the command reads the registry and the filesystem.
- `info` never installs anything and never contacts the marketplace or the
  network. The description and install command it prints are the ones compiled
  into the binary, not anything fetched.
- A registry entry that another entry already covers - a sub-agent of a pack,
  for example - has its own row and its own expected paths, so
  `info c4-context` answers for the sub-agent rather than for the pack.
- The retained earlier capture uses `forgeplan-plugins-info 0.37.0`; the published binary version is 0.38.0.

## See also

- [`forgeplan plugins`](/docs/cli/plugins/) - the parent command
- [`forgeplan plugins list`](/docs/cli/plugins-list/) - every detected plugin as a table
- [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/) - ok / outdated / missing across the registry
- [`forgeplan_plugins_info`](/docs/mcp/forgeplan_plugins_info/) - the MCP equivalent
- [CLI overview](/docs/cli/)
