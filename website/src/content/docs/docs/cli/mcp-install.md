---
title: forgeplan mcp install
description: "Smart-merge Forgeplan into Claude Code, Cursor, or Windsurf MCP config - cross-platform, idempotent, brew-upgrade-safe."
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

- For per-tool configuration - this writes the launch entry, not workspace settings. The server finds `.forgeplan/` from its launch working directory.
- For HTTP / network MCP - Forgeplan only ships stdio.
- To remove Forgeplan from a client - edit the config file by hand; `install` only adds.

## Usage

```text
forgeplan mcp install [OPTIONS] --client <CLIENT>
```

## Options

```text
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

`--binary-path` and `--use-name` are mutually exclusive. Automatic detection prefers a PATH entry only when it resolves to the running executable; otherwise it falls back to `current_exe`. A stable Homebrew path such as `/opt/homebrew/bin/forgeplan` is preferred, not guaranteed. Inspect the chosen path after installation.

## Examples

### Example 1: Claude Code, user-wide

```bash
forgeplan mcp install --client claude
```

Writes `~/.claude.json`. Default scope is `user`, so every project Claude Code opens
sees Forgeplan tools.

### Example 2: Cursor, project-only

```bash
forgeplan mcp install --client cursor --scope project
```

Writes `./.cursor/mcp.json`. Forgeplan only loads when this repo is the active workspace -
useful when only some projects in a monorepo use Forgeplan.

### Example 3: Preview before writing

```bash
forgeplan mcp install --client windsurf --dry-run
```

Prints a labeled line diff between the current and proposed JSON without writing the config or creating its directories. It is not a standalone merged JSON document. Re-run the same client/scope/path options without `--dry-run` to apply; the printed generic `forgeplan mcp install` hint omits the required client flag.

### Example 4: Use short-name instead of absolute path

```bash
forgeplan mcp install --client cursor --use-name forgeplan
```

Writes `"command": "forgeplan"` - relies on `$PATH` at MCP launch time. **Caveat for
macOS GUI clients**: Claude Code Mac and Cursor app do **not** inherit shell PATH, so
short names break unless you have set up `launchctl setenv PATH ...`. Default (absolute
path) is the safer choice.

## Config files written

| Client | User scope | Project scope |
|---|---|---|
| `claude` | `~/.claude.json` | `./.mcp.json` |
| `cursor` | `~/.cursor/mcp.json` | `./.cursor/mcp.json` |
| `windsurf` | `~/.codeium/windsurf/mcp_config.json` | _not supported_ |

Windsurf has no per-project config; pass `--scope user` (the default).

## Smart-merge behaviour

- Replaces `command` and `args` (`["serve"]`). Removes `transport` and `type` only when their value is `"stdio"`; explicit HTTP/SSE values and other unknown fields survive.
- **Preserves** any existing `env` block on the entry (project-specific API keys etc.).
- Leaves all other servers in `mcpServers` untouched.
- Idempotent - running twice with the same flags is a no-op.
- Rejects malformed JSON, a non-object root or `mcpServers`, and symlinked config paths instead of silently overwriting them. An explicit `--binary-path` must be an existing executable regular file at an absolute path.

## How it fits the workflow

`mcp install` is the bridge between "binary is on disk" and "agent can call Forgeplan
tools". After this succeeds, restart the client and confirm that it discovers Forgeplan's tools in the intended project. A separate CLI `forgeplan health` checks the workspace, not whether the client launched the MCP server successfully. No LLM/model is needed merely to install the entry; individual tools have their own requirements.

## See also

- [`forgeplan mcp`](/docs/cli/mcp/) - parent command
- [`forgeplan mcp serve`](/docs/cli/mcp-serve/) - start the server (alias)
- [`forgeplan serve`](/docs/cli/serve/) - the underlying server reference
- [MCP tools index](/docs/mcp/) - what the server exposes after install
- [`forgeplan health`](/docs/cli/health/) - verify after the client restarts
