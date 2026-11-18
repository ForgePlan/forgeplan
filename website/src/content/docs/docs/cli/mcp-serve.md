---
title: forgeplan mcp serve
description: "Alias for `forgeplan serve` - starts the Forgeplan MCP server on stdio."
---

`forgeplan mcp serve` starts the full MCP server over stdio. It shares the
full-profile behavior of [`forgeplan serve`](/docs/cli/serve/), including
write tools, but does **not** accept `--profile`. The standalone
`forgeplan-mcp` binary likewise accepts `--auto-index` and `--workspace`,
not `--profile`. Use `forgeplan serve --profile constellation-read` for the
separate read profile.

For version-specific tools and their schemas, see [MCP overview](/docs/mcp/)
and the running server's `tools/list`.

## When to use

- Manual debugging: pipe JSON-RPC requests in, inspect responses.
- Validating MCP protocol compliance with `mcp-inspector`.
- Smoke-testing after a release that the binary boots and lists its tool schema.

## When NOT to use

- For day-to-day agent work - the MCP client (Claude Code, Cursor, Windsurf) launches
  the server for you. Manual invocation is rare.
- To select a read profile - use `forgeplan serve --profile constellation-read`.

## Usage

```text
forgeplan mcp serve [OPTIONS]
```

Equivalent to:

```text
forgeplan serve --profile full
```

## Options

```text
      --auto-index        Enable background Constellation maintenance
      --workspace <PATH>  Server default (startup environment can override)
  -h, --help              Print help
  -V, --version           Print version
```

At full-server startup, maintenance selects `FORGEPLAN_WORKSPACE` → server
default from `--workspace` → current-directory discovery. Bind the environment
value to the project or explicitly set it empty; unsetting permits `.env`
reload. The captured help below omits this override. A request
selects explicit `workspace` → environment → server default → current
directory. Later requests do not move the running maintenance host.
Maintenance is implied on an already-migrated workspace. `--auto-index` does not
configure automatic embeddings. See [Constellation setup](/docs/guides/constellation/) for prerequisites.

## Examples

### Smoke test after install

```bash
cd /path/to/project
forgeplan mcp serve
# Server waits on stdin for JSON-RPC. Ctrl-C to exit.
```

Process survival only shows that it has not exited yet. Complete MCP initialization
and `tools/list` (for example through `mcp-inspector`) to inspect the interface; neither alone proves every tool works.

### Debug a custom MCP tool

```bash
RUST_LOG=debug forgeplan mcp serve
```

`RUST_LOG=debug` surfaces the rmcp dispatch trace - useful when a new tool is registered
but the client claims it does not exist.

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

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan mcp serve --help`.

```text
Start MCP server (alias for `forgeplan serve`)

Usage: forgeplan mcp serve [OPTIONS]

Options:
      --auto-index        Run a Constellation maintenance pass + embedding-queue drain in the background while serving. Same meaning as `forgeplan serve --auto-index`, and on an already-migrated workspace it is implied
      --workspace <PATH>  Workspace this process maintains, when it is not the one the server was started in. Same meaning as `forgeplan serve --workspace`
  -h, --help              Print help
  -V, --version           Print version
```
<!-- forgeplan-runtime:end -->
