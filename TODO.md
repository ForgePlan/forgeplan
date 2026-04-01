# TODO — Forgeplan

## Current: v0.12-dev (post-v0.12.0)

### Stats
- 52 CLI commands, 35 MCP tools, ~693 tests
- 109 dogfood artifacts (68 active, 26 draft, 13 deprecated)
- ~25K LOC Rust, 41MB release binary
- PRs #60-#84 merged
- E2E smoke test: 193 tests, 92.7% pass rate (179 PASS, 8 FAIL, 6 SKIP)
- Smart search by default (keyword + semantic + graph boosters)
- MCP methodology hints (_next_action in tool responses)
- 3-level routing: L0 keywords, L1 LLM classify, L2 FPF ADI reasoning
- Estimate engine: multi-grade effort scoring (PRD-022, RFC-005, ADR-004)
- MemoryDriver: remember/recall commands (RFC-003 Phase 2)

### P0: E2E Bug Fixes — Sprint 2 (PROB-018)
- [x] **BUG-001 (P1 Security):** `scan --path /tmp` path traversal — added project root boundary validation (coverage.rs)
- [x] **BUG-002 (P2):** `unlink` не проверяет существование связи — added existence check (link.rs)
- [x] **BUG-003 (P3):** lifecycle transition message "draft → active" hardcoded — uses old_status (activate.rs)
- [x] 2 new unit tests for delete_relation (store.rs)
- [x] Create Evidence EVID-040 + link to PROB-018
- [x] Audit (4-agent: logic+security+rust+task) + PR #85 merged

### P0: Estimate Engine (PRD-022) ✅
- [x] PRD-022 shaped + validated (8 FR, 3 journeys)
- [x] RFC-005 architecture (3 phases, 12 tasks)
- [x] ADR-004 hybrid approach (rule-based L0 + LLM L1)
- [x] Phase 1: types, extractor, scorer, calculator, display (35 tests)
- [x] Phase 2: confidence, CLI estimate command (39 tests)
- [x] 2-agent audit: 2 CRITICAL + 4 HIGH fixed (50 tests)
- [x] Config integration: grade_profile, multipliers, --my-grade
- [x] FORGEPLAN-GUIDE: Estimate Engine section
- [x] Evidence EVID-036 linked, PRD-022 + RFC-005 + ADR-004 activated

### P0: MemoryDriver (RFC-003 Phase 2) ✅
- [x] remember/recall CLI commands
- [x] ArtifactKind::Memory with mem- prefix
- [x] DRY helpers in common.rs + 3 tests

### P0: Quality Cycle Sprint (v0.12.0) ✅
- [x] 17 MUST gaps → 0 (enable --depth update)
- [x] Evidence parser: ignores template placeholders (CL0→CL3)
- [x] LLM scorer (RFC-005 Phase 3): --llm-score flag, PR #79
- [x] Hints system: 9 commands, 11 tests, shared hints.rs
- [x] Domain inference: frontmatter → keywords → LLM (3-level)
- [x] Manual complexity override: --complexity "FR-001=8"
- [x] Spec/Evidence confidence boost: +15%/+20%
- [x] forgeplan link body reset fix (PR #75)
- [x] dotenvy for .env API keys
- [x] Template FR filtering, keyword TaskType improvements
- [x] 40 commands smoke-tested
- [x] Release v0.12.0 tagged + installed

### P0: Integrity Issues (PROB-012 dogfood audit) ✅
- [x] **Semantic search broken** — feature flag propagated CLI→core via Cargo.toml
- [x] **R_eff divergence** — update_r_eff_score() persists to LanceDB, NaN guard
- [x] **health vs journal inconsistency** — expanded blind spots + decision kinds aligned
- [x] **coverage 0%** — Affected Files section added to templates + backfilled 18 active PRD/RFC/ADR
- [x] **route underestimates** — 4 keyword triggers + 2 heuristics added

Fixed in commit d84bc69 (fix/prob-012-integrity-remediation). 2 audit rounds, 403 tests.

---

## Known Issues
- [ ] **changelog commit_hash**: LanceDB schema migration — old workspaces lack `commit_hash` column. `forgeplan update` logs warning. Fix: reinit workspace or add column migration.
- [x] **RFC-005 3.2**: estimate MCP tool — grade param wired (Sprint 1 housekeeping).
- [ ] **1 STUB artifact**: unidentified, low priority.
- [ ] **e2e_coverage_backfill test**: pre-existing failure, unrelated to v0.12 changes.
- [x] **Self-link guard** (PROB-019): `link X X` rejected with "Self-link not allowed" (Sprint 3).
- [x] **Runbook outdated** (NOTE-031): deprecated — file doesn't exist in repo, discrepancies noted in TODO.
- [ ] **LLM tests not run**: Wave 10 tests 10.1-10.5 skipped (no API key configured).
- [ ] **--semantic feature flag**: `search --semantic` fails at runtime if not compiled with `semantic-search`.

---

## Open Problems

| ID | Priority | Title | Status |
|----|----------|-------|--------|
| PROB-001 | Done | Data loss → solved by PRD-009 export/import | ✅ |
| PROB-002 | P2 | Auth reuse — separate API key barrier | Open |
| PROB-003 | Done | Dead statuses → solved by PRD-007 lifecycle | ✅ |
| PROB-004 | Done | Agent drift → solved by PRD-010 hooks | ✅ |
| PROB-005 | Done | Cold start → solved by PRD-012 init --scan | ✅ |
| PROB-006 | Deprecated | Routing misses UX scope → fixed v0.11, keywords expanded | ✅ |
| PROB-009 | Deprecated | F-G-R Granularity → future PRD scope | ⚠️ |
| PROB-010 | Deprecated | Markdown projections → fixed by RFC-004 files-first | ✅ |
| PROB-012 | Deprecated | Feature integrity gap → 5 fixes, 2 audit rounds | ✅ |
| PROB-013 | Deleted | R_eff skip non-active → implemented in ADR-002, deleted | ✅ |
| PROB-014 | Deprecated | Smart search gaps → fixed v0.12, real cosine | ✅ |
| PROB-016 | Deprecated | CLI quality → 13 fixes, 6-agent audit | ✅ |
| PROB-018 | Done | E2E Smoke Test Findings — 3 bugs fixed, 4-agent audit, PR #85 | ✅ |
| PROB-019 | Deprecated | Self-link guard added — case-insensitive check, 4 tests | ✅ |

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
- [ ] **PRD-019 Layer 3** — MCP session state machine (backlog, PRD-019 activated)

### P2: Route & Enforcement (from usability testing)
- [x] **Route gap**: added "new command/feature" keywords (English)
- [x] **Batch score CLI**: `forgeplan score --all` implemented
- [x] **LLM-first route**: 3-level routing (L0 keywords, L1 LLM classify, L2 FPF ADI reasoning) — PRD-020, 444 tests
- [ ] **PRD-019 Layer 3**: MCP session state machine — агент не может пропустить Shape phase
- [x] **Duplicate notes cleanup**: NOTE-004 deprecated (duplicate of NOTE-005)

### P1: Smart Search (PROB-014, v0.12)
- [x] **F1 (P0)**: Embed title + body snippet — embedding_text() + forgeplan embed command
- [x] **F2 (P0)**: Graph walk shows relation types — neighbors_with_relations()
- [x] **F3 (P1)**: `forgeplan embed` — batch embed all artifacts (persistent in LanceDB)
- [x] **F4 (P1)**: Smart search — text-first + boosters (Algolia-style, not weighted sum). EVID-033.
- [x] **F5 (P1)**: `forgeplan gaps` — 18 MUST gaps found on real data! Audit: 4 fixes.
- [ ] **F6 (P2)**: Fix evidence blind spots (EVID-015, EVID-025, EVID-026, EVID-027)
- [x] `forgeplan search --semantic` — vector-only search
- [x] `forgeplan search` — smart by default (keyword + semantic + R_eff + status + graph boosters)
- [x] `forgeplan search --keyword` — forced keyword grep
- [x] Configurable embedding model via config.yaml (BGE-M3 default)
- [x] Configurable chunk_size via config.yaml (default 2000)
- [ ] **Future**: Reciprocal Rank Fusion (RRF) for production-grade hybrid search

### P0: CLI Quality Remediation (PROB-016, 3-agent deep audit) ✅
**Wave 1 — BROKEN** (6-agent team sprint, PR #65):
- [x] **B1**: `deprecate --reason` stores reason in body (## Deprecation section)
- [x] **B2**: `update --status active` blocked — must use `forgeplan activate`
- [x] **B3**: 4 LLM commands — pre-flight API key check via `require_llm_config()`
- [x] **B4**: `review` checks evidence+stub gates (same as activate)

**Wave 2 — SAFETY**:
- [x] **N1**: `delete` checks dependents, warns + requires --yes
- [x] **N7**: `supports` added to VALID_RELATIONS
- [x] **N8**: `init --force` warns about data loss
- [x] **N9**: `unlink` updates projection

**Wave 3 — CORRECTNESS**:
- [x] **N2**: `new` depth=tactical for note/evidence/problem/solution/refresh
- [x] **N3**: `supersede` warns if replacement already superseded/deprecated
- [x] **N4**: `score --all --json` clean JSON output
- [x] **N5**: `update --depth` bails with error
- [x] **N6**: `order/blocked` structural relations only (informs doesn't block)

EVID-034. 532 tests. **Deferred**: fgr/blindspots redundancy, graph filtering, drift adoption, export embeddings

### P1: Driver Abstraction — RFC-003
- [x] **Phase 1**: StorageDriver trait + LanceDriver + InMemoryStore + factory — PR #61 merged
- [x] **Phase 1 audit**: 3 agents, 13 findings, 7 fixed (C1-C3, H1, H3, M1, M2)
- [ ] **Phase 1 deferred** (PROB-015): H2 EmbedDriver, H4 ISP split, M3-M5, test gaps
- [x] **Phase 2**: MemoryDriver (remember/recall) — PR #72 merged
- [ ] **Phase 3**: SQLite driver + feature flags
- [ ] **Phase 4**: Config-driven selection + forgeplan init shows drivers

### P2: Architecture — Files as Source of Truth (ADR-003, RFC-004) ✅
- [x] Invert direction: .md files = truth, LanceDB = index (RFC-004 Phase 1, PR #67)
- [x] File watcher (notify crate) for auto-reindex (RFC-004 Phase 2, PR #69)
- [x] `forgeplan reindex` — one-time full re-sync from .md files (PR #71)
- [x] R_eff computed on-the-fly from evidence files (not stored)
- [x] Links in frontmatter `related:` field (RFC-004 Phase 1)
- [x] Change log tracking (RFC-004 Phase 3, PR #69)
- [x] Git-sync integration (RFC-004 Phase 4, PRs #80-#81)

### P2: Polish
- [x] Binary size optimization — release profile 163MB→41MB (-75%)
- [ ] fpf.rs миграция на common::store() (6 functions)
- [ ] coverage.rs, scan_import.rs миграция на common::open_store()

### P2: CLI UX Polish (NOTE-029, from E2E findings)
- [ ] `forgeplan links PRD-001` — show all relations for an artifact (1 day)
- [ ] `forgeplan validate --ci` — exit 1 on MUST errors for CI/CD (1 day)
- [ ] `forgeplan doctor` — check workspace, LLM key, feature flags (2 days)
- [ ] Document `capture` as LLM-dependent in --help
- [ ] Error consistency: choose idempotent vs strict philosophy
- [ ] Document case-sensitive IDs

### P2: Agent Memory Engine (NOTE-025, Direction A — HIGH R_eff)
- [ ] Test `forgeplan serve` as MCP server in Claude Code
- [ ] Claude Code plugin: /fp-validate, /fp-context, /fp-score skills
- [ ] `capture` offline mode (create Note/ADR without LLM)
- [ ] `forgeplan watch --emit-events` — JSON event stream for agents

### P2: CI/CD Architecture Linter (NOTE-026, Direction B)
- [ ] `forgeplan health --fail-on` — configurable thresholds
- [ ] GitHub Action: `uses: forgeplan/action@v1`

### P3: Ruflo/Gastown Integration (NOTE-027)
- [ ] MCP config example for Ruflo (.agents/config.toml)
- [ ] Architecture-guardian custom agent YAML
- [ ] Gastown directive template

### P3: Task Tracker Bridges (NOTE-028)
- [ ] Bidirectional sync with task trackers (Linear, Jira, Orchestra)
- [ ] Export to GitHub Issues / Linear tasks
- [ ] Webhook on activate/supersede events

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
