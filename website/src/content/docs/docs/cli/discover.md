---
title: forgeplan discover
description: "Manage a protocol-driven brownfield discovery session. start creates a session and prints instructions; a human or agent must actually read the code, Git history, tests, and documentation and report findings. The CLI does not autonomously inspect or implement the project."
---

Manage a protocol-driven brownfield discovery session. `start` creates a session and prints instructions; a human or agent must actually read the code, Git history, tests, and documentation and report findings. The CLI does not autonomously inspect or implement the project.

## Usage

```text
forgeplan discover <COMMAND>
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Subcommands

```text
  start     Start a new discovery session — prints protocol for AI agent
  list      List all discovery sessions in the workspace
  show      Show status of a discovery session
  complete  Mark a discovery session as completed
  help      Print this message or the help of the given subcommand(s)
```

## Session workflow

`start NAME` starts a session; `list` lists saved sessions; `show SESSION_ID` displays findings; `complete SESSION_ID` records completion. The parent requires a subcommand. MCP additionally exposes `forgeplan_discover_finding` to report findings; no matching CLI finding command exists.

## Example

```bash
forgeplan init -y
forgeplan discover start "Legacy API"
# Follow the printed protocol and report findings through MCP.
forgeplan discover list
# Replace SESSION_ID with the actual identifier:
forgeplan discover show SESSION_ID
forgeplan discover complete SESSION_ID
forgeplan health
```

## Protocol and hand-off

The printed protocol has seven phases: detect, structure, code, git, tests, docs, synthesize. Source tiers distinguish authoritative, extracted, and supplementary material. Review contradictions against actual behavior rather than treating old docs as truth. CLI completion only saves status and prints totals; it does not run health or generate artifact proposals. Review findings and explicitly create chosen Problems, PRDs, ADRs, or Notes with `new`.

## See also

- [`forgeplan discover start`](/docs/cli/discover-start/) - start a session + print protocol
- [`forgeplan discover list`](/docs/cli/discover-list/) - all sessions in the workspace
- [`forgeplan discover show`](/docs/cli/discover-show/) - inspect a session's findings
- [`forgeplan discover complete`](/docs/cli/discover-complete/) - save completion and print totals
- [Methodology guide](/docs/methodology/overview/)
- [`forgeplan health`](/docs/cli/health/) - before/after comparison
