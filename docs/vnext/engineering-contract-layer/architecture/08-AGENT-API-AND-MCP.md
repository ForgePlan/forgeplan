# Agent API and MCP v2

> Предложение Agent API v2, не текущий каталог MCP. Для выбранной сборки `0.39.0+constellation.s1.1` (`c852f40c`) доступны `full` и отдельный `constellation-read`, а не перечисленные ниже role profiles. Полный профиль сохраняет мутации. См. [статус пакета](../README.md).

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

Имена выше проектируемые. Совпадение с существующими `forgeplan_claim`, `forgeplan_status`, `forgeplan_search` не вводит новую семантику: текущий claim — advisory-лиза артефакта, не evidence claim и не authority grant. Migration/parity должны сохранять действующих callers; этот список не разрешает заменить их интерфейсы.

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
