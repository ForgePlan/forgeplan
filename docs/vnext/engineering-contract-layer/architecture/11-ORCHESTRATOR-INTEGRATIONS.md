# Orchestrator Integrations

## Общий mapping

```text
External Task        → source/reference for WorkContract
External Workspace   → ExecutionReceipt reference
External Session/Run → ExecutionReceipt reference
External Work Product→ EvidenceBundle input
ForgePlan Verdict    → external review/completion signal
```

## Kandev

Kandev владеет task, workflow, workspace, agent profile, executor и automation. ForgePlan поставляет MCP profile, contract step, verification step и event bridge.

## Vibe Kanban

Vibe Kanban владеет issue, workspace/worktree, agent session, changes view и PR flow. ForgePlan компилирует contract, регистрирует execution, проверяет diff/Evidence и возвращает verdict.

## Conductor

Conductor владеет workspace, session, harness, terminal, diff и PR. Adapter должен быть versioned из-за beta API. Он создаёт workspace по WorkContract, сохраняет external IDs, получает result SHA и отправляет Evidence.

## Paperclip

Paperclip владеет company, goals, agents, issues, approvals, budgets, schedules, heartbeats и runs. ForgePlan поставляется как Plugin + Skill + MCP. Heartbeat остаётся runtime owner; ForgePlan хранит contract, receipt, Evidence и verdict.

## Запрещено

- синхронизировать task status двунаправленно без чёткого owner;
- создавать task в ForgePlan как копию внешней task;
- запускать собственный heartbeat поверх Paperclip;
- создавать worktree поверх Conductor/Kandev/Vibe;
- считать external completion эквивалентом acceptance.
