---
depth: standard
id: PROB-004
kind: problem
links:
- target: EPIC-001
  relation: informs
status: draft
title: Agent drift
---

# PROB-004: Agent Drift

## Problem Statement

AI агент дрифтит — не следует методологии. Создаёт заглушки, не проверяет depth, не создаёт evidence.

## Signal

- Stub PRD после forgeplan new
- Код без forgeplan route
- Нет evidence после реализации

## Impact

High — Forgeplan теряет ценность если агент обходит методологию.

## Proposed Direction

CLAUDE.md Rules + Hooks + /forge skill. → SOL-001.
