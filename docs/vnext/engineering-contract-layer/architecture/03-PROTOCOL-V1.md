# ForgePlan Protocol v1

## Назначение

Protocol v1 является переносимым контрактом между ForgePlan Core, agent hosts, orchestrators, evidence providers и Web.

## Версионирование

Каждый документ содержит:

```json
{
  "schema": "forgeplan.work-contract.v1",
  "protocol_version": "1.0.0"
}
```

Правила:

- patch — совместимые исправления и новые optional fields;
- minor — совместимые расширения semantics;
- major — breaking schema или semantic changes;
- consumers обязаны отклонять неизвестную major version;
- unknown optional fields должны сохраняться при round-trip;
- canonical JSON используется для digest/signature.

## Основные типы

### ArtifactReference

Стабильная ссылка на artifact, git revision и optional content digest.

### WorkContract

Скомпилированная проекция инженерного замысла для конкретного исполнения.

### ExecutionReceipt

Ссылка на фактическое исполнение во внешнем host/orchestrator.

### EvidenceBundle

Проверяемый набор результатов с provenance.

### VerificationVerdict

Результат проверки контракта, claims и Evidence.

### AuthorityPolicy

Правила полномочий по actor roles и actions.

### CapabilityManifest

Честное описание возможностей host/adapter и уровня enforcement.

## Идентичность

Любой actor имеет:

```text
actor_id
actor_type
provider
instance_id
version
```

`actor_id` является непрозрачным стабильным ID. Display name не используется как authority key.

## Корреляция

Все операции используют:

```text
trace_id
idempotency_key
contract_id
execution_id
actor_id
external references
```

## Ошибки

Machine-readable envelope:

```json
{
  "schema": "forgeplan.error.v1",
  "code": "SCOPE_VIOLATION",
  "message": "Changed path is outside the contract scope",
  "retryable": false,
  "details": {}
}
```

## Security

- contracts и verdicts получают digest;
- secrets передаются только как references;
- Evidence artifacts могут храниться снаружи, но hash и metadata сохраняются в ForgePlan;
- authority decisions являются append-only audit events;
- adapter не может скрывать unsupported capability.
