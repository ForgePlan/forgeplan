---
depth: standard
id: PRD-003
kind: prd
status: active
title: Health Dashboard + Blind Spot Detection
---

# PRD-003: Health Dashboard + Blind Spots

## Problem

Нет единого обзора состояния проекта. Разработчик не видит: сколько артефактов в draft vs active, какие решения без evidence (blind spots), какие артефакты не связаны с другими (orphans), какие протухли (stale).

## Goals

- [ ] Единый dashboard (forgeplan health)
- [ ] Compact формат для hooks (forgeplan health --compact --json)
- [ ] Blind spots detection (forgeplan blindspots)
- [ ] MCP tools

## Non-Goals

- Не показывает содержимое артефактов
- Не исправляет проблемы автоматически

## Target Users

- Developer — не знает с чего начать сессию
- AI Agent — нужен compact JSON на session start

## Functional Requirements

- [x] FR-001: Aggregated health dashboard with counts by kind and status
- [x] FR-002: Blind spots — active artifacts without evidence
- [x] FR-003: Orphans — artifacts with no relations
- [x] FR-004: At-risk artifacts — low R_eff scores
- [x] FR-005: Compact JSON output for hooks
- [x] FR-006: Stale artifact count
- [x] FR-007: Next actions based on findings

## Related

- EPIC-001 (parent)

## Affected Files
- crates/forgeplan-core/src/health/**
- crates/forgeplan-cli/src/commands/health.rs
