---
title: MCP Setup - One-Command Install
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

Restart the client and inspect its tool list. The interface is version- and
profile-specific; see the [live MCP overview](/docs/mcp/).

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
- Replaces `command` / `args` when re-run after a Forgeplan upgrade
- **Preserves your `env`** (API keys, `RUST_LOG`, custom paths)
- Leaves other MCP servers in the file untouched
- Idempotent for its generated defaults; custom `args` are still replaced. After re-running, review `--profile`, `--workspace` and `--auto-index` before reconnecting a customized Constellation server.

Captured `mcp install --help` still says “command/args/transport”; the current
writer actually omits `transport` and removes legacy `transport: "stdio"`
and `type: "stdio"` entries. It preserves other unknown fields. Review
custom HTTP/SSE settings rather than assuming they become valid stdio config.

### Profiles, workspaces, and maintenance

In local `0.39.0+constellation.s1.1` (not published 0.38.0), `serve` defaults
to `--profile full`, including writes. `--profile constellation-read` is a
separate registry/handle read interface, not the full namespace with writes
disabled. `forgeplan mcp serve` and standalone `forgeplan-mcp` accept
`--auto-index` and `--workspace`, but not `--profile`.

For an explicit full-profile target, use this illustrative argument list in
your client's server entry:

```json
["serve", "--profile", "full", "--workspace", "/path/to/project", "--auto-index"]
```

`--workspace` sets the server default. Full-server startup selects maintenance
using `FORGEPLAN_WORKSPACE` → server default → CWD discovery. Bind the client's
environment to the project, or explicitly set `FORGEPLAN_WORKSPACE=""`:
unsetting alone allows the CLI's ancestor `.env` loader to restore a value.
A tool's explicit `workspace` wins for that request; the remaining order is
environment → server default → CWD. Another request target does not move
maintenance. The agent's CWD is not automatically the server's CWD.

`--auto-index` alone does not configure embeddings. Auto-drain needs explicit
`embedding:` config, a semantic-search build, BGE-M3 and a complete cached
snapshot. See [Constellation setup](/docs/guides/constellation/) and its
[read contracts](/docs/reference/constellation/).

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
macOS GUI applications may not inherit your shell's `$PATH`, so a short
binary name can fail if its directory is absent. Prefer the generated
absolute path; inspect the client's actual environment before relying on
short names.
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

A successful `forgeplan_health` response from the intended workspace shows
that this call reached the server. A reported health problem can still be a
valid MCP response; one call does not verify every tool or semantic readiness.

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

Initialize from the intended project root, then check the reported path.
`init -y` can find an ancestor workspace and return without changes. For
existing workspaces, follow [Configuration](/docs/getting-started/configuration/).
Pass explicit `workspace` in tools that accept it; do not assume the
agent's directory controls the server.

### Re-run after `brew upgrade`

`forgeplan mcp install` is idempotent - re-run it after any version bump to
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

For this stdio entry, omit `transport`; the generated configuration does.
