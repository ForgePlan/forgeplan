# ForgePlan Web v2

## Позиционирование

**Read-only explorer for engineering contracts, executions, evidence and decisions.**

ForgePlan Web показывает, что агенту поручили, что фактически изменилось, какими доказательствами подтверждено и почему результат принят.

## Новые представления

### Contract view

- source artifacts;
- contract version/digest;
- scope and forbidden paths;
- constraints;
- acceptance criteria;
- contract diff.

### Evidence matrix

Criterion → Evidence → provenance → verdict → freshness.

### Execution view

- host/orchestrator;
- external task/workspace/session IDs;
- actor;
- base/result SHA;
- changed paths;
- PR/CI links.

### Verification timeline

contract compiled → execution registered → Evidence submitted → verification → activation.

### Authority map

Кто предложил, утвердил, исполнил, проверил и активировал.

### PR graph delta

До/после по artifact graph, contracts, claims и Evidence.

## Граница

Web остаётся read-only и не становится:

- Kanban;
- terminal;
- code editor;
- agent launcher;
- worktree manager;
- scheduler.

## Документация

Web README и Marketplace guide должны генерироваться из единого source или проверяться cross-repo CI, чтобы install flow и feature status не расходились.
