# Agent API and MCP v2

## Цели

- уменьшить tool-selection burden;
- устранить CLI/MCP semantic drift;
- дать role-specific surfaces;
- повысить производительность agent pipelines;
- обеспечить JSON Schema для всех outputs.

## Высокоуровневый agent API

```text
forgeplan_next
forgeplan_context
forgeplan_contract
forgeplan_claim
forgeplan_execution
forgeplan_evidence
forgeplan_verify
forgeplan_status
forgeplan_search
```

## Profiles

- `minimal`;
- `planner`;
- `builder`;
- `reviewer`;
- `operator`;
- `full`.

## Требования

1. CLI и MCP вызывают один application service.
2. Никакой разницы в parsing и validation.
3. Все agent-facing read operations имеют versioned JSON.
4. Batch context bundle заменяет N+1 calls.
5. Errors имеют stable codes и retryability.
6. Tool descriptions генерируются из schemas.
7. Profile не публикует запрещённые role tools.
8. Performance benchmark покрывает cold/warm MCP paths.

## Существующие issues, которые входят в программу

- #304 MCP latency;
- #353 CLI/MCP identity asymmetry;
- #374 missing JSON outputs;
- #397 read-only JSON projection gaps.
