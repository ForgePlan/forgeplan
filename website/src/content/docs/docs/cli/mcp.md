---
title: forgeplan mcp
description: "Parent command for MCP integration helpers — install Forgeplan into Claude Code, Cursor, or Windsurf, and start the MCP server."
---

`forgeplan mcp` groups the helpers an AI-agent client needs to talk to Forgeplan over the
Model Context Protocol. It is **not** a tool you call from agents — its subcommands run on
the host machine to wire the binary into client config files (`mcp install`) or to start
the stdio server when launched manually (`mcp serve`, an alias for [`forgeplan serve`](/docs/cli/serve/)).

Forgeplan is MCP-first: most of the day-to-day surface area (63 tools) is exposed via
the server. This parent command exists so a one-shot `forgeplan mcp install --client claude`
gets you from a fresh `brew install forgeplan` to a working agent without hand-editing JSON.

## When to use

- Right after installing Forgeplan and before the first `claude` / `cursor` session.
- When you upgrade Forgeplan via Homebrew and the absolute path in `.mcp.json` becomes
  stale (`forgeplan mcp install` re-detects it).
- When debugging integration: `forgeplan mcp serve` runs the same server as `forgeplan serve`
  so JSON-RPC traffic can be inspected with `mcp-inspector`.

## When NOT to use

- For day-to-day artifact work — that is what the agent-side MCP tools (`forgeplan_*`) are for.
- For HTTP / network exposure — `mcp` only covers stdio. Forgeplan is local-first.

## Usage

```text
forgeplan mcp <COMMAND>
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Subcommands

| Command | Purpose |
|---|---|
| [`install`](/docs/cli/mcp-install/) | Smart-merge Forgeplan into a client config (Claude / Cursor / Windsurf) |
| [`serve`](/docs/cli/mcp-serve/) | Alias for [`forgeplan serve`](/docs/cli/serve/) — starts the stdio MCP server |
| `help` | Print help for `mcp` or its subcommands |

## Examples

### Onboard a fresh Claude Code install

```bash
forgeplan mcp install --client claude --scope user
```

Writes Forgeplan into `~/.claude.json` so every Claude Code session sees `mcp__forgeplan__*`
tools. Idempotent — safe to re-run after upgrades.

### Project-scoped Cursor config

```bash
forgeplan mcp install --client cursor --scope project
```

Creates `./.cursor/mcp.json` so the Forgeplan server only runs when this repo is open.
Ideal for monorepos where some projects use Forgeplan and others do not.

### Manually run the server (debugging only)

```bash
cd /path/to/project
forgeplan mcp serve
```

Same effect as `forgeplan serve`. Useful when piping JSON-RPC by hand or attaching
`mcp-inspector`.

## How it fits the workflow

`mcp install` is a one-time setup step that lives between "binary on disk" and "agent
can call tools". After it succeeds, the rest of the methodology (Shape → Validate → Code
→ Evidence → Activate) runs through the MCP tools the server exposes. `mcp serve` is the
runtime; you almost never invoke it manually because the client launches it for you.

## See also

- [`forgeplan mcp install`](/docs/cli/mcp-install/) — wire Forgeplan into a client
- [`forgeplan mcp serve`](/docs/cli/mcp-serve/) — alias for `forgeplan serve`
- [`forgeplan serve`](/docs/cli/serve/) — primary reference for the MCP server
- [MCP tools index](/docs/mcp/) — what the server exposes
- [`forgeplan health`](/docs/cli/health/) — verify the workspace before connecting
