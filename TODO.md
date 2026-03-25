# TODO — Forgeplan

## Current: v0.10.1 Released

### Stats
- ~35 CLI commands, 28 MCP tools, 318 tests
- 56 dogfood artifacts (43 active, 8 draft, 5 deprecated)
- 20,121 LOC Rust
- v0.10.1 tagged, PRs #35-#41 merged
- Health: clean (0 blind spots, 0 orphans, 0 at risk)

---

## Open Problems

| ID | Priority | Title | Status |
|----|----------|-------|--------|
| PROB-001 | Done | Data loss → solved by PRD-009 export/import | ✅ |
| PROB-002 | P2 | Auth reuse — separate API key barrier | Open |
| PROB-003 | Done | Dead statuses → solved by PRD-007 lifecycle | ✅ |
| PROB-004 | Done | Agent drift → solved by PRD-010 hooks | ✅ |
| PROB-005 | Done | Cold start → solved by PRD-012 init --scan | ✅ |

---

## Backlog (приоритизированный)

### P0: Broken
- [ ] **Embed feature fix** — fastembed API change broke `--all-features` (3 compiler errors)
  - Блокирует: semantic/vector search (PRD-018)

### P1: Distribution & Adoption
- [ ] brew tap formula (macOS)
- [ ] GitHub Actions release pipeline (cross-compile linux/windows/mac)
- [ ] Install script (`curl -fsSL https://forgeplan.dev/install.sh | sh`)
- [ ] `fpl` alias symlink in install
- [ ] Publish to crates.io (`cargo install forgeplan`)

### P2: Polish
- [ ] Binary size optimization (LanceDB feature flags / strip)
- [ ] Markdown projection sync on `forgeplan update` (not just `new`)
- [ ] fpf.rs миграция на common::store() (6 functions)
- [ ] coverage.rs, scan_import.rs миграция на common::open_store()

### P3: Integrations
- [ ] Bidirectional sync with task trackers (Linear, Jira, Orchestra)
- [ ] Export to GitHub Issues / Linear tasks

### Phase 5: Desktop App
- [ ] Tauri 2.0 + React frontend (shared Rust core)

---

## Done ✅

- [x] **v0.10.1** — Hotfix: bidirectional R_eff evidence lookup
- [x] **v0.10.0** — scan-import, --json x14, CLI UX, audit fixes, security hardening
- [x] **v0.9.0** — PRD-016..021: R_eff recursive, BMAD v2, OpenSpec DAG, FPF KB, codebase awareness, decision contracts
- [x] **v0.8.0** — CLI UX: cliclack init, styled output, setup-skill
- [x] **v0.7.0** — EPIC-001 complete, FPF engine, lifecycle, /forge skill
- [x] **v0.6.0** — Methodology Engine: routing, lifecycle, F-G-R
- [x] **v0.5.0** — Health, Journal, Validation v2
- [x] **Phase 4** — MCP Server + AI Features + CRUD
- [x] **Phase 3** — Core CLI + LanceDB Primary
- [x] **Phase 1** — Schemas, Templates & Docs
- [x] **Phase 0** — Foundation & Research

### v0.10.x detailed
- [x] PRD-012: Project Onboarding — init --scan, scan-import, 3-tier detection
- [x] PRD-008: CLI UX — 8 ui helpers, --json for 14 commands, common::store() (-128 LOC)
- [x] 4-agent Rust audit: UTF-8 safety, symlink protection, file size limits, import validation
- [x] R_eff bidirectional evidence fix (get_incoming_relations)
- [x] All blind spots (3), orphans (8), at risk (11) → resolved
- [x] 14 draft evidence → activated
