# TODO — Forgeplan

## Current: v0.7.0 Released

### EPIC-001 — Forgeplan v1.0 Real Methodology Engine — COMPLETE

- [x] PRD-002: FPF Engine (F-G-R, bounded contexts, explore-exploit)
- [x] PRD-003: Health Dashboard + Blind Spots
- [x] PRD-004: Decision Journal
- [x] PRD-005: Validation v2 (depth-aware rules)
- [x] PRD-006: Smart Routing v2 (rule engine, no LLM)
- [x] PRD-007: Artifact Lifecycle (review, activate, supersede, deprecate)

### Stats
- 33 CLI commands, 28 MCP tools, 225 tests
- 21 dogfood artifacts (6 active, 15 draft, 1 evidence)

---

## Next: CLI UX Redesign (PRD-008) — PLANNED

### P0: cliclack interactive init
- [ ] FR-001: ASCII banner FPL при init и --version
- [ ] FR-002: Interactive wizard: name → agents → .mcp.json → spinner → summary
- [ ] FR-003: Auto-generate .mcp.json for selected agents
- [ ] FR-004: Auto-add Forgeplan section to CLAUDE.md
- [ ] FR-005: Auto-generate .cursorrules (if Cursor selected)
- [ ] FR-012: `forgeplan setup-skill` — install /forge to ~/.claude/skills/

### P1: Styled CLI output
- [ ] FR-006: health — note boxes, colored statuses, icons
- [ ] FR-007: validate — colored MUST/SHOULD/COULD
- [ ] FR-008: review — styled checklist
- [ ] FR-009: route — depth colors (tactical=green, deep=red)
- [ ] FR-010: list — colored table by status

### P2: Machine-readable
- [ ] FR-011: All commands support --json for scripting/MCP

### Bugs
- [ ] PROB-006: Routing misses "redesign/overhaul/refactor" scope — add keyword triggers

---

## Backlog

### Polish
- [ ] `forgeplan validate --adversarial` mode
- [ ] Custom prompts `.forgeplan/prompts/`
- [ ] Binary size 152MB → investigate feature flags / strip
- [ ] `forgeplan init --scan` — detect existing docs/ADRs and offer import

### Distribution
- [ ] brew tap formula (macOS)
- [ ] GitHub Actions release pipeline (cross-compile)
- [ ] Install script (`curl | sh`)
- [ ] `fpl` alias symlink in install

### Desktop App (Phase 5)
- [ ] Tauri 2.0 + React frontend (shared Rust core)

### Integrations
- [ ] Bidirectional sync with task trackers (Linear, Jira, Orchestra)

---

## Done

- [x] **v0.7.0** — EPIC-001 complete, 6 PRDs, dogfood lifecycle, FORGEPLAN-GUIDE, /forge skill
- [x] **v0.6.0** — Methodology Engine: routing, lifecycle, FPF, F-G-R
- [x] **v0.5.0** — Health, Journal, Validation v2
- [x] **Phase 4** — MCP Server + AI Features + CRUD
- [x] **Phase 3** — Core CLI + LanceDB Primary
- [x] **Phase 1** — Schemas, Templates & Docs
- [x] **Phase 0** — Foundation & Research
