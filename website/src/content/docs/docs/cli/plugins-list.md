---
title: forgeplan plugins list
description: "List installed plugins (Claude / agent-skills / Cursor / Forgeplan)"
---

Enumerates the plugins that are actually present on this machine. `list` walks
the search roots of every detection source, matches what it finds against the
registry baked into the Forgeplan binary, and prints one row per match with the
source it came from, the version it reported, and the path that resolved.

It is the discovery half of the plugin family: [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/)
turns the same scan into a pass/fail verdict, and [`forgeplan plugins info`](/docs/cli/plugins-info/)
explains a single registry entry whether or not it is installed.

## What it does

1. **Loads the registry** - the compiled-in list of plugins Forgeplan knows how
   to detect. In 0.37.0 it holds 12 entries, including the built-in `forgeplan`
   capability itself.
2. **Builds candidate paths** for each entry: every search root of the entry's
   source, joined with every path the entry expects.
3. **Picks the first match.** A single registry entry yields at most one row,
   so a plugin found both in `~/.claude/plugins/cache/` and in a local
   `.claude/plugins/` is reported once.
4. **Reads a version, best effort.** `manifest.json` first, then `plugin.json`,
   then the YAML frontmatter of `SKILL.md`. An unreadable or missing version is
   not an error - the row simply has no version.
5. **Prints the table** and the next command to run.

Detection is registry-driven. A directory that looks like a plugin but has no
registry entry is not reported, and a registry entry with no matching path is
not reported either - that half lives in
[`forgeplan plugins doctor`](/docs/cli/plugins-doctor/).

## Detection sources

The source label in the table is the lowercased name of the registry entry's
source (`claudeplugin`, `agentskills`, `cursor`, `forgeplan`, `manual`). The
roots each source is scanned in:

| Source label | Roots scanned |
|---|---|
| `claudeplugin` | `~/.claude/plugins/cache/`, `.claude/plugins/` |
| `agentskills` | `.agentskills/`, `~/.agentskills/` |
| `cursor` | `.cursor/skills/` |
| `forgeplan` | none - the built-in capability, always reported |
| `manual` | none - user-supplied entries, never auto-detected |

Three of those roots are relative to the current directory (`.claude/plugins/`,
`.agentskills/`, `.cursor/skills/`), which is what makes a project-local
install visible when you run `list` from the project root. Running the same
command from elsewhere can therefore report a different set - no workspace is
required for any of it.

No registry entry in 0.37.0 uses the `cursor` or `manual` source; the labels
exist for entries shipped by later releases and for project-local registry
overrides.

## Usage

```text
forgeplan plugins list [OPTIONS]
```

## Options

```text
      --json     Output as JSON
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: only the built-in capability present

```bash
forgeplan plugins list
```

```
  ✓ 1 plugin(s) installed

  Name                          Source          Version     Path
  --------------------------------------------------------------------------------
  forgeplan                     forgeplan       0.37.0      (built-in)

Next: forgeplan plugins doctor
```

`forgeplan` itself is always reported, with the path `(built-in)` and the
version of the running binary, so this is the floor - a machine with zero
installed packs still shows one row.

### Example 2: a machine with packs installed

```bash
forgeplan plugins list
```

```
  ✓ 3 plugin(s) installed

  Name                          Source          Version     Path
  --------------------------------------------------------------------------------
  c4-architecture               claudeplugin    0.5.0       .claude/plugins/c4-architecture
  forgeplan                     forgeplan       0.37.0      (built-in)
  brownfield-docs-pack          agentskills     0.2.0       .agentskills/brownfield-docs-pack
```

Three sources, three paths: a Claude plugin installed under the project, the
built-in capability, and an agent-skills pack found through `.agentskills/`.
The order of the rows is the registry's iteration order, not sorted by name or
by source.

### Example 3: the whole registry installed

```bash
forgeplan plugins list
```

```
  ✓ 12 plugin(s) installed

  Name                          Source          Version     Path
  --------------------------------------------------------------------------------
  autoresearch                  claudeplugin    1.5.0       .claude/plugins/autoresearch
  c4-context                    claudeplugin    1.2.0       .claude/plugins/c4-architecture/agents/c4-context
  forgeplan                     forgeplan       0.37.0      (built-in)
  brownfield-docs-pack          agentskills     0.3.0       .agentskills/brownfield-docs-pack
  agents-pro                    claudeplugin    1.5.0       .claude/plugins/agents-pro
  c4-container                  claudeplugin    1.2.0       .claude/plugins/c4-architecture/agents/c4-container
  agents-sparc-specification    claudeplugin    1.2.0       .claude/plugins/agents-sparc/agents/specification
  ddd-domain-expert             claudeplugin    1.2.0       .claude/plugins/agents-pro/agents/ddd-domain-expert
  agent-orchestration-context…  claudeplugin    1.2.0       .claude/plugins/agent-orchestration/agents/context-manager
  sparc-specification           claudeplugin    1.5.0       .claude/plugins/sparc-specification
  c4-component                  claudeplugin    1.2.0       .claude/plugins/c4-architecture/agents/c4-component
  c4-architecture               claudeplugin    1.5.0       .claude/plugins/c4-architecture

Next: forgeplan plugins doctor
```

Two details worth reading off this table: sub-agent entries such as `c4-context`
resolve to their nested path inside the parent pack, and a name longer than 28
characters is truncated with an ellipsis (`agent-orchestration-context…`). The
full name is in the `--json` output.

### Example 4: machine-readable

```bash
forgeplan plugins list --json
```

```json
{
  "_next_action": "forgeplan plugins doctor",
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
  "registry_size": 12
}
```

Each `installed` entry carries the detection result (`detected_path`,
`detected_version`) next to the full registry record it matched. `registry_size`
is how many entries the binary knows about - the denominator that makes the
doctor's missing count meaningful.

## Exit codes

`list` exits 0 whether or not anything was detected; it is an inventory, not a
gate. The non-zero exit code for an incomplete setup lives in
[`forgeplan plugins doctor`](/docs/cli/plugins-doctor/).

## Notes

- When nothing is detected, the summary line reads `⊘ no plugins detected
  (registry knows <n> entries)` and the hint becomes `Or: forgeplan plugins
  doctor`. The built-in `forgeplan` entry is registered unconditionally by the
  scanner, so this branch is only reachable while that entry is absent from the
  registry.
- `list` reads the filesystem only. It never installs, updates, or removes
  anything, and it never contacts the marketplace or the network.
- Versions are best effort. A pack that ships no `manifest.json`, no
  `plugin.json`, and no `SKILL.md` frontmatter is listed without a version, and
  [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/) counts it as OK
  rather than outdated.
- `-V` reports the subcommand-scoped version, `forgeplan-plugins-list 0.37.0`.

## See also

- [`forgeplan plugins`](/docs/cli/plugins/) - the parent command
- [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/) - ok / outdated / missing health check
- [`forgeplan plugins info`](/docs/cli/plugins-info/) - details for one registry entry
- [`forgeplan_plugins_list`](/docs/mcp/forgeplan_plugins_list/) - the MCP equivalent
- [CLI overview](/docs/cli/)
