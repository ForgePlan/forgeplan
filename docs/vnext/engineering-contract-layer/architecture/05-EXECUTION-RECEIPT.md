# ExecutionReceipt Specification

## Назначение

ExecutionReceipt связывает WorkContract с фактическим исполнением, не превращая ForgePlan в runtime или scheduler.

## Владелец runtime state

Внешний host/orchestrator владеет:

- процессом агента;
- workspace/worktree;
- branch;
- session lifecycle;
- retries и heartbeat;
- terminal и sandbox.

ForgePlan хранит нормализованную receipt и external references.

## Поля

- execution ID;
- WorkContract ID/version/digest;
- actor identity;
- host and orchestrator identity;
- external task/workspace/session/run IDs;
- repository base and result refs;
- started/completed timestamps;
- normalized status;
- changed paths reported by provider;
- usage/cost references;
- receipt digest;
- raw provider payload reference where permitted.

## Нормализованные статусы

```text
registered
running
blocked
awaiting_review
completed
failed
cancelled
expired
superseded
```

## Правила

- повторная регистрация с тем же idempotency key не создаёт новую receipt;
- provider-specific поля хранятся в namespaced extension object;
- completion не означает acceptance;
- receipt не может сама активировать artifact;
- external status не становится вторым каноническим task state в ForgePlan.
