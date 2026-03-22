# TODO — Forgeplan

## Current: v0.6.0 Released

### EPIC-001 — Forgeplan v1.0 Real Methodology Engine — COMPLETE ✅

All PRDs implemented:

- [x] **P0: Health + Blind Spots (PRD-003)** — `forgeplan health`, `forgeplan blindspots` (PR #17)
- [x] **P1: Decision Journal (PRD-004)** — `forgeplan journal` with R_eff (PR #18)
- [x] **P2: Validation v2 (PRD-005)** — depth-aware rules, 20 checks for Deep PRD (PR #19)
- [x] **P3: Smart Routing v2 (PRD-006)** — rule engine, 8 keyword triggers, no LLM (PR #20)
- [x] **P4: FPF Engine (PRD-002)** — F-G-R scoring, bounded contexts, explore-exploit, FPF dashboard
- [x] **Lifecycle (PRD-007)** — review, activate, supersede, deprecate with validation gates (PR #20)

### Stats
- 33 CLI commands
- 26 MCP tools
- 225 tests (194 core + 24 CLI + 7 other)
- 18 dogfood artifacts in LanceDB

## Backlog

### P0: Dogfooding & Polish
- [ ] Activate dogfood artifacts (review → activate for mature ones)
- [ ] `forgeplan validate --adversarial` mode (devil's advocate review)
- [ ] Custom prompts directory `.forgeplan/prompts/` (currently hardcoded in Rust)
- [ ] Fix NFR-002: binary 152MB → investigate LanceDB feature flags / strip

### P1: Desktop App (Phase 5)
- [ ] Tauri 2.0 + React frontend (shared Rust core)
- [ ] forgeplan-tauri crate

### P2: Integrations
- [ ] Bidirectional sync with task trackers (Linear, Jira, Orchestra)
- [ ] `.mcp.json` auto-configuration for Claude Code

## Done ✅

- [x] **Phase 0** — Foundation & Research (10/10)
- [x] **Phase 1** — Schemas, Templates & Docs (12/12)
- [x] **Phase 3A** — Core CLI: init, new, list, status (11 tests)
- [x] **Phase 3B** — Validate + Score + Link + Graph (106 tests)
- [x] **Phase 3C** — Search + Stale + Polish (29 tests)
- [x] **Phase 3D** — LanceDB Primary + Async (158 tests)
- [x] **Phase 4A** — MCP Server (11 tools)
- [x] **Phase 4B** — AI Features: generate, reason, decompose, capture, route, semantic search
- [x] **Phase 4C** — CRUD: get, update, delete
- [x] **v0.5.0** — Health, Journal, Validation v2 (PRD-003/004/005)
- [x] **v0.6.0** — Methodology Engine: routing, lifecycle, FPF, F-G-R (PRD-002/006/007)
