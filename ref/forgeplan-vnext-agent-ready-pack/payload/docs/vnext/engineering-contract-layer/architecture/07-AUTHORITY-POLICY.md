# Authority and Policy Engine

## Цель

Перенести критические ограничения из prompts/hooks в проверяемую domain policy.

## Actor roles

- human_principal;
- planner_agent;
- builder_agent;
- verifier_agent;
- orchestrator;
- ci;
- service;
- policy_bot.

## Actions

- artifact.create;
- artifact.modify;
- decision.bind;
- contract.compile;
- contract.approve;
- contract.expand_scope;
- execution.register;
- execution.claim;
- evidence.submit;
- evidence.dismiss;
- evidence.accept;
- artifact.activate;
- merge.approve;
- deployment.authorize.

## Depth profiles

### Tactical

Автоматическое исполнение и acceptance разрешены при полном deterministic verification.

### Standard

Agent исполняет; ForgePlan и CI проверяют; human approval зависит от policy проекта.

### Deep

Builder и verifier обязаны быть разными actor instances.

### Critical

Human principal утверждает contract и final verdict. Agent не может bind decision или принять собственное Evidence.

## Enforcement levels

- `full` — host adapter технически блокирует нарушение;
- `core` — действие отклоняется ForgePlan Core;
- `ci` — нарушение блокирует merge;
- `advisory` — только предупреждение;
- `unsupported` — host не может обеспечить правило.

Capability matrix обязана показывать реальный уровень.

## Audit

Authority decisions append-only и содержат actor, action, resource, policy version, decision, reason, timestamp и trace ID.
