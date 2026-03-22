---
id: RFC-004
title: "MCP Server — expose Forgeplan tools via Model Context Protocol"
status: Accepted
author: explosovebit
created: 2026-03-22
updated: 2026-03-22
prd: PRD-001
depth: standard
---

# RFC-004: MCP Server Architecture

## Progress

```
Phase 4A  ████████████████████████  1/1   (100%)  MCP Server
─────────────────────────────────────────────────
TOTAL                               1/1   (100%)
```

---

## Summary

Новый crate `forgeplan-mcp` — MCP сервер на базе rmcp 1.2.0, экспонирующий все 11 CLI команд как MCP tools через stdio transport. AI-агенты (Claude Code, Cursor, и др.) используют Forgeplan через стандартный Model Context Protocol.

## Motivation

Phase 2 (Workflow & Claude Code Integration) планировал slash commands и Hindsight интеграцию. MCP — стандартизированный протокол, который покрывает те же use cases лучше:
- Любой MCP-совместимый клиент может использовать Forgeplan (не только Claude Code)
- Tools discovery через `tools/list` вместо hardcoded slash commands
- Structured JSON responses вместо текстового вывода
- Schema validation на уровне протокола (JSON Schema через schemars)

Phase 2 superseded by MCP.

## Goals

- [x] Expose 11 CLI commands as MCP tools
- [x] Stdio transport для запуска через `forgeplan serve`
- [x] Structured JSON responses (не ASCII-таблицы)
- [x] Lazy workspace initialization (server может стартовать без .forgeplan/)
- [x] Integration с CLI binary (один бинарник)

## Non-Goals

- HTTP/SSE transport (stdio достаточен для CLI-based MCP clients)
- MCP resources и prompts (только tools)
- Authentication / multi-user (local-first single-user)

## Architecture

```
forgeplan-core (shared library)
    ↑                    ↑
forgeplan-cli         forgeplan-mcp
  `forgeplan <cmd>`     ForgeplanServer + rmcp tools
  + `forgeplan serve`   ← calls forgeplan_mcp::run_stdio()
```

### Crate Structure

```
crates/forgeplan-mcp/
├── Cargo.toml          ← rmcp 1.2.0, schemars 0.8
├── src/
│   ├── lib.rs          ← pub run_stdio(), re-exports
│   ├── main.rs         ← standalone binary entry point
│   ├── server.rs       ← ForgeplanServer + 11 tools
│   ├── types.rs        ← Request/Response DTOs (JsonSchema)
│   └── convert.rs      ← From impls: core types → DTOs
```

## Key Decisions

### 1. Store Lifecycle: `Arc<RwLock<Option<LanceStore>>>`

**Problem**: MCP server может быть запущен в директории без `.forgeplan/`. Tool `forgeplan_init` должен уметь создать workspace на лету.

**Decision**: Store хранится как `Option` — `None` до init, `Some` после. `RwLock` вместо `Mutex` — большинство tools только читают (read lock), только `init` пишет (write lock).

**Alternative rejected**: Require workspace at startup — ограничивает UX, агент не может сам инициализировать workspace.

### 2. Structured JSON vs Text Output

**Decision**: Все tools возвращают `serde_json::to_string_pretty()` в `Content::text()`. AI-агенты получают парсабельный JSON.

**Why not separate Content types**: MCP `Content::text()` — единственный universal тип. Embedding JSON в text — стандартная практика MCP серверов.

### 3. Error Handling: Application vs Protocol

**Decision**: Application errors (artifact not found, validation failure) → `CallToolResult::error()` с текстовым описанием. Protocol errors (malformed request) → `McpError` (rmcp ErrorData).

**Rationale**: AI-агент должен видеть application errors как результат tool call, не как protocol failure. Это стандартная MCP конвенция.

### 4. Library + Binary Crate

**Decision**: `forgeplan-mcp` — и библиотека (`run_stdio()`), и бинарник (`forgeplan-mcp`). CLI добавляет `forgeplan-mcp` как dependency для `forgeplan serve`.

**Rationale**: Один shared `run_stdio()` — нет дублирования. Standalone binary полезен для MCP config файлов (`mcp.json`).

## Tool Mapping

| CLI Command | MCP Tool | Params | Mutation |
|-------------|----------|--------|----------|
| `forgeplan init [--force]` | `forgeplan_init` | `{ force? }` | Write |
| `forgeplan new <kind> <title>` | `forgeplan_new` | `{ kind, title }` | Write |
| `forgeplan list [-t kind] [-s status]` | `forgeplan_list` | `{ kind?, status? }` | Read |
| `forgeplan status` | `forgeplan_status` | — | Read |
| `forgeplan validate [id]` | `forgeplan_validate` | `{ id? }` | Read |
| `forgeplan score <id>` | `forgeplan_score` | `{ id }` | Read |
| `forgeplan link <src> <tgt> [--rel]` | `forgeplan_link` | `{ source, target, relation? }` | Write |
| `forgeplan graph` | `forgeplan_graph` | — | Read |
| `forgeplan search <q> [-t kind]` | `forgeplan_search` | `{ query, kind? }` | Read |
| `forgeplan stale` | `forgeplan_stale` | — | Read |
| `forgeplan progress [id]` | `forgeplan_progress` | `{ id? }` | Read |

## Dependencies

```toml
rmcp = { version = "1.2", features = ["server", "transport-io"] }
schemars = "0.8"   # JSON Schema generation for tool parameters
```

rmcp macros: `#[tool_router]` на impl блоке с tools, `#[tool_handler]` на `impl ServerHandler`.

## Testing

- 158 existing tests unaffected
- Smoke test: `initialize` + `tools/list` via stdio JSON-RPC
- Integration tests (future): temp workspace → call tools → verify JSON responses

## Usage

### Standalone
```bash
forgeplan-mcp              # запускает stdio MCP server
```

### Via CLI
```bash
forgeplan serve            # то же через CLI binary
```

### MCP Config (Claude Code)
```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"]
    }
  }
}
```

## References

- ADR-002: LanceDB as storage backend
- RFC-003: LanceDB integration (async, tables, store API)
- rmcp 1.2.0: https://crates.io/crates/rmcp
- MCP Specification: https://modelcontextprotocol.io
