---
id: RFC-005
title: "CRUD Completeness + MCP Config + Workflow Routing"
status: Accepted
author: explosovebit
created: 2026-03-22
updated: 2026-03-22
prd: PRD-001
depth: standard
---

# RFC-005: CRUD Completeness + MCP Config + Workflow Routing

## Progress

```
Phase 4D  ████████████████████████  5/5   (100%)  CRUD + MCP Config  ✅ DONE
─────────────────────────────────────────────────
TOTAL                               5/5   (100%)
```

## Summary

Довести движок до production-ready: полный CRUD на артефактах (get, update, delete), MCP config для Claude Code, workflow routing через LLM. После этого Forgeplan = полностью рабочий инструмент через MCP.

## Motivation

Текущий MCP server имеет 16 tools, но не хватает базового CRUD:
- Нельзя прочитать артефакт целиком (`get`)
- Нельзя обновить статус/title/body (`update`)
- Нельзя удалить артефакт (`delete`)
- Нет `.mcp.json` для подключения к Claude Code
- Нет workflow routing ("какой артефакт создать для этой задачи?")

Без этого методология не работает end-to-end через MCP.

## Implementation Phases

- [x] **D.1** `forgeplan get <id>` — read artifact (CLI + MCP tool)
- [x] **D.2** `forgeplan update <id>` — update status/title/body (CLI + MCP tool)
- [x] **D.3** `forgeplan delete <id>` — remove artifact (CLI + MCP tool)
- [x] **D.4** MCP config — `.mcp.json` for Claude Code integration
- [x] **D.5** `forgeplan route "<description>"` — LLM suggests depth + artifact type (CLI + MCP tool)

## D.1: forgeplan get

Read full artifact record from LanceDB. Returns all metadata + body.

```
forgeplan get PRD-001
```

MCP response: structured JSON with all fields.

Uses: `LanceStore::get_record(id)` (already exists).

## D.2: forgeplan update

Update artifact metadata and/or body.

```
forgeplan update PRD-001 --status active
forgeplan update PRD-001 --title "New Title"
forgeplan update PRD-001 --depth deep
forgeplan update PRD-001 --body @file.md
```

MCP params: `{ id, status?, title?, depth?, body? }` — all optional except id.

Uses: `LanceStore::update_artifact(id, status, title)` + `LanceStore::update_body(id, body)` (both exist).

After update: re-render markdown projection.

## D.3: forgeplan delete

Remove artifact from LanceDB + delete markdown projection.

```
forgeplan delete PRD-001
```

CLI: confirmation prompt (`--yes` flag to skip).
MCP: no confirmation needed (AI agent decides).

Uses: `LanceStore::delete_artifact(id)` (already exists).

## D.4: MCP Config

Create `.mcp.json` template in workspace:

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

Also: `forgeplan init` should suggest adding MCP config.

## D.5: forgeplan route

LLM-powered workflow routing: given a task description, suggest:
- Depth level (Tactical/Standard/Deep/Critical)
- Which artifact(s) to create
- Template pipeline (e.g., "PRD → RFC → ADR")

```
forgeplan route "Add OAuth2 login with Google and GitHub"
```

Output:
```
Suggested depth: Standard
Create: PRD (requirements) → RFC (architecture)
Reason: Multiple auth providers = non-trivial design choice

Run: forgeplan generate prd "Add OAuth2 login with Google and GitHub"
```

## References

- RFC-004: MCP Server Architecture
- ADR-007: Multi-provider LLM
- DEPTH-CALIBRATION.md: Routing decision tree
