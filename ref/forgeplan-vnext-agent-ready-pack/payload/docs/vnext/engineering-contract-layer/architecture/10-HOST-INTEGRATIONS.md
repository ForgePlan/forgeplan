# Host Integrations

## Cursor

Пакет должен включать:

- Cursor plugin;
- MCP configuration;
- rules;
- portable skills;
- planner/builder/verifier subagents;
- pre-tool, post-tool и stop hooks;
- path scope enforcement;
- Evidence capture;
- local/cloud capability matrix.

Cursor владеет worktree, sandbox и session.

## Codex

Пакет должен включать:

- concise AGENTS.md generator;
- `.agents/skills/forge/`;
- MCP configuration;
- planner/verifier skills;
- Codex SDK adapter;
- thread ↔ execution correlation;
- resume and result capture.

Codex владеет thread, model, tools и workspace.

## OpenCode

Пакет должен включать:

- TypeScript plugin;
- MCP configuration;
- agents and skills;
- permission compiler;
- event bridge;
- execution and Evidence capture.

OpenCode рекомендуется как reference adapter для granular permissions.

## Claude Code

Существующая интеграция становится одним из adapters, а не определением Marketplace. Claude-specific agents, commands и hooks должны быть явно маркированы.

## Generic MCP/AGENTS.md

Минимальная portable integration:

- MCP stdio/HTTP;
- `.agents/skills`;
- short AGENTS.md;
- advisory policy when hooks unavailable.
