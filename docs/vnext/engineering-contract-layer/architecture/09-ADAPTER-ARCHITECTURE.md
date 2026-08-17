# Adapter Architecture

## Типы расширений

- host_adapter;
- orchestrator_adapter;
- methodology_pack;
- evidence_provider;
- domain_pack;
- migration_pack;
- visualization_extension.

## Host adapter

Преобразует WorkContract и AuthorityPolicy в primitives конкретного agent host:

- instructions;
- skills;
- subagents;
- hooks;
- MCP wiring;
- permissions;
- event capture.

Не владеет contract или task state.

## Orchestrator adapter

Связывает external task/workspace/session/run с ForgePlan contract/execution/evidence/verdict.

Не создаёт второй backlog и не забирает ownership runtime.

## Capability manifest

Adapter обязан честно объявлять:

- supported host versions;
- MCP transports;
- skills;
- subagents;
- hooks;
- pre-edit enforcement;
- command enforcement;
- worktree awareness;
- headless/remote execution;
- event stream;
- resume;
- conformance status.

## SDK boundaries

Core предоставляет стабильные ports:

```text
ContractReader
ExecutionRegistrar
EvidenceSubmitter
VerdictReader
PolicyEvaluator
ExternalReferenceStore
```

Adapter-specific API и config не входят в core crates.
