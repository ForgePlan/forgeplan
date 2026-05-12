---
title: forgeplan serve
description: "Start the Forgeplan MCP server (stdio transport) for AI agent integration"
---

Start the Forgeplan MCP (Model Context Protocol) server on stdio. This is
the primary integration point for AI agents — Claude Code, Cursor, Windsurf,
and other MCP clients launch `forgeplan serve` as a subprocess and talk to it
over JSON-RPC on stdin/stdout.

## Usage

```text
forgeplan serve
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## What it exposes

72 MCP tools mapped onto the core Forgeplan operations:

- **Artifact lifecycle** — `new`, `validate`, `review`, `activate`,
  `supersede`, `deprecate`, `stale`, `renew`, `reopen`
- **Query + discovery** — `list`, `get`, `search`, `blocked`, `order`,
  `tree`, `discover`, `blindspots`, `gaps`
- **Scoring + reasoning** — `score`, `fgr`, `reason`, `route`, `estimate`
- **Evidence + links** — `link`, `unlink`, `new_evidence`
- **Tags** — `tag`, `untag`
- **Health + observability** — `health`, `status`, `coverage`, `drift`
- **FPF knowledge base** — `fpf_search`, `fpf_section`, `fpf_check`
- **Memory** — `remember`, `recall`, `promote`

Forgeplan is **MCP-first**: the CLI is a convenience wrapper, and the full
power of the tool is designed to be driven by AI agents through this server.

## Transport

- **stdio only** — no HTTP, no sockets, no network exposure. The MCP client
  owns the process and communicates over pipes.
- **One workspace per server** — the server runs in the current working
  directory and operates on `./.forgeplan/`. Launch it from the project root.
- **Stateless** — no long-lived state between requests beyond what's in
  LanceDB and markdown.

## Examples

### Example 1: Claude Code client config
See "Typical usage" below.

### Example 2: Manual smoke test
See "Manual usage (debugging)" below.

## Typical usage (automatic)

You usually don't run `serve` manually. MCP clients launch it as a child
process via their config file. For Claude Code, add to `~/.claude/mcp.json`:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

Cursor (`~/.cursor/mcp.json`) and Windsurf use the same schema. Restart the
client; Forgeplan tools (`mcp__forgeplan__*`) become available to the agent.

## Manual usage (debugging)

Run `serve` directly only when you need to:

- **Debug a new MCP tool** — wire it up by hand, pipe JSON-RPC requests in,
  inspect the response.
- **Validate protocol compliance** — run against `mcp-inspector` or similar.
- **Smoke-test after a release** — make sure the binary starts and lists its
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

- **"No workspace found"** — the server was launched in a directory without
  `.forgeplan/`. Set `cwd` in your MCP client config to the project root.
- **Tools missing in client** — restart the MCP client after editing its
  config; most clients only read `mcp.json` at startup.
- **LLM features fail** — LLM-backed tools (`reason`, `route`) need a
  provider configured in `.forgeplan/config.yaml`. See the LLM guide.
- **Semantic tools no-op** — `embed` feature flag must be compiled in for
  semantic search; otherwise those tools fall back to keyword search.

## See also

- [CLI overview](/docs/cli/)
- [MCP tools reference](/docs/mcp/) — full list of exposed tools
- [`forgeplan health`](/docs/cli/health/) — verify workspace before serving
- [`forgeplan reindex`](/docs/cli/reindex/) — rebuild index if tools return stale data
