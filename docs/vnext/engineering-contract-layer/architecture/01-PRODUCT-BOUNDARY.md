# Product Boundary

## Каноническое определение

**ForgePlan is the repository-native engineering contract and verification layer for AI coding agents.**

ForgePlan связывает инженерный замысел, исполнение и доказательства независимо от выбранного агента или оркестратора.

## Главная ответственность core

> Сохранять инженерный замысел и определять, соответствует ли фактический результат утверждённому инженерному контракту.

## Входы

- Problems, requirements и non-goals;
- PRD, RFC, ADR и Spec;
- активные ограничения и решения;
- claims и risks;
- repository state;
- execution receipts;
- Evidence и внешние verification results.

## Выходы

- WorkContract;
- применимая policy;
- минимальный context bundle;
- Evidence requirements;
- VerificationVerdict;
- lifecycle transition или блокировка.

## ForgePlan не является

- task tracker;
- Kanban;
- coding agent;
- IDE;
- worktree manager;
- scheduler;
- general-purpose workflow engine;
- company operating system;
- memory platform общего назначения;
- CI runner;
- deployment platform.

## Владение состояниями

| Состояние | Канонический владелец |
|---|---|
| Business goals, org, budgets | Paperclip или бизнес-система |
| Task backlog, priority, assignee | Linear, Jira, Kandev, Vibe Kanban |
| Workspace, worktree, branch, session | Agent host или orchestrator |
| Agent process, tools, model | Cursor, Codex, OpenCode, Claude Code |
| Engineering intent and decisions | ForgePlan |
| WorkContract and policy | ForgePlan |
| Evidence requirements and verdict | ForgePlan |
| CI result | CI provider, referenced by ForgePlan |
| Deployment state | deployment platform |

## SOLID-проверка

### Single Responsibility

Core отвечает только за intent → contract → evidence → verdict → lifecycle.

### Open/Closed

Новые host, orchestrator, methodology и evidence provider подключаются расширениями.

### Liskov Substitution

Один WorkContract имеет одинаковую семантику в Cursor, Codex, OpenCode и Claude Code.

### Interface Segregation

Planner, builder, reviewer и operator получают разные минимальные интерфейсы и полномочия.

### Dependency Inversion

Core зависит от абстракций `ArtifactRepository`, `GitProvenanceProvider`, `EvidenceProvider`, `PolicyProvider`, `HostCapabilities`, но не от конкретных конфигов Cursor или API Paperclip.

## Принципы FORGE

- **F — Facts are canonical.** Каноническая инженерная истина версионируется; индексы и UI являются проекциями.
- **O — One owner per state.** Для каждого состояния существует один источник истины.
- **R — Runtimes are replaceable.** Agent host и модель заменяемы.
- **G — Gates verify outcomes.** Проверяется результат, а не рассказ исполнителя.
- **E — Extensions do not contaminate the kernel.** Методологии и интеграции не определяют core.
