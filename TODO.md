# TODO — Forgeplan

> **Roadmap**: see [`docs/ROADMAP.md`](docs/ROADMAP.md) for full gap analysis by category
> (Architecture 85%, UX 70%, Performance 80%, Distribution 65%, Docs 60%, Integrations 55%).

## Current: v0.18.0 released 2026-04-11

### Next priorities (from ROADMAP)
- [ ] Sprint A: Public Presence — Website (PRD-024) + README + crates.io + Docker
- [ ] Sprint B: CI/CD Integration — validate/health --ci + GH Action
- [ ] Sprint C: Desktop App — EPIC-004 (Tauri + React)
- [ ] Sprint D: Ecosystem — VS Code ext + GitHub Issues bridge

### Open bugs
- [ ] PROB-026: tag canonicalization (PR #169 pending merge)
- [ ] PROB-027: reindex without lance/ (PR #169 pending merge)
- [ ] PROB-035 remainder: code-fence awareness in extract_field

---

## Previous: v0.17.2 quality hotfix 2026-04-09 — E2E verification sprint

### v0.17.2 hotfix P0
- [x] PROB-030 BM25 prefix fallback (smart.rs `max(bm25_norm, kw)`)
- [x] PROB-031 score.rs imports core parser (deleted local duplicate with CL0 default)
- [x] PROB-032 breakdown display (auto-fixed by PROB-030)
- [x] PROB-033 `new evidence` phase-agnostic (no session warning)
- [x] **PROB-034 CRITICAL** multi-line HTML comment state machine in extract_field
- [x] F1 fail-closed on unclosed `<!--` (unclosed → CL0 + warn)
- [x] F2 fail-closed on unparseable congruence_level (garbage → CL0 + warn)
- [x] evidence template simplified (single-line comments only, no booby-trap)
- [x] 12 new regression tests (total 1143 pass, +12 from 1131)
- [x] Cargo.toml workspace + cross-crate refs → 0.17.2
- [x] CHANGELOG.md v0.17.2 entry
- [x] CLAUDE.md status block updated to v0.17.2
- [x] TODO.md updated (this block)
- [x] 4-agent audit completed (A code, B tests, C security, D docs)
- [x] All audit blockers addressed in-scope
- [x] PROB-034 card + EVID-068..072 created
- [x] EVID-068..072 Interpretation + CL Justification filled (audit D)
- [x] PROB-030..034 + EVID-068..072 activated
- [x] PR release/v0.17.2 → main (#163 merged)
- [x] Tag v0.17.2 + push (cargo-dist Release workflow success, brew formula published)
- [x] Sync main → dev via PR (#164 merged)
- [x] Health debt cleaned: 8 stubs deprecated/superseded, 5 dup EVIDs deprecated, 3 orphan NOTEs linked
- [ ] Hindsight retain v0.17.2 finale
- [ ] PROB-035 "extract_field hardening" filed for follow-up sprint (code-fence, token-boundary substring)

### v0.17.1 hotfix ✅ (shipped)
- [x] PROB-028 phantom rows (PRD-044)
- [x] PROB-029 health verdict (PRD-045)
- [x] CHANGELOG + tag + cargo-dist + sync done

---

## Previous: v0.17.0-rc — EPIC-003 complete, ready to tag

### Stats (v0.17.0)
- ~56 CLI commands, ~47 MCP tools, **1109 tests** (+280 from v0.16)
- Workspace: 0 warnings on both default and `--features semantic-search`
- ~13.8K LOC added across EPIC-003 (Sprints 13.0 → 13.7 + post-closeout hotfix)
- PRs #141-#156
- E2E: sprint-13.6-regression.sh (16 checks) + sprint-13.7-regression.sh (16 + SEMANTIC_E2E opt-in), 0 failures
- LLM: gemini-3-flash-preview
- Distribution: cargo-dist v0.31.0, 5 targets, brew + install.sh + checksums
- Pipeline: Shape→Validate→ADI→Code→Test→Fmt→Lint→Audit→Fix→Re-audit→Manual UX→Closeout
- **Sprint Checklist Framework (NOTE-044)** landed 2026-04-08 as reusable quality gate
- ADI mandatory for Standard+ depth (CLAUDE.md methodology update)

### v0.17.0 done — EPIC-003 Search, Discovery, Intelligence ✅

- [x] **Sprint 13.0** Security + ADR-007 (2h, no artifact)
- [x] **Sprint 13.1** PRD-043 Methodology Integrity — EVID-058, PR #145
- [x] **Sprint 13.1.5/.7** Hardening + integrity config wiring — EVID-059
- [x] **Sprint 13.2** PRD-039 Smart Search v2 (BM25 + filter DSL + graph expansion) — EVID-065 (backfill during final audit)
- [x] **Sprint 13.3** PRD-035 p1 Tags + Source Tier — EVID-060
- [x] **Sprint 13.4** PRD-035 p2 Discover MCP tools + CLI — EVID-061
- [x] **Sprint 13.5** PRD-040 Scoring Intelligence (Skills Memory + R_eff CI) — EVID-062, PR #153
- [x] **Sprint 13.6** PRD-041 FPF Rules CLI + MCP — EVID-063, PR #154
- [x] **Sprint 13.7** PRD-042 FPF KB Vector Search (supersedes PRD-018) — EVID-064, PR #155
- [x] **Sprint 13.7 post-closeout hotfix** — 19 debts triaged, 11 fixed, NOTE-044/045 (PR #156)
- [x] **Final release audit** — 4 parallel auditors, version bump, CHANGELOG, bugfix agent
- [x] **PRD-018 superseded** by PRD-042, **EPIC-003 activated**

### P0: Release v0.17.0 tag
- [x] Cargo.toml version bump 0.16.0 → 0.17.0 (workspace + 3 path deps)
- [x] CHANGELOG.md created with full v0.17.0 entry
- [x] 7 EVIDs activated (058..064) + EVID-065 backfill for Sprint 13.2
- [x] PRD-039 activated (R_eff=1.00, F-G-R=0.88 A)
- [x] EPIC-003 activated
- [x] Title validation bugfix (fa97f10, tag-prep-bugfix agent)
- [x] Commit tag-prep changes (6a1904f)

### P1: After release PR merged
- PR release/v0.17.0 → main (merge commit)
- Tag v0.17.0 + push
- Sync main → dev
- Hindsight memory_retain EPIC-003 finale

### P0: FPF Engine v2 Phase 2 — Sprint 12 (RFC-001) ✅
- [x] ADI reasoning: H2 Two-tier Rules selected (FPF B.5.2 Abductive Loop)
- [x] ext/rules.rs: Rule engine with expressions, graph-aware, time-aware (~600 LOC)
- [x] Dashboard integration: rule engine replaces explore::suggest, HashMap O(N+R)
- [x] Bounded context in reason output (CLI + MCP)
- [x] Config template with rule examples in forgeplan init
- [x] FpfConfig.rules field (empty = default 5 rules)
- [x] 4 audit agents: code review, bounded context, Rust expert, security
- [x] Audit fixes: NaN rejection, empty condition guard, circular scoring, TOCTOU, O(N+R)
- [x] 38 rule engine tests (unit + scenario + negative + corner), 829 total
- [x] EVID-057 linked, R_eff=1.00, PRs #133 + #135 merged
- [x] KB vector search — deferred to Phase 3+ (keyword works, NOTE-039 DSL idea)

### P0: FPF Engine v2 Phase 1 — Sprint 11 (RFC-001) ✅
- [x] EPIC-002 shaped + activated (PR #128)
- [x] RFC-001 shaped: 3 options, ADI confirmed Option C (Layered Core+Ext)
- [x] fpf/core/ module: config.rs, trust.rs, adi.rs, model.rs (34 tests)
- [x] FpfConfig wired into CLI scoring (score, fgr, context, dashboard)
- [x] Config templates in init + current config.yaml (all 6 sections)
- [x] Audit: 3 agents, 3H + 1M fixed, NaN validation, reliability clamp
- [x] EVID-055, R_eff=1.00, RFC-001 activated
- [x] Housekeeping: 12 orphans linked, SPRINTS.md updated
- [x] 1.5: AdiRecord wiring — reason --save creates structured JSON in Note body
- [x] 11.3: CI/CD Linter — health --ci + validate --ci + GH Actions workflow (PR #132)
- [x] PR #131 merged + progress synced

### P0: Distribution Pipeline — Sprint 10 (PRD-023) ✅
- [x] PRD-023 shaped + validated (8 FR, 4 journeys, 4 AC, Deep depth)
- [x] ADI reasoning: 3 hypotheses → H1 cargo-dist selected (High confidence)
- [x] cargo-dist v0.31.0 integrated (dist-workspace.toml, release.yml generated)
- [x] 5 targets: macOS arm64/x86, Linux x86/musl, Windows x86
- [x] Installers: shell (install.sh) + Homebrew (AiDogfood/homebrew-tap)
- [x] Cargo.toml metadata: homepage, keywords, categories for crates.io
- [x] 2-agent audit: 4C + 3H + 3M findings → all fixed
- [x] Action versions @v6/@v7 → @v4 (cargo-dist v0.31.0 bug)
- [x] .gitignore: dist manifest files added
- [x] Embed fix resolved: fastembed v5.13.0 compiles (upstream fixed)
- [x] EVID-050 active, R_eff=0.80, PR #97 merged
- [x] 753 tests, 0 failures, project healthy

### P0: ADI Quality + LLM + E2E + Cleanup — Sprint 9 (PROB-021) ✅
- [x] PROB-021: ADI prompt enriched (metadata, relations+titles, architecture hint)
- [x] System prompt: justified confidence, project context awareness
- [x] reason_temperature config field, architecture hint from file
- [x] evidence_needed → CLI "Next steps" UX
- [x] Model benchmark: 4 models × 7 artifacts → gemini-3-flash-preview selected
- [x] E2E Wave 8 (LLM): 10/10 pass (generate, reason, decompose, capture, context)
- [x] Draft cleanup: 34→12 (7 deleted, 15 deprecated)
- [x] cargo fmt entire codebase (122 files) + pre-commit-fmt.sh hook
- [x] Pipeline updated: +Fmt step, ADI mandatory for Standard+
- [x] 6 new E2E integration tests (cascade delete, lifecycle, deprecated blocking)
- [x] EVID-048 active, R_eff=0.90, PR #96 merged
- [x] 753 tests, 0 failures, project healthy

### P0: Graph Integrity — Sprint 8 (PROB-020) ✅
- [x] BUG-1 (P1): blocked/order treated deprecated as blockers → resolved_ids
- [x] BUG-2 (P1): delete cascade relations + phantom PROB-013 cleanup
- [x] BUG-2b: unlink resilient for phantom relations
- [x] 5-agent audit: 2 critical + 5 warnings → all fixed
- [x] 2 new MCP tools: forgeplan_blocked + forgeplan_order
- [x] validate_id_for_filter() whitelist, DRY common::resolved_ids()
- [x] O(n²)→O(n) order.rs, double scan eliminated, TOCTOU fixed
- [x] route "" rejects empty, memory excluded from orphan detection
- [x] 83 E2E commands tested, E2E-TEST-PLAN.md created
- [x] PROB-020 active, EVID-047 linked, R_eff=1.00, PR #95

### P0: E2E Bug Fixes — Sprint 2 (PROB-018)
- [x] **BUG-001 (P1 Security):** `scan --path /tmp` path traversal — added project root boundary validation (coverage.rs)
- [x] **BUG-002 (P2):** `unlink` не проверяет существование связи — added existence check (link.rs)
- [x] **BUG-003 (P3):** lifecycle transition message "draft → active" hardcoded — uses old_status (activate.rs)
- [x] 2 new unit tests for delete_relation (store.rs)
- [x] Create Evidence EVID-040 + link to PROB-018
- [x] Audit (4-agent: logic+security+rust+task) + PR #85 merged

### P0: Lifecycle v2 — Sprint 3 (ADR-005) ✅
- [x] ADR-005 shaped + validated (new state machine design)
- [x] Phase 1: stale status + terminal deprecated/superseded (transitions.rs, 15 tests)
- [x] Phase 2: renew() + reopen() core logic (lifecycle/mod.rs, 10 tests)
- [x] Phase 3: CLI commands (renew.rs, reopen.rs)
- [x] PROB-019: self-link guard (store.rs + in_memory.rs, 4 tests)
- [x] 5-agent audit: 8 findings fixed (date validation, reason sanitization, atomicity)
- [x] EVID-041 (PROB-019) + EVID-042 (ADR-005) linked + activated
- [x] ADR-005 activated, R_eff = 0.80
- [x] PR #85 merged

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
- [x] **LLM tests**: Wave 8 (10 commands) passed with gemini-3-flash-preview. Wave 10 edge cases still pending.
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
| PROB-020 | Done | Graph integrity — 10 bugs, 5-agent audit, cascade delete, PR #95 | ✅ |
| PROB-021 | Done | ADI quality — enriched prompt, model benchmark, fmt hooks, PR #96 | ✅ |

---

## Backlog (приоритизированный)

### P1: Release v0.13.0 (Distribution)
- [ ] Tag v0.13.0 → trigger first automated release via GH Actions
- [ ] Verify brew install + install.sh on clean machine
- [ ] `cargo publish` (manual, safety hook blocks)

### ~~P1: Embed & Distribution~~ ✅
- [x] **Embed feature fix** — fastembed v5.13.0 compiles, upstream resolved
- [x] **Distribution** — cargo-dist v0.31.0, PR #97 merged (Sprint 10)

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
