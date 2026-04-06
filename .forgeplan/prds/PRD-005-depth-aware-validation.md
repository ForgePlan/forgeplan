---
depth: tactical
id: PRD-005
kind: prd
status: active
title: Depth-Aware Validation
---

# PRD-005: Validation v2 — Depth-Aware Rules

## Problem

Валидация v1 не учитывает depth — одинаковые правила для Tactical Note и Deep PRD. Нет проверки risk, rollback, dependencies для Deep PRD.

## Goals

- [ ] Depth-aware validation: разные наборы правил по depth
- [ ] Расширенные правила для Deep PRD
- [ ] Подготовка к adversarial mode

## Non-Goals

- Adversarial mode — planned separately
- Автоматическое исправление

## Target Users

- Developer — тактический PRD получает лишние warnings
- Architect — Deep PRD не проверяет risk и rollback

## Functional Requirements

- [x] FR-001: Different rule sets based on depth (Tactical/Standard/Deep)
- [x] FR-002: Deep PRD validates risk section
- [x] FR-003: Deep PRD validates rollback/mitigation
- [x] FR-004: Deep PRD validates success_metrics
- [x] FR-005: Deep PRD validates dependencies
- [x] FR-006: FR format check [Actor] can [capability]

## Related

- EPIC-001 (parent)

## Affected Files
- crates/forgeplan-core/src/depth/**
- crates/forgeplan-core/src/validation/**

