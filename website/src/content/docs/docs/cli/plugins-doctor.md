---
title: forgeplan plugins doctor
description: "Health check across known plugins - reports missing, outdated, OK"
---

Compares every plugin in the compiled-in registry against what is actually
installed, and prints the same scan [`forgeplan plugins list`](/docs/cli/plugins-list/)
performs as three groups: **ok**, **outdated**, and **missing**. It is the only
command in the family that exits non-zero, which is what makes it usable as a
gate - in CI, in a session-start check, or in a hook.

Every missing entry is printed with the exact command that would install it, so
the output is a to-do list you can paste rather than a diagnosis you have to
research.

## What it does

1. **Loads the registry** - the entries Forgeplan knows how to detect. In
   0.37.0 that is 12, counting the built-in `forgeplan` capability.
2. **Detects installed plugins** - the same filesystem scan as
   [`forgeplan plugins list`](/docs/cli/plugins-list/): source roots joined with
   the paths each entry expects, first match wins, version read best-effort
   from `manifest.json`, `plugin.json`, or `SKILL.md` frontmatter.
3. **Classifies** each registry entry into exactly one of the three groups
   using the rules in the table below.
4. **Prints the groups**, then a single `Fix:` line pointing at the first
   missing entry's install command.
5. **Exits 1** when anything is missing or outdated, `0` when the whole
   registry is accounted for.

## The three classifications

| Group | Marker | Rule |
|---|---|---|
| ok | `✓` | a path resolved and the detected version satisfies the registry's requirement |
| ok | `✓` | a path resolved and no version could be read - incompatibility cannot be proven, so it is not reported as a fault |
| outdated | `!` | a path resolved, a version was read, and that version does not satisfy the requirement |
| missing | `✗` | no candidate path resolved for the entry |

An unreadable semver - in the registry requirement or in a manifest - is also
surfaced as outdated rather than being swallowed: the `found` column shows the
detected version, or `?` when there is none.

## Usage

```text
forgeplan plugins doctor [OPTIONS]
```

## Options

```text
      --json     Output as JSON
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: the common case - packs not installed

```bash
forgeplan plugins doctor
```

```
  ✓ 1 ok, 11 missing, 0 outdated

    ✓ forgeplan                        0.37.0 (req *)

    ✗ c4-component                     C4 component-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows
    ✗ agents-pro                       Pro agent pack: ddd-domain-expert, agents-architect, agents-reviewer
        install: /plugin install agents-pro@ForgePlan-marketplace
    ✗ agent-orchestration-context-manager Context-manager agent (via agent-orchestration pack)
        install: /plugin install agent-orchestration@claude-code-workflows
    ✗ autoresearch                     Automated research workflows for greenfield kickoff
        install: /plugin install autoresearch@claude-code-workflows
    ✗ brownfield-docs-pack             Brownfield Obsidian/markdown vault migration skill pack
        install: forgeplan skill install brownfield-docs-pack
    ✗ sparc-specification              SPARC specification agent — pseudocode-driven spec writer
        install: /plugin install sparc-specification@ForgePlan-marketplace
    ✗ c4-context                       C4 context-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows
    ✗ c4-architecture                  C4 model diagrams (context/container/component/code) generator
        install: /plugin install c4-architecture@claude-code-workflows
    ✗ ddd-domain-expert                Domain-driven design domain-expert sub-agent (via agents-pro)
        install: /plugin install agents-pro@ForgePlan-marketplace
    ✗ agents-sparc-specification       SPARC specification agent (via agents-sparc pack)
        install: /plugin install agents-sparc@ForgePlan-marketplace
    ✗ c4-container                     C4 container-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows

Fix: /plugin install c4-architecture@claude-code-workflows
```

Exit code 1. Each missing entry gets two lines: the name with its registry
description, then the install command indented under it. The trailing `Fix:`
line repeats only the first of those commands - the complete list is the body.

### Example 2: an installed pack that does not satisfy its requirement

```bash
forgeplan plugins doctor
```

```
  ✓ 2 ok, 9 missing, 1 outdated

    ✓ brownfield-docs-pack             0.2.0 (req >=0.1)
    ✓ forgeplan                        0.37.0 (req *)

    ! c4-architecture                  found 0.5.0 expected >=1.0
```

Exit code 1. The `!` line is the outdated group: the detected `0.5.0` does not
satisfy the registry's `>=1.0`. `c4-architecture` is installed and it is still
a problem - this is the group a bare `list` cannot tell you about. The nine
missing entries follow in the same two-line shape as Example 1, then the
`Fix:` line.

### Example 3: a complete install

```bash
forgeplan plugins doctor
```

```
  ✓ 12 ok, 0 missing, 0 outdated

    ✓ ddd-domain-expert                1.2.0 (req >=1.0)
    ✓ agents-sparc-specification       1.2.0 (req >=1.0)
    ✓ c4-container                     1.2.0 (req >=1.0)
    ✓ c4-component                     1.2.0 (req >=1.0)
    ✓ agent-orchestration-context-manager 1.2.0 (req >=1.0)
    ✓ sparc-specification              1.5.0 (req >=1.0)
    ✓ brownfield-docs-pack             0.3.0 (req >=0.1)
    ✓ c4-context                       1.2.0 (req >=1.0)
    ✓ autoresearch                     1.5.0 (req >=1.0)
    ✓ forgeplan                        0.37.0 (req *)
    ✓ c4-architecture                  1.5.0 (req >=1.0)
    ✓ agents-pro                       1.5.0 (req >=1.0)

Done.
```

Exit code 0, and the closer is `Done.` instead of a `Fix:` line. In this run
every registry entry resolved on disk, version manifests included, so there is
nothing left to report - the shape a hook or a CI gate should see.

### Example 4: machine-readable

The text output groups entries visually; `--json` returns them as three arrays
plus the same next-action hint the text mode prints as `Fix:` or `Done.`. The
arrays are long on a machine with packs missing, so the useful views are
projections.

Verdict and hint:

```bash
forgeplan plugins doctor --json | jq '{_next_action, ok, outdated}'
```

```json
{
  "_next_action": "/plugin install c4-architecture@claude-code-workflows",
  "ok": [
    {
      "detected_version": "0.37.0",
      "expected": "*",
      "name": "forgeplan"
    },
    {
      "detected_version": "0.2.0",
      "expected": ">=0.1",
      "name": "brownfield-docs-pack"
    }
  ],
  "outdated": [
    {
      "expected": ">=1.0",
      "found": "0.5.0",
      "name": "c4-architecture"
    }
  ]
}
```

One missing entry, showing the fields each of them carries:

```bash
forgeplan plugins doctor --json | jq '.missing[0]'
```

```json
{
  "description": "C4 component-level diagram agent (via c4-architecture)",
  "install_command": "/plugin install c4-architecture@claude-code-workflows",
  "name": "c4-component"
}
```

Counts, as a one-line summary for a log or a badge:

```bash
forgeplan plugins doctor --json | jq '{ok:(.ok|length), missing:(.missing|length), outdated:(.outdated|length)}'
```

```json
{
  "ok": 2,
  "missing": 9,
  "outdated": 1
}
```

`ok` entries carry `name`, `detected_version`, and `expected`; `outdated`
entries carry `name`, `found`, and `expected`; `missing` entries carry `name`,
`description`, and `install_command`.

## Using it as a gate

The exit code is the contract:

```bash
forgeplan plugins doctor --json > /tmp/plugins.json
echo "doctor exit=$?"
```

```
doctor exit=1
```

The JSON lands in the file whether the run passed or failed, so a pipeline can
keep the artifact for a report and still fail the job on the exit code. Do not
depend on the position of an entry inside a group: the order is the registry's
iteration order and it changes between runs, so match by `name` when parsing.

## Notes

- No workspace is required. `doctor` reads the filesystem and the compiled-in
  registry only - nothing is installed, upgraded, or written.
- Only the first missing install command gets the `Fix:` line. To act on the
  whole list, read the body or the `--json` `missing` array.
- An entry that resolves but ships no version manifest counts as ok, so a
  version-less pack does not produce a false alarm.
- `-V` reports the subcommand-scoped version, `forgeplan-plugins-doctor 0.37.0`.

## See also

- [`forgeplan plugins`](/docs/cli/plugins/) - the parent command
- [`forgeplan plugins list`](/docs/cli/plugins-list/) - the same scan without the verdict
- [`forgeplan plugins info`](/docs/cli/plugins-info/) - details and install command for one entry
- [`forgeplan_plugins_doctor`](/docs/mcp/forgeplan_plugins_doctor/) - the MCP equivalent
- [`forgeplan health`](/docs/cli/health/) - the artifact-side counterpart to this environment check
