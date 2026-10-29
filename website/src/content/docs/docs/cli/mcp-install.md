---
title: forgeplan mcp install
description: "Smart-merge Forgeplan into Claude Code, Cursor, or Windsurf MCP configuration."
---

`forgeplan mcp install` writes the Forgeplan MCP server into a client's config file
(`.mcp.json`, `~/.claude.json`, `~/.cursor/mcp.json`, or `~/.codeium/windsurf/mcp_config.json`)
without clobbering anything else that lives there. It detects the absolute path to the
running binary, merges into the `mcpServers` map, and preserves any existing `env` block
on the Forgeplan entry - so re-running after a Homebrew upgrade just refreshes the path.

Cross-platform: macOS, Linux, Windows (uses `dirs::home_dir()` and `PATHEXT` for resolution).

## When to use

- First-time setup after `brew install forgeplan` (or equivalent).
- After `brew upgrade forgeplan` invalidates the absolute Cellar path baked into `.mcp.json`.
- When onboarding a new client (e.g. you used Claude Code, now adding Cursor).
- Inside CI to bootstrap a sandboxed agent environment with Forgeplan available.

## When NOT to use

- For selecting server profile or workspace policy - installation writes client configuration; configure and inspect the actual server launch separately.
- For HTTP / network MCP - Forgeplan only ships stdio.
- To remove Forgeplan from a client - edit the config file by hand; `install` only adds.

## Usage

```text
forgeplan mcp install [OPTIONS] --client <CLIENT>
```

## Options

```text
  -c, --client <CLIENT>          Target client: claude, cursor, or windsurf
  -s, --scope <SCOPE>            Config scope: user (global) or project (local) [default: user]
      --binary-path <PATH>       Override binary path (default: detected from current_exe)
      --use-name <NAME>          Use short name instead of absolute path: forgeplan or fpl
      --dry-run                  Print proposed change without writing
  -h, --help                     Print help
  -V, --version                  Print version
```

`--binary-path` and `--use-name` are mutually exclusive. Detection prefers a stable PATH entry only when it resolves to the same executable as the installer; otherwise it falls back to `current_exe`. Stability across upgrades is not unconditional. To keep a selected `0.39.0+constellation.s1.1` build separate from installed `0.38.0`, use its explicit absolute `--binary-path` and inspect the dry run.

## Examples

### Example 1: Claude Code, user-wide

```bash
forgeplan mcp install --client claude
```

Writes a user-scoped entry in `~/.claude.json`; availability across projects still depends on the client loading and initializing it.

### Example 2: Cursor, project-only

```bash
forgeplan mcp install --client cursor --scope project
```

Writes `./.cursor/mcp.json` for project scope. The client's workspace and policy determine whether it is loaded.

### Example 3: Preview before writing

```bash
forgeplan mcp install --client windsurf --dry-run
```

Prints the merged JSON with no filesystem changes. Inspect the diff, then re-run without
`--dry-run` once happy.

### Example 4: Use short-name instead of absolute path

```bash
forgeplan mcp install --client cursor --use-name forgeplan
```

Writes `"command": "forgeplan"` and relies on PATH at MCP launch time. GUI applications on macOS may not inherit the shell's PATH; an absolute executable path avoids that dependency. Inspect the client's actual environment rather than assuming a universal PATH value.

## Config files written

| Client | User scope | Project scope |
|---|---|---|
| `claude` | `~/.claude.json` | `./.mcp.json` |
| `cursor` | `~/.cursor/mcp.json` | `./.cursor/mcp.json` |
| `windsurf` | `~/.codeium/windsurf/mcp_config.json` | _not supported_ |

Windsurf has no per-project config; pass `--scope user` (the default).

## Smart-merge behaviour

- Replaces `command`, `args`, and transport for the `forgeplan` entry.
- **Preserves** any existing `env` block on the entry (project-specific API keys etc.).
- Leaves all other servers in `mcpServers` untouched.
- Idempotent - running twice with the same flags is a no-op.

## How it fits the workflow

`mcp install` writes configuration; it does not prove the client can start or call the server. It emits `args: ["serve"]` and preserves existing `env`. After restarting the client, verify its MCP initialization and tool inventory for the intended binary, profile, and workspace. A separate CLI `forgeplan health` call does not verify that MCP connection.

## See also

- [`forgeplan mcp`](/docs/cli/mcp/) - parent command
- [`forgeplan mcp serve`](/docs/cli/mcp-serve/) - start the server (alias)
- [`forgeplan serve`](/docs/cli/serve/) - the underlying server reference
- [MCP tools index](/docs/mcp/) - what the server exposes after install
- [`forgeplan health`](/docs/cli/health/) - verify after the client restarts

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan mcp install --help`.

```text
Install forgeplan MCP config into a client (Claude / Cursor / Windsurf).

Smart-merge: replaces command/args/transport, preserves existing `env`. Idempotent — safe to re-run.

Usage: forgeplan mcp install [OPTIONS] --client <CLIENT>

Options:
  -c, --client <CLIENT>
          Target client: claude, cursor, or windsurf

  -s, --scope <SCOPE>
          Config scope: user (global) or project (local). Default: user
          
          [default: user]

      --binary-path <BINARY_PATH>
          Override binary path (default: detected from current_exe)

      --use-name <USE_NAME>
          Use short name instead of absolute path: forgeplan or fpl. Requires PATH to include the binary's directory at MCP launch time. macOS GUI apps may not inherit shell PATH — use absolute path (the default) for maximum portability

      --dry-run
          Print proposed change without writing

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
