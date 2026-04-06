---
depth: tactical
id: PRD-007
kind: prd
status: active
title: Artifact Lifecycle
---

# PRD-007: Artifact Lifecycle

## Problem

Артефакты вечно в Draft — нет механизма перевода в Active/Superseded/Deprecated. "Active" PRD без реализации = ложное обещание.

## Goals

- [ ] State machine: Draft → Active → Superseded/Deprecated
- [ ] Validation gate при activation
- [ ] Chain warnings при supersede
- [ ] Lightweight kinds без validation gate

## Non-Goals

- Автоматические transitions
- Undo transitions

## Target Users

- Developer — забывает активировать готовые PRD
- Tech Lead — не видит какие PRD живые vs заброшенные

## Functional Requirements

- [x] FR-001: Review artifact (forgeplan review)
- [x] FR-002: Activate (Draft→Active) with validation gate
- [x] FR-003: Supersede (Active→Superseded) with replacement link
- [x] FR-004: Deprecate (Active→Deprecated) with reason
- [x] FR-005: Chain warnings for dependents
- [x] FR-006: Lightweight activation for Notes/Problems

## Related

- EPIC-001 (parent)
- PROB-003 (Dead Statuses — this PRD solves it)

## Affected Files
- crates/forgeplan-core/src/lifecycle/**
- crates/forgeplan-cli/src/commands/activate.rs

