---
title: MCP Setup — One-Command Install
description: Connect Forgeplan as an MCP server to Claude Code, Cursor, or Windsurf in 30 seconds.
---

After `brew install forgeplan`, hooking it up to your AI agent is **one command**.
No JSON editing, no copy-paste. Smart-merge preserves your existing config.

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

Restart the client. Done — all 47 `forgeplan_*` MCP tools are now available.

## What it does

The command writes a `forgeplan` entry into your client's MCP config:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "/opt/homebrew/bin/forgeplan",
      "args": ["serve"],
      "transport": "stdio"
    }
  }
}
```

It's **smart-merge**:
- Replaces `command` / `args` / `transport` (so `forgeplan upgrade` works cleanly)
- **Preserves your `env`** (API keys, `RUST_LOG`, custom paths)
- Leaves other MCP servers in the file untouched
- Idempotent — safe to re-run

## Options

### Scope: user vs project

```bash
forgeplan mcp install --client claude --scope user      # ~/.claude.json (default)
forgeplan mcp install --client claude --scope project   # ./.mcp.json (per-repo)
```

Per-project install lets each repo pin a different `forgeplan` binary or env.

### Short name (`forgeplan` or `fpl`)

By default the command writes the **absolute path** to your binary. That's the
safest choice — works in any client, including macOS GUI apps that don't
inherit your shell `$PATH`.

If you'd rather use the short name (and you're sure your client launches with
`$PATH` set up):

```bash
forgeplan mcp install --client claude --use-name fpl       # writes "fpl"
forgeplan mcp install --client claude --use-name forgeplan # writes "forgeplan"
```

:::caution
**macOS GUI applications** (Claude Code Mac app, Cursor app) get only the
system default `$PATH` — `/opt/homebrew/bin` is **not** in it. Short names
fail silently in those clients. Stick with the default (absolute path)
unless you've configured `launchctl setenv PATH ...` system-wide.
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
#    (Claude Code, Cursor, Windsurf — fully quit and re-open)

# 2. In your project directory, initialize a workspace
forgeplan init -y

# 3. Verify MCP is wired up
#    Ask the AI agent: "use forgeplan_health to check the project"
```

If the agent reports back a healthy project status, MCP is working.

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
Error: refusing to write to symlink: ~/.claude.json — remove the symlink and re-run install
```

The target file is a symlink. We refuse to follow it (security: prevents
attackers from steering writes to sensitive files via pre-planted symlinks).
Replace the symlink with a regular file or remove it.

### Already up to date

```
✓ Claude Code MCP config already up to date: ~/.claude.json
```

The config matches what we'd write — nothing to change. Idempotency working
as intended.

### Workspace not initialized

After install, the agent calls a `forgeplan_*` tool and gets:

```
Workspace not initialized. Call forgeplan_init first.
```

Run `forgeplan init -y` in your project directory, or ask the agent to call
`forgeplan_init` via MCP — it will use whatever directory the agent's
working in.

### Re-run after `brew upgrade`

`forgeplan mcp install` is idempotent — re-run it after any version bump to
refresh the config. The detected binary path will pick up the new version
automatically.

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

The `transport: "stdio"` field is optional (most clients default to stdio).
