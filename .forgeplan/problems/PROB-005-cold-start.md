---
depth: standard
id: PROB-005
kind: problem
links:
- target: EPIC-001
  relation: informs
status: deprecated
title: Cold start
---

# PROB-005: Cold Start

## Problem Statement

Новый чат не знает контекст проекта. Нет быстрого bootstrap.

## Signal

- Каждый чат: расскажи о проекте
- Нет compact summary

## Impact

Medium — замедляет старт сессии.

## Proposed Direction

forgeplan health --compact --json + SessionStart hook. → SOL-001.
