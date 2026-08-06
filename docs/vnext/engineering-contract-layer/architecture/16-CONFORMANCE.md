# Conformance Program

## Цель

Не заявлять поддержку host или orchestrator без автоматического подтверждения.

## Host Conformance v1

Проверяет:

- installation and discovery;
- MCP connectivity;
- skill loading;
- contract retrieval;
- path policy enforcement;
- forbidden command behavior;
- actor identity propagation;
- execution registration;
- Evidence submission;
- independent verification;
- error mapping;
- resume/retry behavior where supported;
- uninstall and cleanup.

## Semantic portability test

Один fixture WorkContract выполняется минимум в Cursor, Codex и OpenCode. Сравниваются:

- contract digest;
- allowed/forbidden behavior;
- changed path set;
- required Evidence;
- criterion verdicts;
- final VerificationVerdict.

## Orchestrator Conformance v1

Проверяет ownership boundary, external references, idempotency, retries, no duplicate tasks/runs, result collection и failure recovery.

## Capability levels

```text
full
partial
advisory
unsupported
experimental
```

## Публикация

Capability matrix и badges генерируются из последнего успешного run с датой, версиями и commit SHA.
