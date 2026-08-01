# Documentation Architecture

## Навигация

1. Start — what/why/quickstart/setup selector.
2. Concepts — intent, claim, contract, Evidence, authority, lifecycle.
3. Protocol — schemas, versioning, errors, events.
4. Agent Hosts — Cursor, Codex, OpenCode, Claude Code, generic MCP.
5. Orchestrators — Kandev, Vibe Kanban, Conductor, Paperclip.
6. Components — Core, CLI, MCP, Web, Extensions, Server.
7. Methodologies — depth, ADI, FPF, BMAD, SPARC, TDD.
8. Reference — generated CLI/MCP/schema/config reference.
9. Operations — CI, security, recovery, telemetry, migration.

## Integration guide template

Каждый integration guide содержит:

1. purpose;
2. responsibility boundary;
3. supported versions;
4. capability matrix;
5. installation;
6. architecture;
7. object mapping;
8. workflow;
9. permissions/secrets;
10. Evidence collection;
11. failures/recovery;
12. limitations;
13. conformance;
14. troubleshooting;
15. uninstall/rollback.

## Docs-as-code gates

- CLI reference генерируется из clap;
- MCP reference генерируется из schemas;
- extension catalog генерируется из manifests;
- examples выполняются smoke tests;
- JSON/YAML examples schema-validated;
- links проверяются;
- versions и shipped/planned проверяются;
- EN/RU structure parity проверяется;
- cross-repo Web/Marketplace docs сверяются;
- counts не хранятся вручную.
