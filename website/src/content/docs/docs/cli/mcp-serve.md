---
title: forgeplan mcp serve
description: "Alias for `forgeplan serve` — starts the Forgeplan MCP server on stdio."
---

`forgeplan mcp serve` is a thin alias for [`forgeplan serve`](/docs/cli/serve/). Both
commands start the same MCP server (stdio transport, ~47 tools, one workspace) and exit
on Ctrl-C. The alias exists so the `mcp` namespace is internally consistent — once you
have run `forgeplan mcp install`, the obvious follow-up to debug it is `forgeplan mcp serve`.

For the full reference (tools exposed, transport details, troubleshooting, client config
examples), see [`forgeplan serve`](/docs/cli/serve/).

## When to use

- Manual debugging: pipe JSON-RPC requests in, inspect responses.
- Validating MCP protocol compliance with `mcp-inspector`.
- Smoke-testing after a release that the binary boots and lists its tool schema.

## When NOT to use

- For day-to-day agent work — the MCP client (Claude Code, Cursor, Windsurf) launches
  the server for you. Manual invocation is rare.
- To configure the server — there is nothing to configure on the command line. The
  server reads `./.forgeplan/`.

## Usage

```text
forgeplan mcp serve
```

Equivalent to:

```text
forgeplan serve
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

No runtime options — the server picks up the workspace from `./.forgeplan/` and the LLM
provider from `.forgeplan/config.yaml`.

## Examples

### Smoke test after install

```bash
cd /path/to/project
forgeplan mcp serve
# Server waits on stdin for JSON-RPC. Ctrl-C to exit.
```

If you do not see the process error out immediately, the binary boots. Use `mcp-inspector`
for an interactive tool-list dump.

### Debug a custom MCP tool

```bash
RUST_LOG=debug forgeplan mcp serve
```

`RUST_LOG=debug` surfaces the rmcp dispatch trace — useful when a new tool is registered
but the client claims it does not exist.

## How it fits the workflow

`mcp serve` is the runtime entry point: AI agents (Claude Code, Cursor, Windsurf) launch
it as a subprocess via the config file `forgeplan mcp install` wrote. You almost never
invoke it directly during normal artifact work — the methodology cycle
(Shape → Validate → Code → Evidence → Activate) runs through the tools the server
exposes, not through this command.

## See also

- [`forgeplan serve`](/docs/cli/serve/) — primary reference (tools, transport, troubleshooting)
- [`forgeplan mcp`](/docs/cli/mcp/) — parent command
- [`forgeplan mcp install`](/docs/cli/mcp-install/) — wire this into a client
- [MCP tools index](/docs/mcp/) — what the server exposes
- [`forgeplan health`](/docs/cli/health/) — verify the workspace before launching
