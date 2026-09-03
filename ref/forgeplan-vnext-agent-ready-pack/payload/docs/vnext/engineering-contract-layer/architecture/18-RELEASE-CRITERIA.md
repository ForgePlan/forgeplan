# Program Release Criteria

## Product

- одно каноническое определение во всех surfaces;
- явно описано, что ForgePlan не заменяет;
- three usage tiers доступны с working quickstarts.

## Protocol

- schemas versioned and published;
- canonical serialization and digest defined;
- compatibility policy documented;
- protocol fixtures available.

## Core

- deterministic contract compilation;
- core-side authority checks;
- git provenance verification;
- criterion-level Evidence verification;
- CLI/MCP parity;
- stable machine-readable errors.

## Agents

- Cursor, Codex и OpenCode прошли Host Conformance v1;
- unsupported capabilities показаны честно;
- builder не принимает Critical result;
- scope violation блокируется минимум core/CI.

## Orchestrators

- external task/workspace/run IDs сохраняются;
- ForgePlan не дублирует external state;
- retries idempotent;
- completion не равен acceptance.

## Web

- contract, execution, Evidence и verdict доступны read-only;
- PR graph delta доступен;
- нет mutation/task/runtime функций.

## Documentation

- examples tested;
- reference generated;
- cross-repo drift checks green;
- current/planned разделены;
- version matrix generated from conformance.
