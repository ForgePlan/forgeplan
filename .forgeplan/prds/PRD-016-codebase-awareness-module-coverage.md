---
depth: tactical
id: PRD-016
kind: prd
status: active
title: Codebase Awareness — module coverage
---

# PRD-019: Codebase Awareness — module coverage, file-to-decision mapping

## Problem

Forgeplan управляет артефактами решений (PRD, ADR, RFC), но не знает о коде. Нет связи "этот модуль покрыт решением X". Нет способа ответить: "какие модули не имеют архитектурных решений?" или "какие решения затрагивают этот файл?". Quint-code решает это через module coverage — сканирует codebase и показывает тепловую карту: зелёное = решения есть, красное = blind spot.

## Goals

- [ ] Module scanning — forgeplan scan сканирует codebase и строит карту модулей
- [ ] File-to-decision mapping — каждый ADR/RFC с affected_files связан с модулями
- [ ] Coverage report — forgeplan coverage показывает % модулей с решениями
- [ ] Blind spot detection — модули без решений = architectural blind spots

## Non-Goals

- Code analysis (AST parsing, dependency injection) — только file-level mapping
- Auto-generate decisions for uncovered modules
- Real-time file watching

## Target Users

- Architects — видят какие части codebase не покрыты решениями
- Tech leads — forgeplan coverage в CI показывает architectural coverage
- AI agents — forgeplan_coverage MCP tool для informed decision-making

## Functional Requirements

### FR-001: Module Scanner
- [ ] forgeplan scan [--path <dir>] — сканирует codebase, строит карту модулей
- [ ] Распознаёт: Rust (crates/*/src/**), TypeScript (src/**), Python (src/**)
- [ ] Output: список модулей с path, line count, file count

### FR-002: File-to-Decision Mapping
- [ ] Парсит affected_files из active ADR/RFC
- [ ] Матчит glob patterns (src/scoring/*.rs) к реальным файлам
- [ ] Строит map: file → [decision_ids]

### FR-003: Coverage Report
- [ ] forgeplan coverage — показывает modules с/без решений
- [ ] Coverage %: modules_with_decisions / total_modules
- [ ] Per-module detail: PRD-001, ADR-002 cover this module

### FR-004: CLI + MCP
- [ ] forgeplan scan CLI command
- [ ] forgeplan coverage CLI command
- [ ] forgeplan_coverage MCP tool

## Related

- PRD-020 (Decision Contracts — affected_files field)
- PROB-010 (source utilization gap)
- Reference: sources/quint-code/src/mcp/cmd/coverage/

## Affected Files
- crates/forgeplan-core/src/coverage/**
- crates/forgeplan-cli/src/commands/coverage.rs

