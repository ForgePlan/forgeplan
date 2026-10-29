---
title: forgeplan serve
description: "Start the Forgeplan MCP server (stdio transport) for AI agent integration"
---

Start the Forgeplan MCP (Model Context Protocol) server on stdio. This is
the primary integration point for AI agents - Claude Code, Cursor, Windsurf,
and other MCP clients launch `forgeplan serve` as a subprocess and talk to it
over JSON-RPC on stdin/stdout.

## Usage

```text
forgeplan serve [OPTIONS]
```

## Options

```text
      --auto-index        Enable background Constellation maintenance
      --profile <PROFILE> full (default) or constellation-read
      --workspace <PATH>  Server default (startup environment can override)
  -h, --help              Print help
  -V, --version           Print version
```

## What it exposes

The default `full` profile includes read **and write** tools. Consult the
[MCP overview](/docs/mcp/) and the running server's `tools/list` for the
version-specific interface; registration does not prove that every tool
supports a migrated workspace.

In the local `0.39.0+constellation.s1.1` build (not the published 0.38.0),
`--profile constellation-read` selects a separate registry-bound, handle-based
read interface. It is not a read-only switch for the full tool namespace.
See [Constellation setup](/docs/guides/constellation/) and the
[read contracts](/docs/reference/constellation/).

`--auto-index` enables index maintenance, not automatic embeddings by itself.
Vector queue draining also requires a semantic-search build, explicit
`embedding:` configuration with `model: bge-m3`, a complete cached model
snapshot, and a running maintenance host. Maintenance is implied for an
already-migrated workspace.

## Transport

- **stdio only** - no HTTP, no sockets, no network exposure. The MCP client
  owns the process and communicates over pipes.
- **Workspace routing** - `--workspace` sets the server default. Full-server
  startup selects maintenance using `FORGEPLAN_WORKSPACE` → server default →
  current-directory discovery. Bind that value to the project or explicitly
  set it empty; unsetting permits `.env` reload. Captured help below omits
  this override. For tools accepting
  `workspace`, request parameter → environment → server default → current
  directory. Request paths accept absolute or `~/`-expanded paths. A later
  request for another workspace does not move maintenance.
- **Profile state** - the read profile holds process-local scope/snapshot
  handles; restart invalidates those handles.

## Examples

### Example 1: Claude Code client config
See "Typical usage" below.

### Example 2: Manual smoke test
See "Manual usage (debugging)" below.

## Typical usage (automatic)

You usually don't run `serve` manually. MCP clients launch it as a child
process via their config file. For Claude Code, use `forgeplan mcp install`
or a project `.mcp.json` entry:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve", "--profile", "full", "--workspace", "/path/to/your/project"],
      "env": { "FORGEPLAN_WORKSPACE": "/path/to/your/project" }
    }
  }
}
```

Client configuration formats vary. Restart the client after editing its
configuration and inspect `tools/list` to confirm the selected profile.

## Manual usage (debugging)

Run `serve` directly only when you need to:

- **Debug a new MCP tool** - wire it up by hand, pipe JSON-RPC requests in,
  inspect the response.
- **Validate protocol compliance** - run against `mcp-inspector` or similar.
- **Smoke-test after a release** - make sure the binary starts and lists its
  tool schema.

```bash
# Manual smoke test
cd /path/to/project
forgeplan serve
# (server waits on stdin for JSON-RPC messages)
# Ctrl-C to exit
```

For interactive exploration the `mcp-inspector` tool is much more productive
than hand-writing JSON-RPC.

## Troubleshooting

- **"No workspace found"** - set `--workspace` to the intended project root;
  do not assume the AI agent's working directory is the server's directory.
- **Tools missing in client** - restart the MCP client after editing its
  config; most clients only read `mcp.json` at startup.
- **LLM features fail** — `reason` and LLM-classified routing need a provider
  configured in `.forgeplan/config.yaml`; keyword routing does not itself need an LLM.
- **Semantic search unavailable** - distinguish the `semantic-search` build
  feature, cached model files, and embeddings in the index. See
  [`forgeplan embed`](/docs/cli/embed/); forced semantic search on a migrated
  workspace does not promise keyword fallback.

## See also

- [CLI overview](/docs/cli/)
- [MCP tools reference](/docs/mcp/) - full list of exposed tools
- [`forgeplan health`](/docs/cli/health/) - verify workspace before serving
- [`forgeplan index`](/docs/cli/index-command/) - Constellation maintenance; use legacy `reindex` only with the appropriate layout

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan serve --help`.

```text
Start MCP server (stdio transport) for AI agent integration

Usage: forgeplan serve [OPTIONS]

Options:
      --auto-index
          Run a Constellation maintenance pass + embedding-queue drain in the background while serving (explicit opt-in; the index itself is never required for reads)

      --profile <PROFILE>
          Server profile: `full` (default — all tools) or `constellation-read` (stage-5 read profile — only scope/snapshot handle reads; no write tools, no raw paths accepted)
          
          [default: full]

      --workspace <PATH>
          Workspace this process maintains, when it is not the one the server was started in.
          
          Absent, the maintenance host follows the process's working directory. Present, it follows **this** path — so `serve --auto-index --workspace Y` in a checkout of X maintains Y and never X. Without the flag, `forgeplan serve` started in X had no way to keep a second workspace current at all, and any attempt to infer one from the server's own default resolution would silently pick X again.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
