# Target Architecture

## Слои

```text
Business / Organization
Paperclip, goals, budgets, approvals
                │
Task and Workspace Orchestration
Kandev, Vibe Kanban, Conductor, Linear, Jira
                │
ForgePlan Engineering Contract Layer
intent, contract, authority, evidence, verdict
                │
Agent Hosts
Cursor, Codex, OpenCode, Claude Code
                │
Execution Infrastructure
Git, CI, tests, builds, deployments
```

## Компоненты ForgePlan

### ForgePlan Protocol

Стабильные версионированные schemas и semantics:

- ArtifactReference;
- Claim;
- WorkContract;
- ExecutionReceipt;
- EvidenceBundle;
- VerificationVerdict;
- AuthorityPolicy;
- ExternalReference;
- CapabilityManifest;
- lifecycle events.

### ForgePlan Core

- artifact graph;
- validation;
- depth routing;
- contract compilation;
- policy evaluation;
- claims/leases;
- Evidence assessment;
- provenance verification;
- lifecycle management.

### CLI and MCP

Два транспорта к одному application layer. Различия семантики запрещены.

### ForgePlan Extensions

- host adapters;
- orchestrator adapters;
- methodology packs;
- evidence providers;
- domain packs;
- migration packs.

### ForgePlan Web

Read-only explorer для contract, graph, evidence, executions, authority и timeline.

### Optional ForgePlan Server

Поздний опциональный слой для remote MCP, event ingestion, auth, audit и multi-repository registry. Не запускает агентов и не заменяет orchestrators.

## Поток данных

```text
PRD + Spec + RFC + ADR + Policy + RepositoryState
                         │
                         ▼
                Contract Compiler
                         │
                         ▼
                  WorkContract vN
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
       Cursor          Codex         OpenCode
          │              │              │
          └──────────────┼──────────────┘
                         ▼
               ExecutionReceipt
                         │
                         ▼
                  EvidenceBundle
                         │
                         ▼
             Verification Engine
                         │
                         ▼
 accepted / rejected / incomplete / human-review
```

## Неизменяемые инварианты

1. WorkContract immutable после начала исполнения.
2. Scope expansion создаёт новую версию контракта.
3. Исполнитель не может изменить acceptance criteria запущенного контракта.
4. Текст «готово» не является Evidence.
5. Для code-claiming Evidence обязательны base SHA и result SHA.
6. Critical execution не может быть принято тем же actor, который его выполнял.
7. External task/runtime state не дублируется в ForgePlan.
8. CLI и MCP обязаны возвращать одинаковую domain semantics.
9. Extension не может получить больше полномочий, чем объявлено в manifest.
10. Web остаётся read-only относительно канонического workspace.
