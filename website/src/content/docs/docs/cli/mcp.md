---
title: forgeplan mcp
description: "Parent command for MCP integration helpers - install Forgeplan into Claude Code, Cursor, or Windsurf, and start the MCP server."
---

`forgeplan mcp` groups the helpers an AI-agent client needs to talk to Forgeplan over the
Model Context Protocol. It is **not** a tool you call from agents - its subcommands run on
the host machine to wire the binary into client config files (`mcp install`) or to start
the stdio server when launched manually (`mcp serve`, an alias for [`forgeplan serve`](/docs/cli/serve/)).

The selected `0.39.0+constellation.s1.1` binary exposes 76 tools in the full profile and 9 in the additional read-only profile. Those are captured inventories, not acceptance of every operation or a global write ban. The installed `0.38.0` release is unchanged. `mcp install` configures a client; successful initialization and calls still need verification in that client.

## When to use

- Right after installing Forgeplan and before the first `claude` / `cursor` session.
- When you upgrade Forgeplan via Homebrew and the absolute path in `.mcp.json` becomes
  stale (`forgeplan mcp install` re-detects it).
- When debugging integration: `forgeplan mcp serve` runs the same server as `forgeplan serve`
  so JSON-RPC traffic can be inspected with `mcp-inspector`.

## When NOT to use

- For day-to-day artifact work - that is what the agent-side MCP tools (`forgeplan_*`) are for.
- For HTTP / network exposure - `mcp` only covers stdio. Forgeplan is local-first.

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
| [`serve`](/docs/cli/mcp-serve/) | Alias for [`forgeplan serve`](/docs/cli/serve/) - starts the stdio MCP server |
| `help` | Print help for `mcp` or its subcommands |

## Examples

### Onboard a fresh Claude Code install

```bash
forgeplan mcp install --client claude --scope user
```

Writes a user-scoped entry to `~/.claude.json`. The client must still load and successfully initialize it before tools are available; inspect the executable path after upgrades.

### Project-scoped Cursor config

```bash
forgeplan mcp install --client cursor --scope project
```

Writes a project-scoped entry to `./.cursor/mcp.json`. Loading and startup depend on the actual client's workspace and policy.

### Manually run the server (debugging only)

```bash
cd /path/to/project
forgeplan mcp serve
```

Same effect as `forgeplan serve`. Useful when piping JSON-RPC by hand or attaching
`mcp-inspector`.

## How it fits the workflow

`mcp install` is client configuration, while `mcp serve` is the runtime. The client normally launches that runtime. Confirm the intended binary, profile, workspace and handshake before relying on its tools; registration alone does not establish methodology gate completion.

## See also

- [`forgeplan mcp install`](/docs/cli/mcp-install/) - wire Forgeplan into a client
- [`forgeplan mcp serve`](/docs/cli/mcp-serve/) - alias for `forgeplan serve`
- [`forgeplan serve`](/docs/cli/serve/) - primary reference for the MCP server
- [MCP tools index](/docs/mcp/) - what the server exposes
- [`forgeplan health`](/docs/cli/health/) - verify the workspace before connecting

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan mcp --help`.

```text
MCP integration helpers (install config for AI agent clients)

Usage: forgeplan mcp <COMMAND>

Commands:
  serve    Start MCP server (alias for `forgeplan serve`)
  install  Install forgeplan MCP config into a client (Claude / Cursor / Windsurf)
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

- [`forgeplan mcp serve`](/docs/cli/mcp-serve/)
- [`forgeplan mcp install`](/docs/cli/mcp-install/)
<!-- forgeplan-runtime:end -->
