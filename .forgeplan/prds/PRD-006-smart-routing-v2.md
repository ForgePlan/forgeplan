---
depth: tactical
id: PRD-006
kind: prd
status: active
title: Smart Routing v2
---

# PRD-006: Smart Routing v2 — Rule-based Depth Engine

## Problem

Routing v1 использовал LLM — медленно, требует API key, не работает offline. Нужен детерминированный rule engine.

## Goals

- [ ] Rule-based routing: instant, offline, no LLM
- [ ] Keyword triggers + structural signals
- [ ] Confidence score

## Non-Goals

- LLM enrichment (optional)
- Custom keyword triggers (hardcoded)

## Target Users

- Developer — routing через LLM = 2-5 секунд
- AI Agent — нужен instant offline routing

## Functional Requirements

- [x] FR-001: Route task to depth+pipeline using rule engine
- [x] FR-002: Keyword signals (security→Deep, breaking→Deep)
- [x] FR-003: Structural signals (FR count, link count)
- [x] FR-004: Confidence from signal agreement
- [x] FR-005: Output: depth, pipeline, triggers, confidence
- [x] FR-006: Post-factum calibration

## Related

- EPIC-001 (parent)
- PRD-002 (routing = FR-001 of FPF)

## Affected Files
- crates/forgeplan-core/src/routing/**
- crates/forgeplan-cli/src/commands/route.rs

