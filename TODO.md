# TODO — Forgeplan

## Current: v0.11.0 Released

### Stats
- ~37 CLI commands (tree, context, scan-import), 28 MCP tools, 392 tests
- 71 dogfood artifacts (49 active, 17 draft, 5 deprecated)
- ~21K LOC Rust
- v0.11.0 tagged, PRs #35-#52 merged
- 0 compiler warnings

### P0: Integrity Issues (PROB-012 dogfood audit)
- [ ] **Semantic search broken** — feature flag not propagated CLI→core
- [ ] **R_eff divergence** — tree shows stored 0.00 vs computed 1.0 (score/context)
- [ ] **health vs journal inconsistency** — health says healthy, journal shows 23 without evidence
- [ ] **coverage 0%** — no artifacts have affected_files sections
- [ ] **route underestimates** — complex tasks rated Tactical

Next: PRD for remediation → ADR for R_eff source-of-truth → phased fixes

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
