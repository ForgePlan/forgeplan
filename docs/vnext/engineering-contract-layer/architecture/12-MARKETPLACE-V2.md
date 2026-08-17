# ForgePlan Extensions / Marketplace v2

## Новое позиционирование

Marketplace становится каталогом **ForgePlan Extensions**, а не Claude Code-only набором, который одновременно обещает универсальность.

## Категории

```text
Host Integrations
Orchestrator Adapters
Methodology Packs
Domain Packs
Evidence Providers
Migration Packs
Visualization Extensions
```

## Manifest

Каждое расширение содержит:

- stable ID and version;
- kind;
- protocol/core compatibility;
- host/orchestrator versions;
- capabilities;
- permissions;
- owned and non-owned states;
- install/uninstall instructions;
- conformance suite and result;
- security disclosure;
- maturity: experimental/beta/stable/deprecated.

## Smith

Smith остаётся optional UX router и methodology orchestrator. Core не зависит от Smith.

## Cross-CLI

- MCP и `.agents/skills` — portable baseline;
- agents/commands/hooks — host-specific generated outputs;
- нельзя называть integration full, если доступны только skills и MCP;
- compatibility matrix генерируется из conformance runs.

## Каталог

README и сайт генерируются из manifests. Static counts запрещены.
