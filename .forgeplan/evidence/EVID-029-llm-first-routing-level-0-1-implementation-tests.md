---
depth: standard
id: EVID-029
kind: evidence
links:
- target: PRD-019
  relation: informs
- target: PRD-020
  relation: informs
status: draft
title: LLM-first routing Level 0→1 implementation tests
---

## Summary

LLM-first Smart Routing (PRD-020) реализован: 3-уровневая система.

## Results

- 444 теста pass (16 новых routing тестов)
- 8 файлов изменено, 8 коммитов
- Level 0: keywords (offline, <10ms) — fallback
- Level 1: LLM classify (Gemini/OpenAI/Claude/Ollama, 2-5 sec)
- Level 2: FPF ADI reasoning (auto для Deep/Critical, 5-15 sec)
- 15s timeout Level 1, 30s timeout Level 2, graceful fallback
- Empty input guard (<3 chars → skip LLM)
- FPF KB context injection в route prompt
- MCP server: Level 1 + FPF context
- CLI: --level 0|1|2, auto-detect, auto-escalate
- Тестировано: русский, английский, китайский, пустой input, без workspace

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Test Matrix

| Scenario | Level | Depth | Status |
|----------|-------|-------|--------|
| Russian feature request | L1 | Standard | PASS |
| English typo fix | L1 | Tactical | PASS |
| Chinese auth system | L1 | Deep | PASS |
| OAuth2 RBAC (English) | L2 | Deep (confirmed) | PASS |
| ASCII tree command | L1 | Standard | PASS |
| No workspace | L0 | varies | PASS |
| Empty input | L0 | Tactical | PASS |
| Forced --level 0 | L0 | varies | PASS |
| health --compact | - | - | PASS (82 artifacts) |
| score --all | - | - | PASS (23/23 >= 0.5) |

## Bugs Found & Fixed

1. MCP server used only Level 0 → fixed: route_with_llm + FPF context
2. FPF KB not injected into route prompt → fixed: route_with_llm_and_context()
3. CLI didn't auto-detect Level 1 → fixed: auto-detect when config available

