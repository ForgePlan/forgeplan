# Forgeplan Rules for Cursor

## Before coding any non-trivial task

1. Run `forgeplan route "task description"` to determine depth
2. If Standard+ → create artifact: `forgeplan new prd "title"`
3. Fill ALL required sections (Problem, Goals, Non-Goals, Target Users, FR, Related)
4. Run `forgeplan validate` → must PASS before coding

## After implementation

1. Create evidence: `forgeplan new evidence "what was proven"`
2. Link evidence: `forgeplan link EVID-XXX PRD-XXX --relation informs`
3. Review and activate: `forgeplan review PRD-XXX` → `forgeplan activate PRD-XXX`

## MCP Server

Forgeplan MCP server is available via `.mcp.json`. Key tools:
- `forgeplan_health` — project state
- `forgeplan_route` — depth + pipeline
- `forgeplan_new` — create artifact
- `forgeplan_validate` — check quality
- `forgeplan_review` — lifecycle check
- `forgeplan_activate` — draft → active
