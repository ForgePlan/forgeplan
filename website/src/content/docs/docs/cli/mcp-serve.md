---
title: forgeplan mcp serve
description: "Alias for `forgeplan serve` - starts the Forgeplan MCP server on stdio."
---

`forgeplan mcp serve` is a thin alias for [`forgeplan serve`](/docs/cli/serve/). Both
commands start the same stdio MCP server (73 tools in the released 0.38.0
snapshot). It runs until the connection ends or the process is interrupted.
The alias exists so the `mcp` namespace is internally consistent - once you
have run `forgeplan mcp install`, the obvious follow-up to debug it is `forgeplan mcp serve`.

For the full reference (tools exposed, transport details, troubleshooting, client config
examples), see [`forgeplan serve`](/docs/cli/serve/).

## When to use

- Manual debugging: pipe JSON-RPC requests in, inspect responses.
- Validating MCP protocol compliance with `mcp-inspector`.
- Smoke-testing after a release that the binary boots and lists its tool schema.

## When NOT to use

- For day-to-day agent work - the MCP client (Claude Code, Cursor, Windsurf) launches
  the server for you. Manual invocation is rare.
- To configure tools through runtime flags — there are none. Startup searches
  for a workspace from the launch directory; supported tools may override it
  through their `workspace` input or `FORGEPLAN_WORKSPACE`.

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

No runtime options. The launch directory seeds the default workspace; this is
not a single-workspace isolation boundary. Workspace-aware tools can route
individual calls to other workspaces. Provider settings come from configuration
when an LLM operation needs them; starting the server does not call an LLM.

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

The CLI entrypoint does not install a tracing subscriber, so `RUST_LOG=debug`
alone is not a reliable way to obtain rmcp dispatch logs in 0.38.0.
Use a protocol inspector to inspect initialization and `tools/list`.

## How it fits the workflow

`mcp serve` is the runtime entry point: AI agents (Claude Code, Cursor, Windsurf) launch
it as a subprocess via the config file `forgeplan mcp install` wrote. You almost never
invoke it directly during normal artifact work - the methodology cycle
(Shape → Validate → Code → Evidence → Activate) runs through the tools the server
exposes, not through this command.

## See also

- [`forgeplan serve`](/docs/cli/serve/) - primary reference (tools, transport, troubleshooting)
- [`forgeplan mcp`](/docs/cli/mcp/) - parent command
- [`forgeplan mcp install`](/docs/cli/mcp-install/) - wire this into a client
- [MCP tools index](/docs/mcp/) - what the server exposes
- [`forgeplan health`](/docs/cli/health/) - verify the workspace before launching
