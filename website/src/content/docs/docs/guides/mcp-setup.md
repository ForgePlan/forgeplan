---
title: MCP Setup - One-Command Install
description: Register Forgeplan as an MCP server for Claude Code, Cursor or Windsurf.
---

After `brew install forgeplan/tap/forgeplan`, register the binary with your
AI client using one command. No JSON editing is needed; smart-merge
preserves your existing config.

## Quick install

Pick your client:

```bash
# Claude Code (default scope: user-global ~/.claude.json)
forgeplan mcp install --client claude

# Cursor
forgeplan mcp install --client cursor

# Windsurf
forgeplan mcp install --client windsurf
```

Restart the client and approve the server if prompted. The published
Forgeplan 0.38.0 stdio server exposes **73 tools**. Registration does not
install marketplace skills or agents, initialize a workspace, or configure
an LLM provider.

## What it does

The command writes a `forgeplan` entry into your client's MCP config:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "/opt/homebrew/bin/forgeplan",
      "args": ["serve"]
    }
  }
}
```

It's **smart-merge**:
- Replaces `command` / `args` and removes legacy `transport: "stdio"` and `type: "stdio"` fields
- **Preserves your `env`** (API keys, `RUST_LOG`, custom paths)
- Leaves other MCP servers in the file untouched
- Idempotent - safe to re-run

## Options

### Scope: user vs project

```bash
forgeplan mcp install --client claude --scope user      # ~/.claude.json (default)
forgeplan mcp install --client claude --scope project   # ./.mcp.json (per-repo)
```

Per-project install lets each repo pin a different `forgeplan` binary or env.

### Short name (`forgeplan` or `fpl`)

By default the command writes the **absolute path** to your binary. That's the
safest choice - works in any client, including macOS GUI apps that don't
inherit your shell `$PATH`.

If you'd rather use the short name (and you're sure your client launches with
`$PATH` set up):

```bash
forgeplan mcp install --client claude --use-name fpl       # writes "fpl"
forgeplan mcp install --client claude --use-name forgeplan # writes "forgeplan"
```

:::caution
**macOS GUI applications** may not inherit your shell's `$PATH`. A short name
can therefore fail to start the server. Keep the default absolute path unless
you have verified the client's launch environment.
:::

### Custom binary

```bash
forgeplan mcp install --client claude --binary-path /custom/path/forgeplan
```

The path is validated: must be absolute, exist, be a regular file, and be
executable. Empty strings, relative paths, control characters, and bidi
override codepoints are rejected.

### Dry-run

See what would change without writing:

```bash
forgeplan mcp install --client claude --dry-run
```

Output shows a line-by-line diff of the proposed changes.

## After install

```bash
# 1. Restart your AI client to load the new config
#    (Claude Code, Cursor, Windsurf - fully quit and re-open)

# 2. In your project directory, initialize a workspace
forgeplan init -y

# 3. Verify MCP is wired up
#    Ask the AI agent: "use forgeplan_health to check the project"
```

A successful `forgeplan_health` response confirms that the tool call reached
the server; its verdict may still report project issues. Confirm that it
describes the intended project, not just that the verdict is `healthy`.

### Workspace selection

`forgeplan serve` starts in the client's launch directory. Project-scoped MCP
registration chooses a config file, not a workspace binding. Launch the client
from the intended project and initialize that workspace. Some 0.38.0 tools
accept an explicit `workspace` argument for worktrees, but `forgeplan_health`
and `forgeplan_init` do not. Do not assume the agent's current shell directory
changes the directory of an already-running MCP server.

The model and LLM are separate prerequisites. Artifact reads, validation and
health need no LLM; `forgeplan_reason` does. Semantic search needs the model
and artifact vectors. See [Installation](/docs/getting-started/installation/)
and [Configuration](/docs/getting-started/configuration/).

## Config paths per client

| Client | User scope | Project scope |
|--------|------------|---------------|
| Claude Code | `~/.claude.json` | `./.mcp.json` |
| Cursor | `~/.cursor/mcp.json` | `./.cursor/mcp.json` |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` | not supported |

Windows uses `%USERPROFILE%` instead of `~`.

## Troubleshooting

### Symlink rejected

```
Error: refusing to write to symlink: ~/.claude.json - remove the symlink and re-run install
```

The target file is a symlink. We refuse to follow it (security: prevents
attackers from steering writes to sensitive files via pre-planted symlinks).
Replace the symlink with a regular file or remove it.

### Already up to date

```
✓ Claude Code MCP config already up to date: ~/.claude.json
```

The config matches what we'd write - nothing to change. Idempotency working
as intended.

### Workspace not initialized

After install, the agent calls a `forgeplan_*` tool and gets:

```
Workspace not initialized. Call forgeplan_init first.
```

Run `forgeplan init -y` in the intended project, then ensure the MCP server
was launched for that project. `forgeplan_init` initializes relative to the
server's startup directory, not an arbitrary agent shell directory.

### Re-run after `brew upgrade`

Re-run `forgeplan mcp install` if the binary location changed or the config
still points into a versioned Cellar directory. Detection prefers the stable
PATH entry when it resolves to the running binary. Restart the client to
replace an already-running server process after upgrading.

## Manual setup (if you prefer)

If you'd rather edit the JSON yourself, here's the minimal entry:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "/opt/homebrew/bin/forgeplan",
      "args": ["serve"]
    }
  }
}
```

Do not add `transport: "stdio"`. The 0.38.0 installer omits it and removes
that legacy value from existing entries; `serve` uses stdio by default.
