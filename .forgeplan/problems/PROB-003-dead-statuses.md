---
depth: standard
id: PROB-003
kind: problem
links:
- target: PRD-007
  relation: informs
status: draft
title: Dead statuses
---

# PROB-003: Dead Statuses

## Problem Statement

Артефакты создаются в Draft и навсегда остаются в Draft. Нет enforce mechanism. Draft PRD с полной реализацией и Active PRD без кода выглядят одинаково.

## Signal

- forgeplan list: 15+ draft, 0 active
- Health dashboard не отличает в работе от забыто

## Impact

High — подрывает доверие к системе.

## Proposed Direction

Lifecycle commands: review → activate → supersede/deprecate. → PRD-007.
