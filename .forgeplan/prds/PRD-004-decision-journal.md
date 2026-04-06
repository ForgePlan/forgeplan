---
depth: tactical
id: PRD-004
kind: prd
status: active
title: Decision Journal
---

# PRD-004: Decision Journal

## Problem

Нет хронологического обзора принятых решений. ADR, Note, Problem, Solution разбросаны по list без timeline.

## Goals

- [ ] Timeline решений с R_eff scores
- [ ] Фильтрация по типу и уровню риска
- [ ] MCP tool

## Non-Goals

- Не визуализирует timeline
- Не редактирует артефакты

## Target Users

- Tech Lead — нет обзора принятых решений
- Architect — не видит рискованные решения

## Functional Requirements

- [x] FR-001: Chronological timeline of decision artifacts
- [x] FR-002: Filter by kind (--type adr)
- [x] FR-003: Risk filter (--risk)
- [x] FR-004: R_eff score, evidence count, stale flag per entry

## Related

- EPIC-001 (parent)
- NOTE-003 (datetime format issue)

## Affected Files
- crates/forgeplan-core/src/journal/**
- crates/forgeplan-cli/src/commands/journal.rs

