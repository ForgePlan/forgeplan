---
title: forgeplan discover start
description: "Start a named brownfield discovery session and print the seven-phase protocol for an agent. This command creates session state and instructions, not a scan or an automatic backlog."
---

Start a named brownfield discovery session and print the seven-phase protocol for an agent. This command creates session state and instructions, not a scan or an automatic backlog.

## Usage

```text
forgeplan discover start <NAME>
```

## Arguments

```text
  <NAME>  Project name for the discovery session
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Example

```bash
forgeplan discover start "Auth System Onboarding"
# Follow the protocol and use the returned session ID:
forgeplan discover show SESSION_ID
forgeplan discover complete SESSION_ID
forgeplan health
```

## Protocol and hand-off

Phases are detect, structure, code, git, tests, docs, synthesize. The protocol describes source tiers and reporting through `forgeplan_discover_finding` on MCP. A person or agent must execute the survey. CLI completion records status and totals; it does not generate artifact recommendations or invoke health. Review findings and create selected artifacts explicitly.

## See also

- [`forgeplan discover`](/docs/cli/discover/) - parent command and protocol overview
- [`forgeplan discover show`](/docs/cli/discover-show/) - inspect a running session
- [`forgeplan discover list`](/docs/cli/discover-list/) - all sessions
- [`forgeplan discover complete`](/docs/cli/discover-complete/) - record completion and finding totals
- [`forgeplan init`](/docs/cli/init/) - workspace bootstrap step before discover
