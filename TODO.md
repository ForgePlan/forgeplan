# TODO — Forgeplan

## Current: v0.11.0 Released

### Stats
- ~37 CLI commands (tree, context, scan-import, coverage --backfill), 28 MCP tools, 428 tests
- 77 dogfood artifacts (51 active, 20 draft, 6 deprecated)
- ~21K LOC Rust
- v0.11.0 tagged, PRs #35-#55 merged
- 0 compiler warnings, 5 enforcement hooks

### P0: Integrity Issues (PROB-012 dogfood audit) ✅
- [x] **Semantic search broken** — feature flag propagated CLI→core via Cargo.toml
- [x] **R_eff divergence** — update_r_eff_score() persists to LanceDB, NaN guard
- [x] **health vs journal inconsistency** — expanded blind spots + decision kinds aligned
- [x] **coverage 0%** — Affected Files section added to templates + backfilled 18 active PRD/RFC/ADR
- [x] **route underestimates** — 4 keyword triggers + 2 heuristics added

Fixed in commit d84bc69 (fix/prob-012-integrity-remediation). 2 audit rounds, 403 tests.

---

## Open Problems

| ID | Priority | Title | Status |
|----|----------|-------|--------|
| PROB-001 | Done | Data loss → solved by PRD-009 export/import | ✅ |
| PROB-002 | P2 | Auth reuse — separate API key barrier | Open |
| PROB-003 | Done | Dead statuses → solved by PRD-007 lifecycle | ✅ |
| PROB-004 | Done | Agent drift → solved by PRD-010 hooks | ✅ |
| PROB-005 | Done | Cold start → solved by PRD-012 init --scan | ✅ |
| PROB-006 | Done | Routing misses UX scope → solved by PROB-012 keyword expansion | ✅ |
| PROB-009 | Deprecated | F-G-R Granularity → future PRD scope | ⚠️ |
| PROB-010 | Tracked | Markdown projections not updated → design decision (ADR-002) | 📋 |
| PROB-012 | Done | Feature integrity gap → 5 fixes, 2 audit rounds | ✅ |
| PROB-013 | Done | R_eff includes deprecated/draft in chain → skip non-active | ✅ |

---

## Backlog (приоритизированный)

### P1: Embed & Distribution
- [ ] **Embed feature fix** — fastembed API v5 broke `--all-features` (upstream dep, feature flag propagation fixed in PROB-012)
  - Блокирует: semantic/vector search (PRD-018)
  - Downgraded from P0→P1: feature flag chain fixed, actual fix depends on fastembed upstream

### P1: Distribution & Adoption
- [ ] brew tap formula (macOS)
- [ ] GitHub Actions release pipeline (cross-compile linux/windows/mac)
- [ ] Install script (`curl -fsSL https://forgeplan.dev/install.sh | sh`)
- [ ] `fpl` alias symlink in install
- [ ] Publish to crates.io (`cargo install forgeplan`)

### P2: Integrity Follow-up (from FPF audit) ✅
- [x] **Read-back verify** in update_r_eff_score — pre-check with get_record before update
- [x] **DRY decision_kinds** — DECISION_KINDS_EVIDENCE + DECISION_KINDS_JOURNAL in types.rs
- [x] **Coverage batch-update** — `forgeplan coverage --backfill` (18 artifacts updated)
- [x] **PROB-013** — R_eff skip deprecated/draft in recursive chain (ADR-002)
- [x] **Tree visual** — evidence/note show `··` instead of `0.00`
- [x] **METHODOLOGY-COURSE.md** — Chapter 8 added (tree, coverage, hooks, R_eff rules)
- [ ] **PRD-019 Layer 3** — MCP session state machine (next sprint)

### P2: Route & Enforcement (from usability testing)
- [x] **Route gap**: added "new command/feature" keywords (English)
- [x] **Batch score CLI**: `forgeplan score --all` implemented
- [ ] **LLM-first route**: replace keyword matching with LLM classification (supports any language, no declension issues)
- [ ] **PRD-019 Layer 3**: MCP session state machine — агент не может пропустить Shape phase
- [ ] **Duplicate notes cleanup**: NOTE-004 и NOTE-005 → deprecate один

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
