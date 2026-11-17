---
title: forgeplan serve
description: "Start the Forgeplan Model Context Protocol (MCP) server over stdio. The client owns the subprocess and exchanges JSON-RPC over stdin/stdout. The 0.38.0 server exposes 73 tools, not a one-to-one copy of all CLI commands."
---

Start the Forgeplan Model Context Protocol (MCP) server over stdio. The client owns the subprocess and exchanges JSON-RPC over stdin/stdout. The 0.38.0 server exposes 73 tools, not a one-to-one copy of all CLI commands.

## Usage

```text
forgeplan serve
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Available operations

Examples include `forgeplan_new`, `forgeplan_get`, `forgeplan_validate`, `forgeplan_score`, `forgeplan_reason`, `forgeplan_claim`, and `forgeplan_discover_finding`. There are no MCP tools named `forgeplan_context`, `forgeplan_tree`, `forgeplan_fgr`, `forgeplan_tag`, `forgeplan_remember`, or `forgeplan_new_evidence`. Create evidence with `forgeplan_new` and its kind parameter. See the MCP reference for all 73 schemas.

## Client setup

Prefer the client-aware installer rather than copying one configuration schema across clients:

```bash
forgeplan mcp install --client claude --scope user --dry-run
forgeplan mcp install --client claude --scope user
```

Claude user configuration is `~/.claude.json`; project scope uses `.mcp.json`. Cursor and Windsurf have their own locations and conventions. Start the server with the project as working directory; do not assume every client supports a `cwd` entry. Restart or reconnect the client after configuration changes.

## Manual use

```bash
# Run from the intended project root:
forgeplan serve
```

The server waits for protocol messages, not interactive commands. Keep stdout available for protocol traffic. Persistent workspace files and methodology session state survive individual requests; this is not a claim of a stateless API.

## Troubleshooting

If no workspace is found, check the launch directory. If tools are missing, reconnect the client and inspect tools/list. LLM operations need the configured provider. Distinguish the CLI's compiled semantic-search capability from the MCP handler: in 0.38.0, MCP smart/semantic requests use the non-vector smart-search path because semantic scores are not wired into that handler. Downloading BGE-M3 weights does not fix this MCP limitation. Keyword search remains available.

## See also

- [CLI overview](/docs/cli/)
- [MCP tools reference](/docs/mcp/) - full list of exposed tools
- [`forgeplan health`](/docs/cli/health/) - verify workspace before serving
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild index if tools return stale data
