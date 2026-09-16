---
title: forgeplan plugins
description: "Plugin detection + recommendations (PRD-067) - list installed plugins, doctor health-check, info on specific plugin"
---

`forgeplan plugins` is the parent for plugin detection and recommendation (PRD-067). It surfaces which plugins are installed on this machine, which ones the registry knows about but you do not have, and what each one does. The command groups detection by source - Claude Code, the agentskills.io standard, Cursor skills, and the built-in forgeplan core - so you can see at a glance which ecosystem a plugin came from.

The parent command requires a subcommand; running it bare exits with code `2` and prints the help banner.

## Subcommands

- [`plugins list`](/docs/cli/plugins-list/) - list every installed plugin (Claude / agent-skills / Cursor / Forgeplan), one row per detection.
- [`plugins doctor`](/docs/cli/plugins-doctor/) - health check across the whole registry: which known plugins are installed, which are missing, which are outdated. Emits an exact install command per missing entry.
- [`plugins info <NAME>`](/docs/cli/plugins-info/) - details for a single plugin: name, source, version requirement, description, install command, and - when installed - the detected path and version.

Each subcommand also accepts `--json` for scripting and CI.

## Usage

```text
forgeplan plugins <COMMAND>
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Subcommands

```text
  list    List installed plugins (Claude / agent-skills / Cursor / Forgeplan)
  doctor  Health check across known plugins - reports missing, outdated, OK
  info    Show details for a specific plugin (path, version, description)
  help    Print this message or the help of the given subcommand(s)
```

## What it does

Detection walks a small set of canonical search paths per source:

- Claude Code plugins: `~/.claude/plugins/cache/` and `.claude/plugins/`.
- agentskills.io: `.agentskills/` and `~/.agentskills/`.
- Cursor skills: `.cursor/skills/`.
- Forgeplan core: built into the binary, registered with a synthetic `(built-in)` path.

A candidate manifest is found by joining each search root with the registry entry's `expected_paths`. The first match wins (a plugin found in two places is reported once). When a candidate directory is found, the scanner reads `manifest.json`, `plugin.json`, or `SKILL.md` (YAML frontmatter `version: ...`) for a version string. A missing or unparseable version is not an error - it just blocks the compatibility check.

The built-in `forgeplan` core plugin is always registered as installed, so `plugins list` always has at least one row. Manual / out-of-band installs are tracked but not auto-detected.

## When to use

- At session start, to confirm the plugins you expect to be available are actually available.
- After upgrading the binary or installing a marketplace plugin, to verify detection picked it up.
- Before a sprint, to plan which missing plugins to install (`plugins doctor` is the canonical pre-sprint check).
- When troubleshooting an agent that "knows about" a plugin but cannot find it.

## When NOT to use

- To install a plugin. The command only reports; use the marketplace install command surfaced by `plugins doctor` or `plugins info`.
- To detect skills outside the four known sources. Out-of-band installs stay invisible to the scanner.

## Examples

### Example 1: Parent command without a subcommand

```bash
forgeplan plugins
```

```
Plugin detection + recommendations (PRD-067) - list installed plugins, doctor health-check, info on specific plugin

Usage: forgeplan plugins <COMMAND>

Commands:
  list    List installed plugins (Claude / agent-skills / Cursor / Forgeplan)
  doctor  Health check across known plugins - reports missing, outdated, OK
  info    Show details for a specific plugin (path, version, description)
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

Exit code `2` - a subcommand is required.

### Example 2: `plugins list` (text)

```bash
forgeplan plugins list
```

```
  v 1 plugin(s) installed

  Name                          Source          Version     Path
  --------------------------------------------------------------------------------
  forgeplan                     forgeplan       0.37.0      (built-in)

Next: forgeplan plugins doctor
```

Exit code `0`. The `Next:` hint follows PRD-071 and points at `plugins doctor`, the canonical follow-up.

### Example 3: `plugins list --json`

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

`registry_size` is the number of entries in the extended registry (1 ok + 11 missing in this example).

### Example 4: `plugins doctor`

```bash
forgeplan plugins doctor
```

```
  v 1 ok, 11 missing, 0 outdated

    v forgeplan                        0.37.0 (req *)

    X sparc-specification              SPARC specification agent - pseudocode-driven spec writer
        install: /plugin install sparc-specification@ForgePlan-marketplace
    X c4-architecture                  C4 model diagrams (context/container/component/code) generator
        install: /plugin install c4-architecture@claude-code-workflows
    X autoresearch                     Automated research workflows for greenfield kickoff
        install: /plugin install autoresearch@claude-code-workflows
    X c4-container                     C4 container-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows
    X agents-pro                       Pro agent pack: ddd-domain-expert, agents-architect, agents-reviewer
        install: /plugin install agents-pro@ForgePlan-marketplace
    X c4-component                     C4 component-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows
    X brownfield-docs-pack             Brownfield Obsidian/markdown vault migration skill pack
        install: forgeplan skill install brownfield-docs-pack
    X ddd-domain-expert                Domain-driven design domain-expert sub-agent (via agents-pro)
        install: /plugin install agents-pro@ForgePlan-marketplace
    X agents-sparc-specification       SPARC specification agent (via agents-sparc pack)
        install: /plugin install agents-sparc@ForgePlan-marketplace
    X c4-context                       C4 context-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows
    X agent-orchestration-context-manager Context-manager agent (via agent-orchestration pack)
        install: /plugin install agent-orchestration@claude-code-workflows

Fix: /plugin install sparc-specification@ForgePlan-marketplace
```

Exit code `1` when at least one known plugin is missing or outdated; `0` when everything is OK. Each missing row carries an exact install command so a CI gate or a human can act without re-reading the docs.

### Example 5: `plugins info <NAME>` (installed)

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

Exit code `0`.

### Example 6: `plugins info <NAME>` (registered but not installed)

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

Exit code `0`. The `Next:` hint is the exact install command, copy-paste-ready.

### Example 7: `plugins info <NAME>` for an unknown name

```bash
forgeplan plugins info nope-not-real
```

```
Error: plugin 'nope-not-real' not in registry
Or: forgeplan plugins list
```

Exit code `2`. The `Or:` hint points at `plugins list` so the caller can discover the correct name.

## Source taxonomy

Four sources are recognised, with these canonical search paths (HOME-relative where applicable):

| Source | Default search paths |
|---|---|
| `ClaudePlugin` | `~/.claude/plugins/cache/`, `.claude/plugins/` |
| `AgentSkills` | `.agentskills/`, `~/.agentskills/` |
| `Cursor` | `.cursor/skills/` |
| `Forgeplan` | (built-in - not scanned) |
| `Manual` | (no auto-detect) |

The `Forgeplan` and `Manual` sources return no search paths. `Forgeplan` is the synthetic entry for the binary itself; `Manual` exists for entries that arrive out-of-band and are tracked but not auto-detected.

## Exit codes

- `0` - the operation succeeded (a healthy `doctor` reports no missing/outdated plugins; `list` and `info` always exit `0` on success).
- `1` - `doctor` found at least one missing or outdated plugin.
- `2` - usage error (parent command with no subcommand; `info` called with a name not in the registry).

## Notes

- The registry is bundled in the binary (`extended_registry`), so detection works offline and on first boot.
- A plugin found in two places (e.g. both `~/.claude/plugins/cache/foo` and `.claude/plugins/foo`) is reported once. The first matching path wins.
- Version parsing is best-effort: `manifest.json` is checked first, then `plugin.json`, then `SKILL.md` YAML frontmatter. A missing or unparseable version is recorded as `None` and does not raise.
- Tilde (`~`) in search paths expands to `$HOME` if set; an unset HOME leaves the literal `~` and the scan skips that root.
- Hidden directories (those starting with `.`) are descended into for the same reason as the rest of the toolchain: layouts like `.local/spike-*.md` are reachable.

## Related commands

- [`forgeplan plugins list`](/docs/cli/plugins-list/) - the per-source listing subcommand.
- [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/) - the missing/outdated health check.
- [`forgeplan plugins info`](/docs/cli/plugins-info/) - the single-plugin detail page.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan plugins list`](/docs/cli/plugins-list/) - installed plugins
- [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/) - missing/outdated health check
- [`forgeplan plugins info`](/docs/cli/plugins-info/) - single-plugin details
- [Marketplace overview](/docs/marketplace/overview/) - where the plugin packs come from
